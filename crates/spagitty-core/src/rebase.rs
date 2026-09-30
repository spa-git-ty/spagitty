// SPDX-License-Identifier: GPL-3.0-or-later

//! Planning a history rewrite.
//!
//! Interactive rebase is the most feared operation in git because the todo list
//! is edited blind: you choose squash and reword against a list of short ids and
//! find out what you did afterwards. The whole argument for a rebase screen is
//! the preview, and this module is that half — the half that carries the value
//! and none of the risk.
//!
//! **Nothing here runs anything, and that is still true.** Planning and
//! executing are kept apart: [`plan`] answers "what would this do" and
//! [`todo_text`] turns the same order into a `git-rebase-todo` file, but
//! neither touches the repository. Execution is one call up, in
//! [`crate::ops::rebase_interactive`], which hands that file to `git` — so the
//! preview the user approved and the instructions git receives are generated
//! from one ordering, and cannot describe two different rebases.
//!
//! [`progress`] is the one thing here that reads a running rebase, and it still
//! runs nothing: it reads the counters git itself writes into its state
//! directory. That is deliberate. Parsing `Rebasing (3/7)` out of git's stderr
//! would tie the screen to the wording of a progress message that is localised
//! and has changed before; `rebase-merge/msgnum` and `rebase-merge/end` are the
//! same numbers, in a file, in a format other tools already depend on.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::graph::short_id;

/// How far a rebase that is running has got.
///
/// `None` for a repository that is not in the middle of one. A rebase that has
/// stopped — on a conflict, or on an `edit` — is still in progress, and this is
/// what says where it stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Which commit is being applied, counting from 1.
    pub step: usize,
    /// How many there are in total.
    pub total: usize,
    /// The branch being rebased, when git recorded one. A rebase started from
    /// a detached HEAD has none.
    pub branch: Option<String>,
    /// The commit the branch was on before the rebase — `ORIG_HEAD`, and the
    /// whole of the undo path once the rebase has finished.
    pub original: Option<String>,
}

/// The longest range this screen will plan.
///
/// A rebase of a thousand commits is a different operation from the one this
/// screen is for, and a todo list nobody can read is not a preview. The cap is
/// reported rather than silently applied.
pub const MAX_TODO: usize = 250;

/// What to do with one commit, using git's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// Keep it as it is.
    #[default]
    Pick,
    /// Fold it into the row above, which is the direction git folds.
    Squash,
    /// Keep the change, ask for a new message at execution time.
    Reword,
    /// Leave it out of the result entirely.
    Drop,
}

/// One row of the todo list, as `git rebase -i` would open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoRow {
    pub id: String,
    pub short: String,
    pub summary: String,
    pub author_name: String,
    /// Author time, seconds since the unix epoch.
    pub time: i64,
    /// Paths this commit changed. What the conflict heuristic compares.
    pub paths: Vec<String>,
}

/// The list `git rebase -i <upstream>` would open, before any edit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    /// The commit the rebase would replay onto.
    pub upstream: String,
    pub upstream_short: String,
    /// Oldest first, the order git lists them in — a rebase replays forwards.
    pub rows: Vec<TodoRow>,
    /// True when the range was longer than [`MAX_TODO`] and was cut.
    pub truncated: bool,
}

/// One edit: a row, where it now sits, and what to do with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    /// The commit this edit is about, by full id.
    pub id: String,
    pub action: Action,
    /// The new message, for a [`Action::Reword`].
    ///
    /// Collected on screen rather than at execution time, because "at execution
    /// time" means an editor on a terminal that does not exist. A reword with
    /// no message is a pick — see [`todo_text`].
    #[serde(default)]
    pub message: Option<String>,
}

/// One row of the result: what a commit would become.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    /// The commit this row is built on, by full id. A squash makes it the
    /// commit that absorbed the others.
    pub id: String,
    pub short: String,
    pub summary: String,
    /// Commits folded into this one, oldest first. Empty for a plain pick.
    pub absorbed: Vec<String>,
    /// True when the message would be asked for at execution time.
    pub reworded: bool,
    /// Two commits in the plan touch a path this one also touches, so replaying
    /// it may not apply cleanly. A heuristic — see [`plan`].
    pub may_conflict: bool,
}

/// What the plan would produce.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// Oldest first, the same direction as the todo.
    pub rows: Vec<PreviewRow>,
    /// Commits the plan drops entirely.
    pub dropped: Vec<String>,
    /// Set when the plan cannot be executed as written, with the reason. A plan
    /// that drops everything is *not* one of these: an empty result is a
    /// legitimate thing to look at before deciding against it.
    pub refusal: Option<String>,
    /// True when the plan would leave no commits at all.
    pub empties_the_branch: bool,
}

/// The todo list for rebasing `HEAD` onto `upstream`.
///
/// Generated rather than read: running `git rebase -i` to see the file it opens
/// would start a rebase, which is the thing this screen exists to avoid. The
/// range is `upstream..HEAD`, oldest first, merges excluded — the commits
/// `git rebase` itself lists. Where two lines of history run side by side the
/// order can differ from git's own todo; it is always one git can replay, every
/// commit after its parent, and it is the list git is handed.
pub fn todo(repo: &gix::Repository, upstream: &str) -> Result<Todo> {
    let head = repo.head_id().map_err(|_| Error::EmptyRepository)?.detach();

    let onto = repo
        .rev_parse_single(upstream)
        .map_err(|_| Error::UnknownCommit(upstream.to_string()))?
        .object()
        .map_err(|_| Error::UnknownCommit(upstream.to_string()))?
        .peel_to_commit()
        .map_err(|_| Error::UnknownCommit(upstream.to_string()))?
        .id;

    if merge_base(repo, head, onto)?.is_none() {
        return Err(Error::NotStageable(format!(
            "{upstream} shares no history with this branch, so there is nothing to rebase onto"
        )));
    }

    let walk = repo
        .rev_walk([head])
        .with_hidden([onto])
        .all()
        .map_err(|e| Error::Walk(e.to_string()))?;

    let mut rows = Vec::new();
    let mut truncated = false;

    for info in walk {
        let info = info.map_err(|e| Error::Walk(e.to_string()))?;
        let commit = repo
            .find_commit(info.id)
            .map_err(|e| Error::Walk(e.to_string()))?;

        // `git rebase` without `--rebase-merges` does not replay merges, and
        // listing one would promise something the execution cannot keep.
        if commit.parent_ids().count() > 1 {
            continue;
        }

        if rows.len() == MAX_TODO {
            truncated = true;
            break;
        }

        let (author_name, time) = match commit.author() {
            Ok(sig) => (
                sig.name.to_string(),
                sig.time().map(|t| t.seconds).unwrap_or(0),
            ),
            Err(_) => (String::new(), 0),
        };

        rows.push(TodoRow {
            id: info.id.to_string(),
            short: short_id(&info.id),
            summary: commit
                .message()
                .map(|m| m.summary().to_string())
                .unwrap_or_default(),
            author_name,
            time,
            paths: changed_paths(repo, &commit)?,
        });
    }

    // The walk runs newest first; a rebase replays forwards, and the todo list
    // git opens is in that order.
    rows.reverse();

    Ok(Todo {
        upstream: onto.to_string(),
        upstream_short: short_id(&onto),
        rows,
        truncated,
    })
}

/// What `edits` would produce, in the order they are given.
///
/// **The plan is the complete list.** Its order *is* the reordering — moving a
/// row is moving an entry — and every row of the todo appears in it exactly
/// once. A row the plan does not mention is drift between the screen and the
/// repository rather than an instruction, so it is appended at the end and
/// picked; that keeps a commit visible instead of silently dropping it, which
/// is the failure that would matter.
///
/// **`may_conflict` is a heuristic.** It marks a row when an earlier row in the
/// plan touched one of the same paths, which is where replaying is most likely
/// to need a merge. Knowing for certain means performing those merges, which is
/// execution — so the flag says "may", and the screen uses that word.
pub fn plan(todo: &Todo, edits: &[Edit]) -> Preview {
    let ordered = order(todo, edits);

    if let Some(first) = ordered.first() {
        if first.1 == Action::Squash {
            return Preview {
                rows: Vec::new(),
                dropped: Vec::new(),
                refusal: Some(
                    "the first commit cannot be a squash: there is nothing above it to fold into"
                        .to_string(),
                ),
                empties_the_branch: false,
            };
        }
    }

    let mut rows: Vec<PreviewRow> = Vec::new();
    let mut dropped = Vec::new();
    // Paths every kept row has touched so far, for the heuristic.
    let mut seen_paths: Vec<String> = Vec::new();

    for (row, action) in ordered {
        match action {
            Action::Drop => dropped.push(row.id.clone()),
            Action::Squash => {
                // Folding upward is what git does, and `rows` cannot be empty
                // here because a leading squash was refused above.
                if let Some(above) = rows.last_mut() {
                    above.absorbed.push(row.id.clone());
                    if touches_seen(&row.paths, &seen_paths) {
                        above.may_conflict = true;
                    }
                    seen_paths.extend(row.paths.iter().cloned());
                }
            }
            Action::Pick | Action::Reword => {
                let may_conflict = touches_seen(&row.paths, &seen_paths);
                seen_paths.extend(row.paths.iter().cloned());
                rows.push(PreviewRow {
                    id: row.id.clone(),
                    short: row.short.clone(),
                    summary: row.summary.clone(),
                    absorbed: Vec::new(),
                    reworded: action == Action::Reword,
                    may_conflict,
                });
            }
        }
    }

    let empties_the_branch = rows.is_empty();

    Preview {
        rows,
        dropped,
        refusal: None,
        empties_the_branch,
    }
}

/// The plan as a `git-rebase-todo` file.
///
/// This is what [`crate::ops::rebase_interactive`] hands to git through
/// `GIT_SEQUENCE_EDITOR`, and it is generated from exactly the same `order` the
/// preview is generated from — so what the screen showed and what git executes
/// cannot disagree. It refuses the same plan [`plan`] refuses, for the same
/// reason, rather than letting git discover it half-way.
///
/// A reword becomes `pick` followed by `exec git commit --amend`. `reword`
/// itself would open an editor on the combined message, and there is no
/// terminal for it — putting the message on the command line is how the choice
/// made on screen survives to the repository. A reword with no message is
/// nothing to do, so it is written as a plain pick.
pub fn todo_text(todo: &Todo, edits: &[Edit]) -> Result<String> {
    let ordered = order(todo, edits);

    if let Some((_, Action::Squash)) = ordered.first().map(|(row, action)| (row, *action)) {
        return Err(Error::NotStageable(
            "the first commit cannot be a squash: there is nothing above it to fold into".into(),
        ));
    }

    let by_id: std::collections::HashMap<&str, &Edit> =
        edits.iter().map(|edit| (edit.id.as_str(), edit)).collect();

    let mut out = String::new();
    for (row, action) in ordered {
        match action {
            Action::Pick => out.push_str(&format!("pick {}\n", row.id)),
            Action::Squash => out.push_str(&format!("squash {}\n", row.id)),
            Action::Drop => out.push_str(&format!("drop {}\n", row.id)),
            Action::Reword => {
                out.push_str(&format!("pick {}\n", row.id));
                let message = by_id
                    .get(row.id.as_str())
                    .and_then(|edit| edit.message.as_deref())
                    .map(str::trim)
                    .filter(|message| !message.is_empty());
                if let Some(message) = message {
                    out.push_str(&format!(
                        "exec git commit --amend --only -m {}\n",
                        shell_quote(message)
                    ));
                }
            }
        }
    }

    Ok(out)
}

/// A string as one POSIX shell word.
///
/// `exec` lines are run through `sh -c`, so a commit message containing a
/// quote, a backtick or a `$(` is otherwise a command injection into the user's
/// own shell. Single quotes take everything literally; the only character that
/// needs handling is the single quote itself, which is closed, escaped and
/// reopened.
///
/// Windows is not a special case here: git runs `exec` lines through its bundled
/// `sh` on every platform.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The todo's rows in the plan's order, each with the action it carries.
///
/// An edit naming a commit that is not in the todo is ignored, and a commit the
/// plan never names is appended and picked. Both are drift between the screen
/// and the repository, and losing one row of a preview is better than losing
/// the preview.
fn order<'a>(todo: &'a Todo, edits: &[Edit]) -> Vec<(&'a TodoRow, Action)> {
    let mut out: Vec<(&TodoRow, Action)> = Vec::new();
    let mut placed: Vec<&str> = Vec::new();

    for edit in edits {
        if let Some(row) = todo.rows.iter().find(|row| row.id == edit.id) {
            if placed.contains(&row.id.as_str()) {
                continue;
            }
            placed.push(&row.id);
            out.push((row, edit.action));
        }
    }

    // Anything the plan did not mention keeps its place and is picked.
    for row in &todo.rows {
        if !placed.contains(&row.id.as_str()) {
            out.push((row, Action::Pick));
        }
    }

    out
}

fn touches_seen(paths: &[String], seen: &[String]) -> bool {
    paths.iter().any(|path| seen.contains(path))
}

/// Paths this commit changed against its first parent. A root commit changed
/// everything it has.
fn changed_paths(repo: &gix::Repository, commit: &gix::Commit<'_>) -> Result<Vec<String>> {
    let tree = commit.tree().map_err(walk_err)?;

    let parent = commit.parent_ids().next();
    let Some(parent) = parent else {
        return tree_paths(&tree);
    };

    let before = repo
        .find_commit(parent.detach())
        .map_err(walk_err)?
        .tree()
        .map_err(walk_err)?;

    let mut changed = Vec::new();
    before
        .changes()
        .map_err(walk_err)?
        .for_each_to_obtain_tree(&tree, |change| {
            changed.push(change.location().to_string());
            Ok::<_, std::convert::Infallible>(gix::object::tree::diff::Action::Continue(()))
        })
        .map_err(walk_err)?;

    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Every blob path in a tree. What a root commit "changed".
fn tree_paths(tree: &gix::Tree<'_>) -> Result<Vec<String>> {
    let mut recorder = gix::traverse::tree::Recorder::default();
    tree.traverse()
        .breadthfirst(&mut recorder)
        .map_err(walk_err)?;

    let mut out: Vec<String> = recorder
        .records
        .into_iter()
        .filter(|entry| entry.mode.is_blob_or_symlink())
        .map(|entry| entry.filepath.to_string())
        .collect();
    out.sort();
    Ok(out)
}

fn merge_base(
    repo: &gix::Repository,
    a: gix::ObjectId,
    b: gix::ObjectId,
) -> Result<Option<gix::ObjectId>> {
    match repo.merge_base(a, b) {
        Ok(id) => Ok(Some(id.detach())),
        Err(_) => Ok(None),
    }
}

fn walk_err(e: impl std::fmt::Display) -> Error {
    Error::Walk(e.to_string())
}

/// Read how far a running rebase has got, from git's own state directory.
///
/// Both interactive and non-interactive rebases keep their state under
/// `rebase-merge/`; the plain `rebase-apply/` form is what `git am` and an
/// old-style `--am` rebase use, and it counts the same way in `next` and
/// `last`. Both are read, so a rebase started from the command line while
/// Spagitty was open is still legible here.
///
/// A missing or unparseable counter is `None` rather than a zero: "not
/// rebasing" and "rebasing, position unknown" are different answers, and only
/// one of them should make a progress bar appear.
pub fn progress(repo: &gix::Repository) -> Option<Progress> {
    progress_in(repo.git_dir())
}

/// [`progress`] against a git directory rather than an open repository.
///
/// The rebase worker has a path and no repository — it took one while it held
/// the session lock and gave the lock back — and opening one just to read two
/// counters would be work done to satisfy a signature.
pub fn progress_in(git_dir: &std::path::Path) -> Option<Progress> {
    let (dir, current, total) = [
        ("rebase-merge", "msgnum", "end"),
        ("rebase-apply", "next", "last"),
    ]
    .into_iter()
    .find_map(|(dir, current, total)| {
        let dir = git_dir.join(dir);
        Some((
            dir.clone(),
            number(&dir.join(current))?,
            number(&dir.join(total))?,
        ))
    })?;

    Some(Progress {
        step: current,
        total,
        branch: text(&dir.join("head-name")).map(|name| {
            name.strip_prefix("refs/heads/")
                .unwrap_or(&name)
                .to_string()
        }),
        // Shortened here rather than by the caller: it is only ever shown, and
        // the caller that needs the full id has `ORIG_HEAD` itself.
        original: text(&git_dir.join("ORIG_HEAD")).map(|id| id.chars().take(7).collect()),
    })
}

fn text(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn number(path: &std::path::Path) -> Option<usize> {
    text(path)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// Write the counters git writes, without running a rebase to get them.
    ///
    /// A real interrupted rebase in a test is a race: it depends on git
    /// stopping where the fixture expects. The counters are a documented file
    /// format, and what is being asserted here is that they are read.
    fn write_state(
        fixture: &Fixture,
        dir: &str,
        current: &str,
        total: &str,
        step: usize,
        end: usize,
    ) {
        let git_dir = fixture.path().join(".git").join(dir);
        std::fs::create_dir_all(&git_dir).expect("state directory");
        std::fs::write(git_dir.join(current), format!("{step}\n")).expect("current");
        std::fs::write(git_dir.join(total), format!("{end}\n")).expect("total");
    }

    #[test]
    fn a_repository_that_is_not_rebasing_has_no_progress() {
        // Not a zero: "not rebasing" and "rebasing, position unknown" are
        // different answers, and only one should make a bar appear.
        let fixture = Fixture::woven();
        assert!(progress(&fixture.open()).is_none());
    }

    #[test]
    fn an_interactive_rebase_reports_its_step_and_total() {
        let fixture = Fixture::woven();
        write_state(&fixture, "rebase-merge", "msgnum", "end", 3, 7);

        let progress = progress(&fixture.open()).expect("progress");

        assert_eq!(progress.step, 3);
        assert_eq!(progress.total, 7);
    }

    #[test]
    fn an_old_style_rebase_counts_the_same_way() {
        // `rebase-apply/` is what `git am` and a `--am` rebase use. A rebase
        // started from the command line is still legible here.
        let fixture = Fixture::woven();
        write_state(&fixture, "rebase-apply", "next", "last", 2, 5);

        let progress = progress(&fixture.open()).expect("progress");

        assert_eq!(progress.step, 2);
        assert_eq!(progress.total, 5);
    }

    #[test]
    fn the_branch_is_reported_without_its_refs_heads_prefix() {
        let fixture = Fixture::woven();
        write_state(&fixture, "rebase-merge", "msgnum", "end", 1, 2);
        std::fs::write(
            fixture.path().join(".git/rebase-merge/head-name"),
            "refs/heads/feature/split-view\n",
        )
        .expect("head-name");

        let progress = progress(&fixture.open()).expect("progress");

        assert_eq!(progress.branch.as_deref(), Some("feature/split-view"));
    }

    #[test]
    fn a_rebase_from_a_detached_head_has_no_branch() {
        let fixture = Fixture::woven();
        write_state(&fixture, "rebase-merge", "msgnum", "end", 1, 2);

        assert!(progress(&fixture.open())
            .expect("progress")
            .branch
            .is_none());
    }

    #[test]
    fn the_original_position_is_shortened_for_showing() {
        let fixture = Fixture::woven();
        write_state(&fixture, "rebase-merge", "msgnum", "end", 1, 2);
        let head = fixture.head();
        std::fs::write(fixture.path().join(".git/ORIG_HEAD"), format!("{head}\n"))
            .expect("ORIG_HEAD");

        let progress = progress(&fixture.open()).expect("progress");

        assert_eq!(progress.original.as_deref(), Some(&head[..7]));
    }

    #[test]
    fn a_state_directory_with_no_counters_is_not_progress() {
        // git creates the directory before it writes the numbers. Reading a
        // half-written state as "step 0 of 0" would flash a bar at zero.
        let fixture = Fixture::woven();
        std::fs::create_dir_all(fixture.path().join(".git/rebase-merge")).expect("directory");

        assert!(progress(&fixture.open()).is_none());
    }

    /// A branch of three commits on top of main, each touching its own file.
    fn linear() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("base.txt", "base\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "work"]);
        for name in ["one", "two", "three"] {
            fixture.write(&format!("{name}.txt"), &format!("{name}\n"));
            fixture.git(&["add", "-A"]);
            fixture.commit(&format!("Add {name}"));
        }
        fixture
    }

    /// A complete plan: every row in its original order, picked, with the
    /// given positions overridden. The screen always sends the whole list.
    fn edits(todo: &Todo, actions: &[(usize, Action)]) -> Vec<Edit> {
        let mut plan: Vec<Edit> = todo
            .rows
            .iter()
            .map(|row| Edit {
                id: row.id.clone(),
                action: Action::Pick,
                message: None,
            })
            .collect();

        for (at, action) in actions {
            plan[*at].action = *action;
        }
        plan
    }

    /// A complete plan in a different order: `order` is positions in the todo.
    fn reordered(todo: &Todo, order: &[usize]) -> Vec<Edit> {
        order
            .iter()
            .map(|at| Edit {
                id: todo.rows[*at].id.clone(),
                action: Action::Pick,
                message: None,
            })
            .collect()
    }

    #[test]
    fn the_todo_list_is_what_git_rebase_would_open() {
        let fixture = linear();
        let repo = fixture.open();

        let todo = todo(&repo, "main").expect("todo");

        let expected: Vec<String> = fixture
            .git(&["rev-list", "--reverse", "--no-merges", "main..HEAD"])
            .lines()
            .map(str::to_string)
            .collect();

        assert_eq!(
            todo.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(todo.upstream, fixture.rev("main"));
    }

    #[test]
    fn the_todo_list_is_oldest_first_because_a_rebase_replays_forwards() {
        let fixture = linear();
        let repo = fixture.open();

        let todo = todo(&repo, "main").expect("todo");

        assert_eq!(
            todo.rows
                .iter()
                .map(|r| r.summary.clone())
                .collect::<Vec<_>>(),
            vec!["Add one", "Add two", "Add three"]
        );
    }

    #[test]
    fn merges_are_left_out_because_a_rebase_does_not_replay_them() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let todo = todo(&repo, "v0.1.0").expect("todo");

        for row in &todo.rows {
            assert!(!row.summary.starts_with("Merge"), "{}", row.summary);
        }

        // The commits git would replay. Not in git's order: that compared
        // equal only while the fixture's commits shared a second (BUG-039).
        let listed: Vec<String> = todo.rows.iter().map(|r| r.id.clone()).collect();
        let mut ours = listed.clone();
        ours.sort();
        let mut git_lists: Vec<String> = fixture
            .git(&["rev-list", "--no-merges", "v0.1.0..HEAD"])
            .lines()
            .map(str::to_string)
            .collect();
        git_lists.sort();
        assert_eq!(ours, git_lists);

        // In an order git can replay: every commit after its parent.
        for (at, id) in listed.iter().enumerate() {
            let parent = fixture.rev(&format!("{id}^"));
            if let Some(before) = listed.iter().position(|other| *other == parent) {
                assert!(before < at, "{id} is listed before its parent {parent}");
            }
        }
    }

    #[test]
    fn a_row_carries_the_paths_its_commit_changed() {
        let fixture = linear();
        let repo = fixture.open();

        let todo = todo(&repo, "main").expect("todo");

        assert_eq!(todo.rows[0].paths, vec!["one.txt"]);
        assert_eq!(todo.rows[2].paths, vec!["three.txt"]);
    }

    #[test]
    fn a_branch_with_no_merge_base_is_refused_with_a_reason() {
        let fixture = linear();
        // An orphan branch shares no history with anything.
        fixture.git(&["switch", "-q", "--orphan", "stranger"]);
        fixture.write("alone.txt", "alone\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Alone");
        let repo = fixture.open();

        let result = todo(&repo, "main");

        let message = result.expect_err("a refusal").to_string();
        assert!(message.contains("shares no history"), "{message}");
    }

    #[test]
    fn an_unknown_upstream_is_an_error() {
        let fixture = linear();
        let repo = fixture.open();

        assert!(todo(&repo, "no-such-branch").is_err());
    }

    #[test]
    fn a_plan_that_changes_nothing_previews_the_branch_as_it_is() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &[]);

        assert_eq!(
            preview
                .rows
                .iter()
                .map(|r| r.id.clone())
                .collect::<Vec<_>>(),
            todo.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>()
        );
        assert!(preview.dropped.is_empty());
        assert!(preview.refusal.is_none());
    }

    #[test]
    fn a_squash_removes_a_row_and_folds_it_into_the_one_above() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &edits(&todo, &[(1, Action::Squash)]));

        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.rows[0].summary, "Add one");
        assert_eq!(preview.rows[0].absorbed, vec![todo.rows[1].id.clone()]);
        assert_eq!(preview.rows[1].summary, "Add three");
    }

    #[test]
    fn a_leading_squash_is_refused_because_there_is_nothing_above_it() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &edits(&todo, &[(0, Action::Squash)]));

        assert!(preview.rows.is_empty());
        assert!(preview
            .refusal
            .expect("a refusal")
            .contains("nothing above it"));
    }

    #[test]
    fn a_drop_takes_the_commit_out_of_the_result() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &edits(&todo, &[(1, Action::Drop)]));

        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.dropped, vec![todo.rows[1].id.clone()]);
        assert!(!preview.rows.iter().any(|r| r.id == todo.rows[1].id));
    }

    #[test]
    fn a_reword_keeps_the_row_and_marks_it() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &edits(&todo, &[(2, Action::Reword)]));

        assert_eq!(preview.rows.len(), 3);
        assert!(preview.rows[2].reworded);
        assert!(!preview.rows[0].reworded);
    }

    #[test]
    fn dropping_every_commit_is_an_empty_preview_and_a_warning_not_an_error() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(
            &todo,
            &edits(
                &todo,
                &[(0, Action::Drop), (1, Action::Drop), (2, Action::Drop)],
            ),
        );

        assert!(preview.rows.is_empty());
        assert!(preview.empties_the_branch);
        assert!(preview.refusal.is_none(), "a warning, not a failure");
        assert_eq!(preview.dropped.len(), 3);
    }

    #[test]
    fn reordering_is_the_order_of_the_edits() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &reordered(&todo, &[2, 0, 1]));

        assert_eq!(
            preview
                .rows
                .iter()
                .map(|r| r.summary.clone())
                .collect::<Vec<_>>(),
            vec!["Add three", "Add one", "Add two"]
        );
    }

    #[test]
    fn a_commit_the_plan_never_mentions_is_kept_rather_than_lost() {
        // Drift between the screen and the repository. Appending it keeps the
        // commit visible; dropping it silently is the failure that would matter.
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &reordered(&todo, &[2, 0]));

        assert_eq!(
            preview
                .rows
                .iter()
                .map(|r| r.summary.clone())
                .collect::<Vec<_>>(),
            vec!["Add three", "Add one", "Add two"]
        );
    }

    #[test]
    fn an_edit_naming_a_commit_that_is_not_there_is_ignored() {
        // The screen and the repository have drifted; losing one row of a
        // preview beats losing the preview.
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let mut stale = edits(&todo, &[]);
        stale.insert(
            0,
            Edit {
                id: "0".repeat(40),
                action: Action::Drop,
                message: None,
            },
        );

        let preview = plan(&todo, &stale);

        assert_eq!(preview.rows.len(), 3);
        assert!(preview.dropped.is_empty());
    }

    #[test]
    fn a_commit_named_twice_is_only_placed_once() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let mut repeated = edits(&todo, &[]);
        repeated.push(Edit {
            id: todo.rows[0].id.clone(),
            action: Action::Drop,
            message: None,
        });

        let preview = plan(&todo, &repeated);

        assert_eq!(preview.rows.len(), 3, "one row per commit, still");
        assert!(preview.dropped.is_empty(), "the first mention won");
    }

    #[test]
    fn two_commits_touching_one_path_mark_the_later_one_as_maybe_conflicting() {
        let fixture = Fixture::empty();
        fixture.write("shared.txt", "one\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "work"]);
        fixture.write("shared.txt", "two\n");
        fixture.commit_all("Change it once");
        fixture.write("shared.txt", "three\n");
        fixture.commit_all("Change it again");
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &[]);

        assert!(!preview.rows[0].may_conflict, "nothing came before it");
        assert!(
            preview.rows[1].may_conflict,
            "it touches a path an earlier row in the plan touched"
        );
    }

    #[test]
    fn commits_touching_different_paths_are_not_flagged() {
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");

        let preview = plan(&todo, &[]);

        assert!(preview.rows.iter().all(|row| !row.may_conflict));
    }

    #[test]
    fn planning_never_touches_the_repository() {
        // Criterion 5: after any amount of editing, no rebase is in progress
        // and ORIG_HEAD is where it was.
        let fixture = linear();
        let repo = fixture.open();
        let todo = todo(&repo, "main").expect("todo");
        let head_before = fixture.head();

        for actions in [
            vec![(0, Action::Reword)],
            vec![(1, Action::Squash)],
            vec![(2, Action::Drop), (0, Action::Pick)],
            vec![(0, Action::Drop), (1, Action::Drop), (2, Action::Drop)],
        ] {
            plan(&todo, &edits(&todo, &actions));
        }

        assert_eq!(fixture.head(), head_before);
        assert_eq!(
            fixture.git(&["status", "--porcelain"]),
            "",
            "nothing was written"
        );
        assert!(
            !fixture.path().join(".git/rebase-merge").exists(),
            "no rebase was started"
        );
        assert!(
            !fixture.path().join(".git/rebase-apply").exists(),
            "no rebase was started"
        );
    }

    #[test]
    fn a_range_longer_than_the_cap_says_it_was_cut() {
        let fixture = Fixture::empty();
        fixture.write("base.txt", "base\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "work"]);
        for n in 0..MAX_TODO + 5 {
            fixture.write("base.txt", &format!("{n}\n"));
            fixture.commit_all(&format!("Change {n}"));
        }
        let repo = fixture.open();

        let todo = todo(&repo, "main").expect("todo");

        assert_eq!(todo.rows.len(), MAX_TODO);
        assert!(todo.truncated, "the cap is reported, not applied silently");
    }
}

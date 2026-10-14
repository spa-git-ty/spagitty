// SPDX-License-Identifier: GPL-3.0-or-later

//! The restore point, and undoing a whole run (FEAT-133).
//!
//! A recipe acts on the real working copy. Before its first step everything
//! it could change is written down under `refs/spagitty/restore/<id>/`, where
//! no branch list, graph or tag list reads, and where git keeps the objects
//! alive:
//!
//! - `meta`: a note of what `HEAD` was, as a branch's name or a commit, with
//!   the tip of every branch the recipe names and the stash list;
//! - `head`: the commit `HEAD` was at;
//! - `index`: the index, as a tree;
//! - `work`: the working tree, as a tree — **with its untracked files**,
//!   which `git stash create` leaves out. It is made through an index of its
//!   own, so the real one is not touched;
//! - `branch/<name>`: each named branch's tip;
//! - `stash/<n>`: each stash.
//!
//! *Undo the whole run* puts those back. It never deletes a file git did not
//! know about, and it cannot take back a push: what reached the remote stays
//! there.
//!
//! The newest [`KEEP`] are kept, and none older than [`KEEP_DAYS`] days.

use std::ffi::OsStr;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::shell;

/// Where restore points are kept.
pub const PREFIX: &str = "refs/spagitty/restore/";
/// How many are kept.
pub const KEEP: usize = 20;
/// None is kept longer than this.
pub const KEEP_DAYS: u64 = 7;

/// A branch and where it pointed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tip {
    pub name: String,
    pub id: String,
}

/// A stash as it was: the commit it is, and what it was called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stashed {
    pub id: String,
    pub message: String,
}

/// What a restore point puts back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePoint {
    pub id: String,
    /// Unix seconds.
    pub created: u64,
    /// The recipe it was made for.
    pub recipe: String,
    /// The branch checked out. `None` when `HEAD` was detached.
    pub branch: Option<String>,
    /// The commit `HEAD` was at.
    pub head: String,
    /// The branches the recipe names, and where each pointed.
    pub tips: Vec<Tip>,
    /// The branches the recipe makes, which were not there.
    pub makes: Vec<String>,
    /// The stash list, newest first.
    pub stashes: Vec<Stashed>,
}

impl RestorePoint {
    /// The ref everything of this restore point is under.
    pub fn reference(&self) -> String {
        format!("{PREFIX}{}", self.id)
    }
}

/// An id for a restore point made at `now`: the time, so they sort, and
/// something of the process so two made in one second differ.
pub fn new_id(now: u64) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let count = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{now}-{:04x}", (std::process::id() ^ (count << 8)) & 0xffff)
}

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    Ok(shell::ran_ok(repo, args)?.trim().to_string())
}

fn refused(what: impl Into<String>) -> Error {
    Error::Git {
        command: "restore point".into(),
        stderr: what.into(),
    }
}

/// The stash list, newest first.
fn stashes(repo: &Path) -> Result<Vec<Stashed>> {
    let listed = shell::ran(repo, &["stash", "list", "--format=%H%x1f%gs"])?;
    Ok(listed
        .stdout
        .lines()
        .filter_map(|line| {
            let (id, message) = line.split_once('\u{1f}')?;
            Some(Stashed {
                id: id.to_string(),
                message: message.to_string(),
            })
        })
        .collect())
}

/// The working tree as a tree, untracked files and all, without touching the
/// index: everything is added to a copy of it.
fn work_tree(repo: &Path, id: &str) -> Result<String> {
    let git_dir = shell::absolute_git_dir(repo)?;
    let copy = git_dir.join(format!("spagitty-restore-{id}.index"));
    let real = git_dir.join("index");
    if real.is_file() {
        std::fs::copy(&real, &copy)?;
    }
    let env: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", copy.as_os_str())];
    let made = (|| {
        let added = shell::ran_with(repo, &["add", "-A"], &env, None)?;
        if !added.ok() {
            return Err(refused(added.stderr.trim()));
        }
        let tree = shell::ran_with(repo, &["write-tree"], &env, None)?;
        if !tree.ok() {
            return Err(refused(tree.stderr.trim()));
        }
        Ok(tree.stdout.trim().to_string())
    })();
    let _ = std::fs::remove_file(&copy);
    made
}

/// Write a restore point for a recipe that names `branches` and makes
/// `makes`.
///
/// Refused in a repository with no commit, and with conflicts in the index:
/// neither has a state that can be put back.
pub fn create(
    repo: &Path,
    id: &str,
    recipe: &str,
    branches: &[String],
    makes: &[String],
    now: u64,
) -> Result<RestorePoint> {
    let head = git(repo, &["rev-parse", "--verify", "HEAD^{commit}"])
        .map_err(|_| refused("There is no commit yet, so there is nothing to go back to."))?;
    let branch = shell::ran(repo, &["symbolic-ref", "-q", "--short", "HEAD"])?;
    let branch = branch.ok().then(|| branch.stdout.trim().to_string());
    let index = git(repo, &["write-tree"])
        .map_err(|_| refused("The index has conflicts in it. Resolve them first."))?;
    let work = work_tree(repo, id)?;

    let mut tips = Vec::new();
    let mut names: Vec<&String> = branches.iter().collect();
    if let Some(current) = &branch {
        names.push(current);
    }
    for name in names {
        if tips.iter().any(|tip: &Tip| &tip.name == name) {
            continue;
        }
        let tip = shell::ran(
            repo,
            &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")],
        )?;
        if tip.ok() {
            tips.push(Tip {
                name: name.clone(),
                id: tip.stdout.trim().to_string(),
            });
        }
    }
    let makes: Vec<String> = makes
        .iter()
        .filter(|name| !tips.iter().any(|tip| &tip.name == *name))
        .cloned()
        .collect();

    let point = RestorePoint {
        id: id.to_string(),
        created: now,
        recipe: recipe.to_string(),
        branch,
        head: head.clone(),
        tips,
        makes,
        stashes: stashes(repo)?,
    };
    let meta = serde_json::to_string(&point).map_err(|e| refused(e.to_string()))?;
    let blob = shell::ran_with(repo, &["hash-object", "-w", "--stdin"], &[], Some(&meta))?;
    if !blob.ok() {
        return Err(refused(blob.stderr.trim()));
    }

    let under = point.reference();
    let set = |name: String, object: &str| git(repo, &["update-ref", &name, object]);
    set(format!("{under}/head"), &head)?;
    set(format!("{under}/index"), &index)?;
    set(format!("{under}/work"), &work)?;
    for tip in &point.tips {
        set(format!("{under}/branch/{}", tip.name), &tip.id)?;
    }
    for (at, stash) in point.stashes.iter().enumerate() {
        set(format!("{under}/stash/{at}"), &stash.id)?;
    }
    // Last: a restore point is one that has its note.
    set(format!("{under}/meta"), blob.stdout.trim())?;
    Ok(point)
}

/// The restore point called `id`.
pub fn read(repo: &Path, id: &str) -> Result<RestorePoint> {
    let meta = git(repo, &["cat-file", "blob", &format!("{PREFIX}{id}/meta")])
        .map_err(|_| refused(format!("There is no restore point {id}.")))?;
    serde_json::from_str(&meta).map_err(|e| refused(format!("Restore point {id}: {e}")))
}

/// Every restore point, newest first.
pub fn list(repo: &Path) -> Result<Vec<RestorePoint>> {
    let refs = shell::ran(repo, &["for-each-ref", "--format=%(refname)", PREFIX])?;
    let mut points: Vec<RestorePoint> = refs
        .stdout
        .lines()
        .filter_map(|name| name.strip_prefix(PREFIX)?.strip_suffix("/meta"))
        .filter(|id| !id.contains('/'))
        .filter_map(|id| read(repo, id).ok())
        .collect();
    points.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| b.id.cmp(&a.id)));
    Ok(points)
}

/// Forget a restore point. What it kept alive is git's to collect.
pub fn drop_point(repo: &Path, id: &str) -> Result<()> {
    let under = format!("{PREFIX}{id}/");
    let refs = shell::ran(repo, &["for-each-ref", "--format=%(refname)", &under])?;
    for name in refs.stdout.lines().filter(|name| name.starts_with(&under)) {
        git(repo, &["update-ref", "-d", name])?;
    }
    Ok(())
}

/// Which of `points` (newest first) are past keeping at `now`.
pub fn expired(points: &[RestorePoint], now: u64) -> Vec<String> {
    let oldest = now.saturating_sub(KEEP_DAYS * 24 * 60 * 60);
    points
        .iter()
        .enumerate()
        .filter(|(at, point)| *at >= KEEP || point.created < oldest)
        .map(|(_, point)| point.id.clone())
        .collect()
}

/// Drop the restore points past keeping. Returns how many went.
pub fn prune(repo: &Path, now: u64) -> Result<usize> {
    let gone = expired(&list(repo)?, now);
    for id in &gone {
        drop_point(repo, id)?;
    }
    Ok(gone.len())
}

/// End whatever was left in progress: a merge, a rebase or a cherry-pick
/// that stopped. Nothing is in progress when a restore point is made.
fn end_operation(repo: &Path) -> Result<()> {
    let git_dir = shell::absolute_git_dir(repo)?;
    if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        let _ = shell::ran(repo, &["rebase", "--abort"])?;
    }
    if git_dir.join("CHERRY_PICK_HEAD").exists() {
        let _ = shell::ran(repo, &["cherry-pick", "--abort"])?;
    }
    if git_dir.join("MERGE_HEAD").exists() {
        let _ = shell::ran(repo, &["merge", "--abort"])?;
    }
    Ok(())
}

/// Put back everything `id` wrote down: *Undo the whole run*.
///
/// In this order, because each needs the one before: an operation left in
/// progress is ended; the working tree is set to what it was; the branches
/// and `HEAD` are moved back; the index is set to what it was; and the stash
/// list is made what it was. The restore point is left in place.
pub fn undo(repo: &Path, id: &str) -> Result<RestorePoint> {
    let point = read(repo, id)?;
    let under = point.reference();
    end_operation(repo)?;

    // The tree first, while `HEAD` is still where the run left it: every file
    // git knows about becomes what it was, and one the run added goes.
    git(
        repo,
        &["read-tree", "--reset", "-u", &format!("{under}/work")],
    )?;

    for tip in &point.tips {
        git(
            repo,
            &["update-ref", &format!("refs/heads/{}", tip.name), &tip.id],
        )?;
    }
    match &point.branch {
        Some(branch) => {
            git(
                repo,
                &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")],
            )?;
        }
        None => {
            git(repo, &["update-ref", "--no-deref", "HEAD", &point.head])?;
        }
    }
    // A branch the recipe made, now that nothing is on it.
    for name in &point.makes {
        let made = format!("refs/heads/{name}");
        if shell::ran(repo, &["rev-parse", "--verify", "-q", &made])?.ok() {
            git(repo, &["update-ref", "-d", &made])?;
        }
    }

    // The index alone: what was untracked is untracked again.
    git(repo, &["read-tree", &format!("{under}/index")])?;

    // The stash list. What the run pushed goes, newest first so the numbers
    // hold; its changes are in the tree that was just put back. What the run
    // popped is stored again, oldest first.
    let now = stashes(repo)?;
    let was: Vec<&str> = point.stashes.iter().map(|s| s.id.as_str()).collect();
    for (at, stash) in now.iter().enumerate().rev() {
        if !was.contains(&stash.id.as_str()) {
            git(repo, &["stash", "drop", &format!("stash@{{{at}}}")])?;
        }
    }
    let kept: Vec<String> = stashes(repo)?.into_iter().map(|s| s.id).collect();
    for stash in point.stashes.iter().rev() {
        if !kept.contains(&stash.id) {
            git(repo, &["stash", "store", "-m", &stash.message, &stash.id])?;
        }
    }
    Ok(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// Stage everything, new files too, and commit.
    fn save(fixture: &Fixture, message: &str) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit(message)
    }

    /// A repository with history, a second branch, and work in progress of
    /// every kind: staged, unstaged, untracked, and a stash.
    fn busy() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        fixture.write("b.txt", "bee\n");
        save(&fixture, "First");
        fixture.git(&["branch", "other"]);
        fixture.write("a.txt", "one\ntwo\n");
        save(&fixture, "Second");

        fixture.write("b.txt", "stashed bee\n");
        fixture.git(&["stash", "push", "-m", "earlier work"]);

        fixture.write("a.txt", "one\ntwo\nstaged\n");
        fixture.git(&["add", "a.txt"]);
        fixture.write("a.txt", "one\ntwo\nstaged\nunstaged\n");
        fixture.write("new.txt", "untracked\n");
        fixture
    }

    /// Everything a restore point promises to put back, as text.
    fn state(fixture: &Fixture) -> Vec<String> {
        // A file that is not there reads as nothing: the run takes one away.
        let on_disk = |path: &str| std::fs::read_to_string(fixture.at(path)).unwrap_or_default();
        vec![
            fixture.git(&["rev-parse", "HEAD"]),
            fixture.git(&["symbolic-ref", "-q", "--short", "HEAD"]),
            fixture.git(&["rev-parse", "main", "other"]),
            fixture.git(&["status", "--porcelain"]),
            fixture.git(&["diff", "--cached"]),
            fixture.git(&["diff"]),
            fixture.git(&["stash", "list", "--format=%H %gs"]),
            on_disk("a.txt"),
            on_disk("b.txt"),
            on_disk("new.txt"),
        ]
    }

    fn point(fixture: &Fixture, id: &str, makes: &[&str]) -> RestorePoint {
        create(
            fixture.path(),
            id,
            "a-recipe",
            &["other".to_string()],
            &makes.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            1_800_000_000,
        )
        .expect("a restore point")
    }

    #[test]
    fn making_one_writes_down_what_was_and_changes_nothing() {
        let fixture = busy();
        let before = state(&fixture);

        let made = point(&fixture, "100-aaaa", &[]);

        assert_eq!(state(&fixture), before);
        assert_eq!(made.branch.as_deref(), Some("main"));
        assert_eq!(made.head, fixture.head());
        assert_eq!(made.recipe, "a-recipe");
        let names: Vec<&str> = made.tips.iter().map(|tip| tip.name.as_str()).collect();
        assert_eq!(names, ["other", "main"]);
        assert_eq!(made.stashes.len(), 1);
        assert!(made.stashes[0].message.contains("earlier work"));
        // The working tree's snapshot holds the file git did not know about.
        let work = fixture.git(&[
            "ls-tree",
            "--name-only",
            "refs/spagitty/restore/100-aaaa/work",
        ]);
        assert_eq!(
            work.lines().collect::<Vec<_>>(),
            ["a.txt", "b.txt", "new.txt"]
        );
        // And no leftover of the index it was made through.
        let git_dir = fixture.path().join(".git");
        assert!(std::fs::read_dir(git_dir)
            .expect("the git directory")
            .filter_map(|entry| entry.ok())
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .contains("spagitty-restore")));
        assert_eq!(read(fixture.path(), "100-aaaa").expect("read back"), made);
    }

    /// The item: stash with untracked files, merge, commit and a new branch,
    /// then undone.
    #[test]
    fn undoing_puts_back_head_the_branches_the_index_the_tree_and_the_stashes() {
        let fixture = busy();
        let before = state(&fixture);
        point(&fixture, "200-bbbb", &["made-by-run"]);

        // What a recipe might do, and then some.
        fixture.git(&["stash", "push", "-u", "-m", "the run's stash"]);
        fixture.git(&["switch", "-c", "made-by-run"]);
        fixture.write("c.txt", "added by the run\n");
        save(&fixture, "The run's commit");
        fixture.git(&["switch", "main"]);
        fixture.git(&["merge", "--no-ff", "--no-edit", "made-by-run"]);
        fixture.git(&["branch", "-f", "other", "main"]);
        fixture.git(&["stash", "drop", "stash@{1}"]);
        assert_ne!(state(&fixture), before);

        undo(fixture.path(), "200-bbbb").expect("undone");

        assert_eq!(state(&fixture), before);
        // The file the run committed is gone with the commit, and the branch
        // the run made is gone too.
        assert!(!fixture.at("c.txt").exists());
        assert!(fixture
            .git(&["branch", "--list", "made-by-run"])
            .trim()
            .is_empty());
        // The restore point is still there, to be used again.
        assert_eq!(list(fixture.path()).expect("listed").len(), 1);
    }

    #[test]
    fn undoing_ends_a_merge_that_stopped_on_a_conflict() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "base\n");
        save(&fixture, "Base");
        fixture.git(&["switch", "-c", "other"]);
        fixture.write("a.txt", "theirs\n");
        save(&fixture, "Theirs");
        fixture.git(&["switch", "main"]);
        fixture.write("a.txt", "ours\n");
        save(&fixture, "Ours");
        fixture.write("notes.txt", "mine, untracked\n");
        let before = [
            fixture.head(),
            fixture.git(&["status", "--porcelain"]),
            fixture.read("a.txt"),
        ];
        point(&fixture, "300-cccc", &[]);

        let merged = shell::ran(fixture.path(), &["merge", "--no-edit", "other"]).expect("ran");
        assert!(!merged.ok());
        assert!(fixture.path().join(".git/MERGE_HEAD").exists());

        undo(fixture.path(), "300-cccc").expect("undone");

        assert!(!fixture.path().join(".git/MERGE_HEAD").exists());
        assert_eq!(
            [
                fixture.head(),
                fixture.git(&["status", "--porcelain"]),
                fixture.read("a.txt"),
            ],
            before
        );
        assert_eq!(fixture.read("notes.txt"), "mine, untracked\n");
    }

    #[test]
    fn undoing_goes_back_to_a_detached_head() {
        let fixture = Fixture::linear(3);
        let first = fixture.rev("HEAD~2");
        fixture.git(&["checkout", "-q", "--detach", &first]);
        let made = create(fixture.path(), "400-dddd", "r", &[], &[], 1).expect("made");
        assert_eq!(made.branch, None);

        fixture.git(&["switch", "-q", "main"]);
        undo(fixture.path(), "400-dddd").expect("undone");

        assert_eq!(fixture.head(), first);
        assert!(!shell::ran(fixture.path(), &["symbolic-ref", "-q", "HEAD"])
            .expect("ran")
            .ok());
        assert!(fixture.git(&["status", "--porcelain"]).trim().is_empty());
    }

    /// A file git never knew about is never deleted, by a restore point or
    /// by going back to one.
    #[test]
    fn undoing_never_deletes_a_file_git_did_not_know_about() {
        let fixture = Fixture::linear(2);
        point_plain(&fixture, "500-eeee");

        fixture.write("made-later.txt", "written after the restore point\n");
        undo(fixture.path(), "500-eeee").expect("undone");

        assert_eq!(
            fixture.read("made-later.txt"),
            "written after the restore point\n"
        );
    }

    fn point_plain(fixture: &Fixture, id: &str) -> RestorePoint {
        create(fixture.path(), id, "r", &[], &[], 1_800_000_000).expect("made")
    }

    #[test]
    fn a_repository_with_no_commit_has_nothing_to_go_back_to() {
        let fixture = Fixture::empty();

        let refused = create(fixture.path(), "600-ffff", "r", &[], &[], 1).expect_err("refused");

        assert!(refused.to_string().contains("no commit yet"));
        assert!(list(fixture.path()).expect("listed").is_empty());
    }

    #[test]
    fn conflicts_in_the_index_refuse_a_restore_point() {
        let fixture = Fixture::conflicted();

        let refused = create(fixture.path(), "700-abcd", "r", &[], &[], 1).expect_err("refused");

        assert!(refused.to_string().contains("conflicts"));
    }

    #[test]
    fn they_are_listed_newest_first_and_dropped_one_at_a_time() {
        let fixture = Fixture::linear(1);
        for (id, when) in [("1-a", 10), ("3-c", 30), ("2-b", 20)] {
            create(fixture.path(), id, id, &[], &[], when).expect("made");
        }

        let ids = |fixture: &Fixture| -> Vec<String> {
            list(fixture.path())
                .expect("listed")
                .into_iter()
                .map(|point| point.id)
                .collect()
        };
        assert_eq!(ids(&fixture), ["3-c", "2-b", "1-a"]);

        drop_point(fixture.path(), "2-b").expect("dropped");

        assert_eq!(ids(&fixture), ["3-c", "1-a"]);
        assert!(fixture
            .git(&["for-each-ref", "refs/spagitty/restore/2-b/"])
            .trim()
            .is_empty());
        assert!(read(fixture.path(), "2-b").is_err());
    }

    fn at(id: &str, created: u64) -> RestorePoint {
        RestorePoint {
            id: id.into(),
            created,
            recipe: String::new(),
            branch: None,
            head: String::new(),
            tips: Vec::new(),
            makes: Vec::new(),
            stashes: Vec::new(),
        }
    }

    #[test]
    fn the_newest_twenty_are_kept_and_none_older_than_seven_days() {
        let day = 24 * 60 * 60;
        let now = 100 * day;
        // Twenty-five made today, newest first.
        let today: Vec<RestorePoint> = (0..25).map(|n| at(&format!("t{n}"), now - n)).collect();
        assert_eq!(expired(&today, now), ["t20", "t21", "t22", "t23", "t24"]);

        let aged = [
            at("fresh", now - day),
            at("week", now - 7 * day),
            at("old", now - 7 * day - 1),
        ];
        assert_eq!(expired(&aged, now), ["old"]);
        assert_eq!(KEEP, 20);
        assert_eq!(KEEP_DAYS, 7);
    }

    #[test]
    fn pruning_drops_what_is_past_keeping() {
        let fixture = Fixture::linear(1);
        let day = 24 * 60 * 60;
        create(fixture.path(), "new", "r", &[], &[], 50 * day).expect("made");
        create(fixture.path(), "old", "r", &[], &[], 10 * day).expect("made");

        assert_eq!(prune(fixture.path(), 50 * day + 60).expect("pruned"), 1);

        let left = list(fixture.path()).expect("listed");
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, "new");
    }

    #[test]
    fn two_made_in_one_second_have_ids_of_their_own() {
        let a = new_id(1_800_000_000);
        let b = new_id(1_800_000_000);

        assert_ne!(a, b);
        assert!(a.starts_with("1800000000-"));
        assert!(crate::recipes::validate::branch_name_ok(&a));
    }
}

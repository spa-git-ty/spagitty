// SPDX-License-Identifier: GPL-3.0-or-later

//! Landing a merge (FEAT-101): the result committed, and the receiving branch
//! moved to it, without checking anything out.
//!
//! The result is built from the dry run rather than by re-doing the merge in
//! the user's working tree: the tree `merge-tree` printed, with each conflicted
//! path replaced by what was chosen for it, written through an index of its
//! own; `commit-tree` makes the commit. Only then does a ref move, and it moves
//! against the tip that was read, so a branch somebody moved in the meantime is
//! refused rather than overwritten.
//!
//! Where the receiving branch is checked out — here or in another worktree —
//! the branch is brought forward in that worktree with `merge --ff-only`, which
//! updates its files and refuses rather than overwrite uncommitted work. A
//! branch that is not checked out anywhere is moved with `update-ref`, and no
//! working tree is touched at all.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{checked_out, dry_run, kind_of, preferred, Conflicted, DryRun, RefKind, Scratch};
use crate::conflicts::{self, Side};
use crate::error::{Error, Result};
use crate::repo::workdir;
use crate::shell;

/// Where the result lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    /// B comes into A.
    A,
    /// A comes into B.
    B,
    /// A new branch, starting from A, that B comes into.
    New,
}

/// How it lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Strategy {
    /// One new commit with both tips as parents.
    Merge,
    /// One new commit with the receiving tip as its only parent.
    Squash,
    /// The source's commits replayed on the receiving tip.
    Rebase,
    /// The receiving branch moved to the source's tip. No new commit.
    #[serde(rename = "ff")]
    FastForward,
}

/// What lands at one conflicted path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub path: String,
    /// The file's new content, exactly.
    #[serde(default)]
    pub text: Option<String>,
    /// One side's version, whole: A is `ours`, B is `theirs`. A side that has
    /// no version deletes the path.
    #[serde(default)]
    pub take: Option<Side>,
}

/// Everything a merge needs to land, as the plan read it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandAsk {
    pub a: String,
    pub b: String,
    /// The tips the plan was worked out for. Either having moved is refused.
    pub a_tip: String,
    pub b_tip: String,
    pub target: Target,
    /// The new branch's name, for [`Target::New`].
    #[serde(default)]
    pub new_name: Option<String>,
    pub strategy: Strategy,
    /// The commit message; empty or absent means the default.
    #[serde(default)]
    pub message: Option<String>,
    /// One per conflicted path. Empty for a merge with no conflicts.
    #[serde(default)]
    pub resolutions: Vec<Resolution>,
}

/// What landed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Landed {
    /// The branch that received it.
    pub target: String,
    /// Where that branch is now.
    pub commit: String,
    pub short: String,
    /// Commits written: 1 for a merge or a squash, the replayed ones for a
    /// rebase, 0 for a fast-forward.
    pub written: usize,
}

/// The default message for `strategy`, the way `git merge` words its own.
pub fn default_message(strategy: Strategy, source: &str, target: &str) -> String {
    match strategy {
        Strategy::Squash => format!("Squash {source} into {target}"),
        _ => format!("Merge branch '{source}' into {target}"),
    }
}

/// Land the merge `ask` describes.
pub fn land(repo: &gix::Repository, ask: &LandAsk) -> Result<Landed> {
    let dir = workdir(repo)?;
    if shell::commit_id(dir, &ask.a)? != ask.a_tip {
        return Err(Error::Stale(ask.a.clone()));
    }
    if shell::commit_id(dir, &ask.b)? != ask.b_tip {
        return Err(Error::Stale(ask.b.clone()));
    }

    let (onto_tip, source_tip, source) = match ask.target {
        Target::B => (&ask.b_tip, &ask.a_tip, &ask.a),
        Target::A | Target::New => (&ask.a_tip, &ask.b_tip, &ask.b),
    };
    let target = receiving(repo, dir, ask)?;

    if shell::is_ancestor(dir, source_tip, onto_tip)? {
        return Err(Error::NotStageable(format!(
            "nothing comes in: {target} already has everything in {source}"
        )));
    }

    let message = match ask.message.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => default_message(ask.strategy, source, &target),
    };

    let (commit, written) = match ask.strategy {
        Strategy::FastForward => {
            if !shell::is_ancestor(dir, onto_tip, source_tip)? {
                return Err(Error::NotStageable(format!(
                    "{target} cannot fast-forward: both branches have commits the other lacks"
                )));
            }
            (source_tip.clone(), 0)
        }
        Strategy::Merge | Strategy::Squash => {
            let tree = merged_tree(
                repo,
                &ask.a_tip,
                &ask.b_tip,
                &ask.resolutions,
                preferred(dir),
            )?;
            let parents: Vec<&str> = if ask.strategy == Strategy::Merge {
                vec![onto_tip, source_tip]
            } else {
                vec![onto_tip]
            };
            (shell::commit_tree(dir, &tree, &parents, &message)?, 1)
        }
        Strategy::Rebase => {
            if !ask.resolutions.is_empty() {
                return Err(Error::NotStageable(
                    "a rebase is resolved one commit at a time".into(),
                ));
            }
            replay(repo, onto_tip, source_tip)?
        }
    };

    let reason = format!("merger: {} {source} into {target}", verb(ask.strategy));
    move_to(repo, dir, ask.target, &target, &commit, onto_tip, &reason)?;
    Ok(Landed {
        short: commit.chars().take(7).collect(),
        commit,
        target,
        written,
    })
}

fn verb(strategy: Strategy) -> &'static str {
    match strategy {
        Strategy::Merge => "merge",
        Strategy::Squash => "squash",
        Strategy::Rebase => "rebase",
        Strategy::FastForward => "fast-forward",
    }
}

/// The branch that receives the result, checked: a branch here, or a new
/// name nothing has yet.
fn receiving(repo: &gix::Repository, dir: &Path, ask: &LandAsk) -> Result<String> {
    match ask.target {
        Target::A | Target::B => {
            let name = if ask.target == Target::A {
                &ask.a
            } else {
                &ask.b
            };
            if kind_of(repo, name) != RefKind::Local {
                return Err(Error::NotStageable(format!(
                    "{name} is not a branch here; merge into a new branch instead"
                )));
            }
            Ok(name.clone())
        }
        Target::New => {
            let name = ask.new_name.as_deref().map(str::trim).unwrap_or_default();
            if name.is_empty() {
                return Err(Error::NotStageable("the new branch needs a name".into()));
            }
            if !shell::valid_branch_name(dir, name)? {
                return Err(Error::NotStageable(format!(
                    "{name} is not a name a branch can have"
                )));
            }
            if kind_of(repo, name) == RefKind::Local {
                return Err(Error::NotStageable(format!("{name} already exists")));
            }
            Ok(name.to_string())
        }
    }
}

/// Move `target` from `old` to `commit`, or create it there.
fn move_to(
    repo: &gix::Repository,
    dir: &Path,
    how: Target,
    target: &str,
    commit: &str,
    old: &str,
    reason: &str,
) -> Result<()> {
    if how == Target::New {
        return shell::branch_at(dir, target, commit);
    }
    match checked_out(repo).get(target) {
        Some(path) => {
            let worktree = Path::new(path);
            if shell::commit_id(worktree, "HEAD")? != old {
                return Err(Error::Stale(target.to_string()));
            }
            shell::fast_forward(worktree, commit)
        }
        None => shell::move_branch(dir, target, commit, old, reason),
    }
}

/// The merged tree of `ours` and `theirs` with every conflict resolved.
///
/// Every conflicted path needs exactly one resolution, and a resolution for a
/// path that did not conflict is refused: either means the screen is showing
/// a different merge from the one git just did.
pub(crate) fn merged_tree(
    repo: &gix::Repository,
    ours: &str,
    theirs: &str,
    resolutions: &[Resolution],
    method: DryRun,
) -> Result<String> {
    let dir = workdir(repo)?;
    match method {
        DryRun::MergeTree => {
            let outcome = dry_run(repo, ours, theirs, method)?;
            let tree = outcome.tree.clone().expect("merge-tree prints a tree");
            let chosen = matched(&outcome.conflicted, resolutions)?;
            if chosen.is_empty() {
                return Ok(tree);
            }
            let mut entries = String::new();
            for (conflicted, resolution) in chosen {
                entries.push_str(&entry(dir, conflicted, resolution)?);
            }
            let index = super::scratch_root(repo).join(format!("index-{}", super::unique_name()));
            std::fs::create_dir_all(index.parent().expect("an index has a folder"))?;
            let built = shell::tree_with(dir, &index, &tree, &entries);
            let _ = std::fs::remove_file(&index);
            built
        }
        DryRun::Worktree => {
            let scratch = Scratch::add(repo, ours)?;
            shell::scratch_merge(scratch.path(), theirs)?;
            let unmerged = shell::unmerged_entries(scratch.path())?;
            let conflicted: Vec<Conflicted> = super::parse_unmerged(&unmerged)
                .into_iter()
                .map(|(path, stages)| super::conflicted_from(path, stages, None))
                .collect();
            for (conflicted, resolution) in matched(&conflicted, resolutions)? {
                let file = scratch.path().join(&conflicted.path);
                match content(repo, conflicted, resolution)? {
                    Some(bytes) => std::fs::write(&file, bytes)?,
                    None => {
                        let _ = std::fs::remove_file(&file);
                    }
                }
            }
            shell::write_all_as_tree(scratch.path())
        }
    }
}

/// Each conflicted path with its resolution, or the reason they do not match.
fn matched<'a>(
    conflicted: &'a [Conflicted],
    resolutions: &'a [Resolution],
) -> Result<Vec<(&'a Conflicted, &'a Resolution)>> {
    if let Some(stray) = resolutions
        .iter()
        .find(|r| !conflicted.iter().any(|c| c.path == r.path))
    {
        return Err(Error::Stale(stray.path.clone()));
    }
    conflicted
        .iter()
        .map(|c| {
            let mut found = resolutions.iter().filter(|r| r.path == c.path);
            match (found.next(), found.next()) {
                (Some(resolution), None) => Ok((c, resolution)),
                (None, _) => Err(Error::NotStageable(format!(
                    "{} is not resolved yet",
                    c.path
                ))),
                (Some(_), Some(_)) => Err(Error::NotStageable(format!(
                    "{} was resolved twice",
                    c.path
                ))),
            }
        })
        .collect()
}

/// The bytes that land at a path, or `None` to delete it.
fn content(
    repo: &gix::Repository,
    conflicted: &Conflicted,
    resolution: &Resolution,
) -> Result<Option<Vec<u8>>> {
    match (&resolution.text, resolution.take) {
        (Some(text), None) => {
            if !conflicts::regions(text).is_empty() {
                return Err(Error::NotStageable(format!(
                    "{} still has conflict markers",
                    conflicted.path
                )));
            }
            Ok(Some(text.clone().into_bytes()))
        }
        (None, Some(side)) => match stage(conflicted, side) {
            Some((_, blob)) => Ok(Some(read_blob(repo, blob)?)),
            None => Ok(None),
        },
        _ => Err(Error::NotStageable(format!(
            "{} needs either text or a side",
            conflicted.path
        ))),
    }
}

fn stage(conflicted: &Conflicted, side: Side) -> Option<&(String, String)> {
    match side {
        Side::Ours => conflicted.ours.as_ref(),
        Side::Theirs => conflicted.theirs.as_ref(),
    }
}

/// One `update-index --index-info` line for a resolution.
fn entry(dir: &Path, conflicted: &Conflicted, resolution: &Resolution) -> Result<String> {
    let mode = conflicted
        .ours
        .as_ref()
        .or(conflicted.theirs.as_ref())
        .or(conflicted.base.as_ref())
        .map(|(mode, _)| mode.clone())
        .unwrap_or_else(|| "100644".into());
    match (&resolution.text, resolution.take) {
        (Some(text), None) => {
            if !conflicts::regions(text).is_empty() {
                return Err(Error::NotStageable(format!(
                    "{} still has conflict markers",
                    conflicted.path
                )));
            }
            let blob = shell::hash_blob(dir, &conflicted.path, text.as_bytes())?;
            Ok(format!("{mode} {blob}\t{}\n", conflicted.path))
        }
        (None, Some(side)) => Ok(match stage(conflicted, side) {
            Some((mode, blob)) => format!("{mode} {blob}\t{}\n", conflicted.path),
            None => {
                let zero = "0".repeat(blob_length(conflicted));
                format!("0 {zero}\t{}\n", conflicted.path)
            }
        }),
        _ => Err(Error::NotStageable(format!(
            "{} needs either text or a side",
            conflicted.path
        ))),
    }
}

/// The length of an object id in this repository: 40 for SHA-1, 64 for SHA-256.
fn blob_length(conflicted: &Conflicted) -> usize {
    [&conflicted.base, &conflicted.ours, &conflicted.theirs]
        .into_iter()
        .flatten()
        .map(|(_, blob)| blob.len())
        .next()
        .unwrap_or(40)
}

fn read_blob(repo: &gix::Repository, blob: &str) -> Result<Vec<u8>> {
    let id = gix::ObjectId::from_hex(blob.as_bytes()).map_err(|e| Error::Diff(e.to_string()))?;
    let object = repo
        .find_object(id)
        .map_err(|e| Error::Diff(e.to_string()))?;
    Ok(object.detach().data)
}

/// Replay `source`'s commits since the split onto `onto`, in a scratch
/// worktree, and return the new tip and how many commits were written.
///
/// A replay that stops on a conflict is undone with the worktree, and the
/// error says so: resolving a rebase one commit at a time is FEAT-103.
fn replay(repo: &gix::Repository, onto: &str, source: &str) -> Result<(String, usize)> {
    let dir = workdir(repo)?;
    let base = shell::merge_base(dir, onto, source)?
        .ok_or_else(|| Error::NotStageable("the branches share no history".into()))?;
    let scratch = Scratch::add(repo, source)?;
    if let Err(error) = shell::rebase(scratch.path(), onto, &base, "") {
        let _ = shell::rebase_abort(scratch.path());
        return Err(Error::NotStageable(format!(
            "the rebase stopped on a conflict, so nothing was written: {error}"
        )));
    }
    let tip = shell::commit_id(scratch.path(), "HEAD")?;
    let written = shell::count_commits(dir, &format!("{onto}..{tip}"))?;
    Ok((tip, written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// main and feat, split at Base, apart in different files: merges cleanly.
    ///
    /// ```text
    /// main: Base ─ M1 (main.txt)
    /// feat: Base ─ F1 (feat.txt) ─ F2 (feat.txt again)
    /// ```
    fn apart() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("shared.txt", "one\ntwo\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.write("feat.txt", "feat\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("F1");
        fixture.write("feat.txt", "feat, again\n");
        fixture.commit_all("F2");
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("main.txt", "main\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("M1");
        fixture
    }

    fn ask(fixture: &Fixture, target: Target, strategy: Strategy) -> LandAsk {
        LandAsk {
            a: "main".into(),
            b: "feat".into(),
            a_tip: fixture.rev("main"),
            b_tip: fixture.rev("feat"),
            target,
            new_name: None,
            strategy,
            message: None,
            resolutions: Vec::new(),
        }
    }

    fn parents(fixture: &Fixture, commit: &str) -> Vec<String> {
        fixture
            .git(&["rev-list", "--parents", "-n1", commit])
            .split_whitespace()
            .skip(1)
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn a_merge_into_the_checked_out_branch_moves_it_and_its_files() {
        let fixture = apart();
        let main = fixture.rev("main");
        let feat = fixture.rev("feat");

        let landed = land(&fixture.open(), &ask(&fixture, Target::A, Strategy::Merge)).unwrap();

        assert_eq!(landed.target, "main");
        assert_eq!(fixture.rev("main"), landed.commit);
        assert_eq!(parents(&fixture, &landed.commit), vec![main, feat]);
        assert_eq!(
            fixture.read("feat.txt"),
            "feat, again\n",
            "the working tree followed"
        );
        assert!(fixture.git(&["status", "--porcelain"]).is_empty());
        assert_eq!(
            fixture.git(&["log", "-1", "--format=%s"]).trim(),
            "Merge branch 'feat' into main"
        );
    }

    #[test]
    fn a_merge_into_a_branch_that_is_not_checked_out_leaves_the_working_tree_alone() {
        let fixture = apart();
        let main = fixture.rev("main");
        fixture.write("main.txt", "uncommitted work\n");
        let index = std::fs::read(fixture.at(".git/index")).unwrap();

        let landed = land(&fixture.open(), &ask(&fixture, Target::B, Strategy::Merge)).unwrap();

        assert_eq!(landed.target, "feat");
        assert_eq!(fixture.rev("feat"), landed.commit);
        assert_eq!(fixture.rev("main"), main, "main did not move");
        assert_eq!(fixture.git(&["branch", "--show-current"]).trim(), "main");
        assert_eq!(fixture.read("main.txt"), "uncommitted work\n");
        assert_eq!(std::fs::read(fixture.at(".git/index")).unwrap(), index);
        assert!(
            !fixture.at("feat.txt").exists(),
            "feat's files did not appear here"
        );
        let tree = fixture.git(&["ls-tree", "--name-only", &landed.commit]);
        assert!(
            tree.contains("main.txt") && tree.contains("feat.txt"),
            "{tree}"
        );
    }

    #[test]
    fn a_squash_is_one_commit_on_the_target_alone() {
        let fixture = apart();
        let main = fixture.rev("main");
        let mut squash = ask(&fixture, Target::A, Strategy::Squash);
        squash.message = Some("Squash feat".into());

        let landed = land(&fixture.open(), &squash).unwrap();

        assert_eq!(parents(&fixture, &landed.commit), vec![main]);
        assert_eq!(
            fixture.git(&["log", "-1", "--format=%s"]).trim(),
            "Squash feat"
        );
        assert_eq!(fixture.read("feat.txt"), "feat, again\n");
    }

    #[test]
    fn a_new_branch_starts_from_a_and_nothing_else_moves() {
        let fixture = apart();
        let main = fixture.rev("main");
        let feat = fixture.rev("feat");
        let mut new = ask(&fixture, Target::New, Strategy::Merge);
        new.new_name = Some("merge/main-feat".into());

        let landed = land(&fixture.open(), &new).unwrap();

        assert_eq!(fixture.rev("merge/main-feat"), landed.commit);
        assert_eq!(
            parents(&fixture, &landed.commit),
            vec![main.clone(), feat.clone()]
        );
        assert_eq!(fixture.rev("main"), main);
        assert_eq!(fixture.rev("feat"), feat);
    }

    #[test]
    fn a_rebase_replays_the_source_and_leaves_it_where_it_was() {
        let fixture = apart();
        let main = fixture.rev("main");
        let feat = fixture.rev("feat");

        let landed = land(&fixture.open(), &ask(&fixture, Target::A, Strategy::Rebase)).unwrap();

        assert_eq!(landed.written, 2);
        assert_eq!(fixture.rev("feat"), feat, "the source was not moved");
        assert_eq!(fixture.rev("main"), landed.commit);
        assert_eq!(
            fixture.rev("main~2"),
            main,
            "a straight line on top of main"
        );
        assert_eq!(parents(&fixture, &landed.commit).len(), 1);
        assert_eq!(fixture.read("feat.txt"), "feat, again\n");
        let listed = fixture.git(&["worktree", "list", "--porcelain"]);
        assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    }

    #[test]
    fn a_fast_forward_only_moves_the_pointer() {
        let fixture = apart();
        fixture.git(&["branch", "behind", "main~1"]);
        let mut forward = ask(&fixture, Target::A, Strategy::FastForward);
        forward.a = "behind".into();
        forward.a_tip = fixture.rev("behind");
        forward.b = "main".into();
        forward.b_tip = fixture.rev("main");

        let landed = land(&fixture.open(), &forward).unwrap();

        assert_eq!(landed.written, 0);
        assert_eq!(fixture.rev("behind"), fixture.rev("main"));
    }

    #[test]
    fn fast_forward_is_refused_when_both_have_their_own() {
        let fixture = apart();
        let error = land(
            &fixture.open(),
            &ask(&fixture, Target::A, Strategy::FastForward),
        )
        .unwrap_err();
        assert!(error.to_string().contains("cannot fast-forward"), "{error}");
    }

    #[test]
    fn a_branch_that_moved_since_the_plan_is_refused() {
        let fixture = apart();
        let stale = ask(&fixture, Target::B, Strategy::Merge);
        fixture.git(&["branch", "-f", "feat", "feat~1"]);

        let error = land(&fixture.open(), &stale).unwrap_err();

        assert!(
            matches!(error, Error::Stale(ref name) if name == "feat"),
            "{error}"
        );
    }

    #[test]
    fn a_remote_branch_cannot_receive() {
        let fixture = apart();
        fixture.git(&["update-ref", "refs/remotes/origin/feat", "feat"]);
        let mut remote = ask(&fixture, Target::B, Strategy::Merge);
        remote.b = "origin/feat".into();

        let error = land(&fixture.open(), &remote).unwrap_err();

        assert!(error.to_string().contains("not a branch here"), "{error}");
    }

    #[test]
    fn a_merge_with_conflicts_needs_every_one_resolved() {
        let fixture = apart();
        fixture.write("shared.txt", "one\nTWO main\n");
        fixture.commit_all("M2");
        fixture.git(&["switch", "-q", "feat"]);
        fixture.write("shared.txt", "one\nTWO feat\n");
        fixture.commit_all("F3");
        fixture.git(&["switch", "-q", "main"]);

        let mut merge = ask(&fixture, Target::A, Strategy::Merge);
        let error = land(&fixture.open(), &merge).unwrap_err();
        assert!(
            error.to_string().contains("shared.txt is not resolved"),
            "{error}"
        );

        merge.resolutions = vec![Resolution {
            path: "shared.txt".into(),
            text: Some("one\nTWO both\n".into()),
            take: None,
        }];
        land(&fixture.open(), &merge).unwrap();
        assert_eq!(fixture.read("shared.txt"), "one\nTWO both\n");
    }

    #[test]
    fn the_fallback_builds_the_same_tree() {
        let fixture = apart();
        fixture.write("shared.txt", "one\nTWO main\n");
        fixture.commit_all("M2");
        fixture.git(&["switch", "-q", "feat"]);
        fixture.write("shared.txt", "one\nTWO feat\n");
        fixture.commit_all("F3");
        fixture.git(&["switch", "-q", "main"]);
        let repo = fixture.open();
        let (a, b) = (fixture.rev("main"), fixture.rev("feat"));
        let chosen = [Resolution {
            path: "shared.txt".into(),
            text: None,
            take: Some(Side::Theirs),
        }];

        let fast = merged_tree(&repo, &a, &b, &chosen, DryRun::MergeTree).unwrap();
        let slow = merged_tree(&repo, &a, &b, &chosen, DryRun::Worktree).unwrap();

        assert_eq!(fast, slow);
        let text = fixture.git(&["show", &format!("{fast}:shared.txt")]);
        assert_eq!(text, "one\nTWO feat\n");
    }

    #[test]
    fn text_with_markers_left_in_is_refused() {
        let fixture = apart();
        fixture.write("shared.txt", "one\nTWO main\n");
        fixture.commit_all("M2");
        fixture.git(&["switch", "-q", "feat"]);
        fixture.write("shared.txt", "one\nTWO feat\n");
        fixture.commit_all("F3");
        fixture.git(&["switch", "-q", "main"]);
        let mut merge = ask(&fixture, Target::A, Strategy::Merge);
        merge.resolutions = vec![Resolution {
            path: "shared.txt".into(),
            text: Some("<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n".into()),
            take: None,
        }];

        let error = land(&fixture.open(), &merge).unwrap_err();

        assert!(error.to_string().contains("conflict markers"), "{error}");
        assert_eq!(fixture.rev("main"), merge.a_tip, "nothing moved");
    }

    #[test]
    fn a_deleted_side_taken_deletes_the_file() {
        let fixture = Fixture::empty();
        fixture.write("gone.txt", "x\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.git(&["rm", "-q", "gone.txt"]);
        fixture.commit("Delete it");
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("gone.txt", "y\n");
        fixture.commit_all("Change it");
        let mut merge = ask(&fixture, Target::A, Strategy::Merge);
        merge.resolutions = vec![Resolution {
            path: "gone.txt".into(),
            text: None,
            take: Some(Side::Theirs),
        }];

        let landed = land(&fixture.open(), &merge).unwrap();

        let tree = fixture.git(&["ls-tree", "--name-only", &landed.commit]);
        assert!(!tree.contains("gone.txt"), "{tree}");
        assert!(!fixture.at("gone.txt").exists());
    }

    #[test]
    fn uncommitted_work_in_the_way_is_refused_by_git_and_nothing_moves() {
        let fixture = apart();
        let main = fixture.rev("main");
        fixture.write("feat.txt", "in the way\n");

        let error = land(&fixture.open(), &ask(&fixture, Target::A, Strategy::Merge)).unwrap_err();

        assert!(matches!(error, Error::Git { .. }), "{error}");
        assert_eq!(fixture.rev("main"), main);
        assert_eq!(fixture.read("feat.txt"), "in the way\n");
    }
}

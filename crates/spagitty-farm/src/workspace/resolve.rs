// SPDX-License-Identifier: GPL-3.0-or-later

//! A conflict, opened in the task's own workspace for somebody to resolve
//! (FEAT-146).
//!
//! When the lane meets a conflict it writes nothing (FEAT-144). If the farm's
//! Rules say an agent or a tool resolves it, this starts the merge of the
//! destination into the *task's* workspace — its worktree or its copy, never
//! the person's working copy — and leaves the conflicted files for the
//! resolver. The resolution is committed on the task's branch as an ordinary
//! merge, so the task goes through its checks and its review again before the
//! lane takes it: a resolved conflict is reviewed like any other change.

use std::path::Path;

use spagitty_core::shell;

use crate::error::{Error, Result};

/// What starting the merge found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// The destination moved and the merge is clean now. Committed.
    Clean(String),
    /// These paths are in conflict, and the merge waits in the workspace.
    Conflicted(Vec<String>),
}

/// Merge `commit`, the tip of `destination`, into the branch checked out in
/// `workdir`, without committing, and say what is in conflict.
///
/// For a copy the branch is fetched in first by path from `repo`, since a copy
/// has no remote; for a worktree the commit is already in the shared object
/// store.
pub fn open(
    repo: &Path,
    workdir: &Path,
    destination: &str,
    commit: &str,
    message: &str,
) -> Result<Opened> {
    if shell::ran(
        workdir,
        &["cat-file", "-e", &format!("{commit}^{{commit}}")],
    )
    .map(|ran| !ran.ok())
    .unwrap_or(true)
    {
        let source = repo.to_string_lossy();
        let refspec = format!("+refs/heads/{destination}:refs/spagitty/destination");
        shell::ran_ok(
            workdir,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--no-write-fetch-head",
                &source,
                &refspec,
            ],
        )?;
    }
    let merged = shell::ran(
        workdir,
        &[
            "-c",
            "commit.gpgsign=false",
            "merge",
            "--no-ff",
            "--no-commit",
            "-m",
            message,
            commit,
        ],
    )?;
    let conflicted = unmerged(workdir)?;
    if conflicted.is_empty() {
        if !merged.ok() {
            let _ = abort(workdir);
            return Err(Error::Refused(format!(
                "The merge could not be started: {}",
                merged.output().trim()
            )));
        }
        return Ok(Opened::Clean(commit_merge(workdir, message)?));
    }
    Ok(Opened::Conflicted(conflicted))
}

/// Whether a merge is waiting in `workdir`.
pub fn in_progress(workdir: &Path) -> bool {
    shell::ran(workdir, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .map(|ran| ran.ok())
        .unwrap_or(false)
}

/// The paths git still holds as unmerged.
pub fn unmerged(workdir: &Path) -> Result<Vec<String>> {
    let names = shell::ran_ok(workdir, &["diff", "--name-only", "--diff-filter=U", "-z"])?;
    Ok(names
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect())
}

/// What is not resolved yet: a conflict marker left in one of `paths`, or a
/// path git holds as unmerged that is not among them. A path among them is
/// unmerged in the index until it is staged, which [`finish`] does.
pub fn unresolved(workdir: &Path, paths: &[String]) -> Result<Vec<String>> {
    let mut left: Vec<String> = unmerged(workdir)?
        .into_iter()
        .filter(|path| !paths.contains(path))
        .collect();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(workdir.join(path)) else {
            continue;
        };
        let marked = text.lines().any(|line| {
            line.starts_with("<<<<<<< ") || line == "=======" || line.starts_with(">>>>>>> ")
        });
        if marked {
            left.push(path.clone());
        }
    }
    Ok(left)
}

/// Stage `paths` as resolved and commit the merge.
pub fn finish(workdir: &Path, paths: &[String], message: &str) -> Result<String> {
    let left = unresolved(workdir, paths)?;
    if !left.is_empty() {
        return Err(Error::Refused(format!(
            "Still in conflict: {}.",
            left.join(", ")
        )));
    }
    let mut add = vec!["add", "-A", "--"];
    add.extend(paths.iter().map(String::as_str));
    shell::ran_ok(workdir, &add)?;
    commit_merge(workdir, message)
}

fn commit_merge(workdir: &Path, message: &str) -> Result<String> {
    shell::ran_ok(
        workdir,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--no-verify",
            "-m",
            message,
        ],
    )?;
    Ok(shell::commit_id(workdir, "HEAD")?)
}

/// Give the merge up, leaving the workspace as it was before it started.
pub fn abort(workdir: &Path) -> Result<()> {
    if in_progress(workdir) {
        shell::ran_ok(workdir, &["merge", "--abort"])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentProvider, TaskId};
    use crate::workspace::{copy, worktree};
    use spagitty_core::fixture::Fixture;

    fn commit(dir: &Path, path: &str, text: &str) -> String {
        std::fs::write(dir.join(path), text).unwrap();
        shell::stage(dir, &[path.to_string()]).unwrap();
        shell::commit(dir, &format!("change {path}"), "", false, false).unwrap();
        shell::commit_id(dir, "HEAD").unwrap()
    }

    fn current(repo: &Fixture) -> String {
        shell::ran_ok(repo.path(), &["branch", "--show-current"])
            .unwrap()
            .trim()
            .to_string()
    }

    /// A task's worktree and a destination that both changed `shared.txt`.
    fn conflicting() -> (Fixture, std::path::PathBuf, String) {
        let repo = Fixture::woven();
        commit(repo.path(), "shared.txt", "base\n");
        let workspace = worktree::create(
            repo.path(),
            &TaskId::new("TASK-0005"),
            AgentProvider::Codex,
            "HEAD",
        )
        .unwrap();
        commit(&workspace.path, "shared.txt", "task\n");
        let tip = commit(repo.path(), "shared.txt", "destination\n");
        (repo, workspace.path, tip)
    }

    #[test]
    fn a_conflict_is_opened_in_the_task_s_worktree_and_not_the_person_s() {
        let (repo, workdir, tip) = conflicting();

        let opened = open(
            repo.path(),
            &workdir,
            &current(&repo),
            &tip,
            "Merge main into the task",
        )
        .unwrap();

        assert_eq!(opened, Opened::Conflicted(vec!["shared.txt".into()]));
        assert!(in_progress(&workdir));
        assert!(std::fs::read_to_string(workdir.join("shared.txt"))
            .unwrap()
            .contains("<<<<<<<"));
        assert!(
            !in_progress(repo.path()),
            "the person's working copy was touched"
        );
        assert_eq!(repo.read("shared.txt"), "destination\n");
    }

    #[test]
    fn a_resolution_with_markers_left_is_refused_and_a_clean_one_is_committed() {
        let (repo, workdir, tip) = conflicting();
        open(
            repo.path(),
            &workdir,
            &current(&repo),
            &tip,
            "Merge main into the task",
        )
        .unwrap();
        let paths = vec!["shared.txt".to_string()];

        assert!(finish(&workdir, &paths, "Merge main into the task").is_err());

        std::fs::write(workdir.join("shared.txt"), "task and destination\n").unwrap();
        let merged = finish(&workdir, &paths, "Merge main into the task").unwrap();

        assert!(!in_progress(&workdir));
        let parents =
            shell::ran_ok(&workdir, &["rev-list", "--parents", "-n", "1", &merged]).unwrap();
        assert!(
            parents.contains(&tip),
            "the destination is not a parent: {parents}"
        );
    }

    #[test]
    fn giving_up_leaves_the_worktree_as_it_was() {
        let (repo, workdir, tip) = conflicting();
        open(repo.path(), &workdir, &current(&repo), &tip, "Merge").unwrap();

        abort(&workdir).unwrap();

        assert!(!in_progress(&workdir));
        assert_eq!(
            std::fs::read_to_string(workdir.join("shared.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "task\n"
        );
    }

    #[test]
    fn a_copy_fetches_the_destination_before_merging_it() {
        let repo = Fixture::woven();
        commit(repo.path(), "shared.txt", "base\n");
        let task = TaskId::new("TASK-0006");
        let workspace = copy::create(repo.path(), &task, AgentProvider::Codex, "HEAD").unwrap();
        commit(&workspace.path, "shared.txt", "task\n");
        let tip = commit(repo.path(), "shared.txt", "destination\n");

        let opened = open(repo.path(), &workspace.path, &current(&repo), &tip, "Merge").unwrap();

        assert_eq!(opened, Opened::Conflicted(vec!["shared.txt".into()]));
    }
}

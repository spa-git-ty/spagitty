// SPDX-License-Identifier: GPL-3.0-or-later

//! A working tree of its own for every task.
//!
//! The plan calls this non-negotiable and it is right to. Two agents in one
//! checkout is not a race that shows up occasionally — it is one agent reading
//! a half-written file from another while a third runs the tests over the
//! result, every time. A worktree per task makes each one's edits invisible to
//! the others until the branch is merged, which is the point at which Spagitty
//! is a Git client and can show the user what happened.
//!
//! # Where they go, and why not inside the repository
//!
//! ```text
//! <repo>/.spagitty/farm/worktrees/task-0042/
//! ```
//!
//! Inside the repository, so a user who looks for them finds them next to the
//! code they belong to, and so deleting the repository deletes them. `.spagitty`
//! is added to `.git/info/exclude` when the first one is cut, rather than to
//! `.gitignore`: the ignore file is the user's, it is committed, and a tool that
//! edits it has changed the repository's content without being asked.
//!
//! # Nothing here reimplements git
//!
//! Every operation goes through [`spagitty_core::shell`], which is the module
//! that owns every mutating git command in the product and records what it ran.
//! A worktree created by this crate therefore shows up in the same command log
//! as one the user created from the Worktrees screen.

use std::path::{Path, PathBuf};

use spagitty_core::shell;

use crate::error::{Error, Result};
use crate::model::{AgentProvider, TaskId};
use crate::workspace::branch::{branch_name, worktree_dir};

/// The directory, relative to the repository root, that the farm keeps its
/// state in.
pub const FARM_DIR: &str = ".spagitty";

/// Where a task's worktree lives.
pub fn worktree_path(repo: &Path, task: &TaskId) -> PathBuf {
    repo.join(FARM_DIR)
        .join("farm")
        .join("worktrees")
        .join(worktree_dir(task))
}

/// One task's isolated workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub task: TaskId,
    pub branch: String,
    pub path: PathBuf,
}

/// Cut a branch and a worktree for `task`, starting from `start`.
///
/// Idempotent in the way that matters: if the worktree is already there and
/// already on the right branch, it is returned as it is. A task that failed
/// verification is run again in the workspace it already has — recreating it
/// would throw away the agent's uncommitted work, which is the one thing the
/// retry needs.
pub fn create(
    repo: &Path,
    task: &TaskId,
    provider: AgentProvider,
    start: &str,
) -> Result<Workspace> {
    let branch = branch_name(task, provider);
    let path = worktree_path(repo, task);

    exclude_farm_dir(repo);

    if path.join(".git").exists() {
        return Ok(Workspace {
            task: task.clone(),
            branch,
            path,
        });
    }

    std::fs::create_dir_all(path.parent().unwrap_or(&path))?;

    // `git worktree add -b <branch> <path> <start>`. The branch is created by
    // the same command that checks it out, so there is never a moment where a
    // branch exists with no worktree to work in — which is the state that
    // makes cleanup ambiguous after a crash.
    //
    // A branch left over from a previous attempt is reused rather than being a
    // failure: the commits on it are the previous attempt's work, and the
    // agent should see them.
    let existing = branch_exists(repo, &branch);
    let result = if existing {
        shell::worktree_add(repo, &path, Some(&branch), None, false)
    } else {
        shell::worktree_add(repo, &path, Some(start), Some(&branch), false)
    };

    if let Err(error) = result {
        // A half-created worktree directory left behind would make the next
        // attempt take the "already there" path above and hand back a
        // workspace with no git in it.
        let _ = std::fs::remove_dir_all(&path);
        return Err(Error::Git(error));
    }

    Ok(Workspace {
        task: task.clone(),
        branch,
        path,
    })
}

/// Remove a task's worktree, and its branch when it has been merged.
///
/// `force` throws away uncommitted changes in the worktree. It is what the user
/// pressing "discard this task" means, and it is never the default: an agent
/// that produced work and failed to commit it has still produced work, and a
/// farm that deletes it silently is a farm nobody trusts.
///
/// The branch is deleted without force, always. `git branch -d` refuses a
/// branch that is not merged, and that refusal is the safety here — an
/// unmerged farm branch survives cleanup and stays visible in the graph.
pub fn remove(repo: &Path, task: &TaskId, provider: AgentProvider, force: bool) -> Result<()> {
    let path = worktree_path(repo, task);
    if path.exists() {
        shell::worktree_remove(repo, &path, force)?;
    }
    // A task on a copy farm has a copy instead (FEAT-147); the same rules.
    super::copy::remove(repo, task, provider, force)?;
    let branch = branch_name(task, provider);
    if branch_exists(repo, &branch) {
        // Not fatal: an unmerged branch is *meant* to survive this.
        let _ = shell::delete_branch(repo, &branch, false);
    }
    Ok(())
}

/// Is there a local branch by this name?
///
/// Asked through the worktree listing rather than `git rev-parse`, because
/// `worktree list --porcelain` is already the thing this module reads and one
/// parser is better than two. A branch with no worktree is found by the
/// fallback below it.
fn branch_exists(repo: &Path, branch: &str) -> bool {
    shell::get_config(repo, &format!("branch.{branch}.merge"))
        .ok()
        .flatten()
        .is_some()
        || spagitty_core::worktrees::parse_worktree_porcelain(
            &shell::worktree_list(repo).unwrap_or_default(),
        )
        .iter()
        .any(|worktree| worktree.branch.as_deref() == Some(branch))
        || repo
            .join(".git")
            .join("refs")
            .join("heads")
            .join(branch)
            .exists()
}

/// Keep the farm's own directory out of the user's commits.
///
/// `.git/info/exclude` rather than `.gitignore`, for the reason in the module
/// header. Failures are ignored: an unwritable `info/exclude` costs a noisy
/// `git status`, not the farm.
pub(crate) fn exclude_farm_dir(repo: &Path) {
    let exclude = repo.join(".git").join("info").join("exclude");
    let Some(parent) = exclude.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let current = std::fs::read_to_string(&exclude).unwrap_or_default();
    let entry = format!("/{FARM_DIR}/");
    if current.lines().any(|line| line.trim() == entry) {
        return;
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("# Spagitty's agent farm: worktrees, logs and saved state.\n");
    next.push_str(&entry);
    next.push('\n');
    let _ = std::fs::write(&exclude, next);
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;

    fn task() -> TaskId {
        TaskId::new("TASK-0042")
    }

    #[test]
    fn a_worktree_lands_under_the_farm_directory() {
        let path = worktree_path(Path::new("/repo"), &task());
        assert!(path.ends_with(".spagitty/farm/worktrees/task-0042"));
    }

    #[test]
    fn cutting_a_workspace_produces_a_branch_and_a_checkout() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();

        assert_eq!(workspace.branch, "spagitty-farm/TASK-0042/claude");
        assert!(workspace.path.join(".git").exists(), "no checkout was made");
        // The worktree is a real one as far as git is concerned.
        let listed = spagitty_core::worktrees::parse_worktree_porcelain(
            &shell::worktree_list(repo.path()).unwrap(),
        );
        assert!(listed
            .iter()
            .any(|worktree| worktree.branch.as_deref() == Some("spagitty-farm/TASK-0042/claude")));
    }

    #[test]
    fn the_farm_directory_is_excluded_without_touching_gitignore() {
        let repo = Fixture::woven();
        let before = repo.read(".gitignore");
        create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();

        let exclude =
            std::fs::read_to_string(repo.path().join(".git").join("info").join("exclude")).unwrap();
        assert!(exclude.contains("/.spagitty/"), "{exclude}");
        // `.gitignore` is committed, and belongs to whoever wrote it. Editing
        // it would put a change in the user's working copy that they did not
        // make and would have to explain in a commit.
        assert_eq!(repo.read(".gitignore"), before);
    }

    #[test]
    fn excluding_twice_does_not_write_the_entry_twice() {
        let repo = Fixture::woven();
        create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        create(
            repo.path(),
            &TaskId::new("TASK-0043"),
            AgentProvider::Codex,
            "HEAD",
        )
        .unwrap();

        let exclude =
            std::fs::read_to_string(repo.path().join(".git").join("info").join("exclude")).unwrap();
        assert_eq!(exclude.matches("/.spagitty/").count(), 1, "{exclude}");
    }

    #[test]
    fn asking_twice_returns_the_workspace_that_is_already_there() {
        // A retry must not throw away the previous attempt's uncommitted work.
        let repo = Fixture::woven();
        let first = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        std::fs::write(first.path.join("scratch.txt"), "work in progress").unwrap();

        let second = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        assert_eq!(first, second);
        assert!(second.path.join("scratch.txt").exists());
    }

    #[test]
    fn two_tasks_get_two_directories() {
        let repo = Fixture::woven();
        let a = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        let b = create(
            repo.path(),
            &TaskId::new("TASK-0043"),
            AgentProvider::Codex,
            "HEAD",
        )
        .unwrap();
        assert_ne!(a.path, b.path);
        assert_ne!(a.branch, b.branch);
    }

    #[test]
    fn removing_takes_the_worktree_and_the_merged_branch_away() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        assert!(workspace.path.exists());

        remove(repo.path(), &task(), AgentProvider::ClaudeCode, false).unwrap();
        assert!(!workspace.path.exists(), "the worktree survived");

        let listed = spagitty_core::worktrees::parse_worktree_porcelain(
            &shell::worktree_list(repo.path()).unwrap(),
        );
        assert!(!listed
            .iter()
            .any(|worktree| worktree.branch.as_deref() == Some(&workspace.branch)));
    }

    #[test]
    fn removing_a_workspace_that_was_never_cut_is_not_an_error() {
        let repo = Fixture::woven();
        remove(repo.path(), &task(), AgentProvider::Cursor, false).unwrap();
    }

    #[test]
    fn a_branch_with_unmerged_work_survives_cleanup() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), AgentProvider::ClaudeCode, "HEAD").unwrap();
        std::fs::write(workspace.path.join("new.txt"), "agent output").unwrap();
        shell::stage(&workspace.path, &["new.txt".into()]).unwrap();
        shell::commit(&workspace.path, "agent work", "", false, false).unwrap();

        remove(repo.path(), &task(), AgentProvider::ClaudeCode, true).unwrap();

        // The worktree is gone; the commits are not. This is the whole point of
        // deleting the branch without force.
        assert!(!workspace.path.exists());
        assert!(
            repo.path()
                .join(".git/refs/heads/spagitty-farm/TASK-0042/claude")
                .exists(),
            "an unmerged branch was deleted"
        );
    }
}

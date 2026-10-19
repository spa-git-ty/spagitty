// SPDX-License-Identifier: GPL-3.0-or-later

//! A slice per task (FEAT-143): the task's worktree, holding only what the task
//! may touch and what it must read.
//!
//! A worktree is the whole repository, checked out, and an agent given the
//! whole repository reads and edits more than its task needs. A slice is the
//! same worktree with `git sparse-checkout` limited to the task's paths and the
//! repository's top-level files. The agent edits in the slice; before its
//! checks run the slice is widened to the whole tree, because a build in a
//! workspace with half its members missing is not a build (HUB.md, step 1).
//!
//! # Behind a trait
//!
//! [`SliceProvider`] is the seam `HUB.md` asks for: a provider that hands each
//! task a place to work holding at least its paths. [`WorktreeSlice`] does it
//! with a worktree and `sparse-checkout`. A provider over the network later is
//! a second implementation, not a rewrite of what calls it.

use std::path::{Path, PathBuf};

use spagitty_core::shell;

use crate::error::{Error, Result};
use crate::model::{AgentProvider, Task, TaskId};
use crate::workspace::worktree::{self, Workspace};

/// A place for one task to work, holding at least `paths`.
pub trait SliceProvider {
    /// Open the task's place to work, starting from `start`, narrowed to
    /// `paths` (anchored patterns, as [`patterns`] writes them). An empty list
    /// is the whole tree.
    fn open(&self, task: &TaskId, start: &str, paths: &[String]) -> Result<Workspace>;
    /// Put it away. Uncommitted work is kept unless `force`.
    fn close(&self, task: &TaskId, force: bool) -> Result<()>;
}

/// A slice as a worktree with a sparse checkout.
pub struct WorktreeSlice {
    pub repo: PathBuf,
    pub provider: AgentProvider,
}

impl SliceProvider for WorktreeSlice {
    fn open(&self, task: &TaskId, start: &str, paths: &[String]) -> Result<Workspace> {
        let workspace = worktree::create(&self.repo, task, self.provider, start)?;
        narrow(&workspace.path, paths)?;
        Ok(workspace)
    }

    fn close(&self, task: &TaskId, force: bool) -> Result<()> {
        worktree::remove(&self.repo, task, self.provider, force)
    }
}

/// The patterns a task's slice holds: the repository's top-level files, then
/// each path the task may touch or must read, anchored at the root.
///
/// Empty when the task declares no paths at all: an empty slice is a task with
/// nothing to work on, so such a task works in the whole tree.
pub fn patterns(task: &Task) -> Vec<String> {
    let declared: Vec<&String> = task
        .allowed_paths
        .iter()
        .chain(task.reads.iter())
        .filter(|path| !path.trim().is_empty())
        .collect();
    if declared.is_empty() {
        return Vec::new();
    }
    let mut out = vec!["/*".to_string(), "!/*/".to_string()];
    for path in declared {
        let path = path.trim().trim_start_matches("./").trim_start_matches('/');
        let anchored = format!("/{path}");
        if !out.contains(&anchored) {
            out.push(anchored);
        }
    }
    out
}

fn sparse(workdir: &Path, args: &[&str]) -> Result<()> {
    let output = shell::program("git")
        .current_dir(workdir)
        .arg("sparse-checkout")
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Refused(format!(
            "git sparse-checkout {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// Narrow `workdir` to `patterns`. No patterns is the whole tree.
pub fn narrow(workdir: &Path, patterns: &[String]) -> Result<()> {
    if patterns.is_empty() {
        return widen(workdir);
    }
    let mut args = vec!["set", "--no-cone"];
    args.extend(patterns.iter().map(String::as_str));
    sparse(workdir, &args)
}

/// The whole tree again, for the checks. Harmless on a worktree that was
/// never narrowed.
pub fn widen(workdir: &Path) -> Result<()> {
    if !is_sparse(workdir) {
        return Ok(());
    }
    sparse(workdir, &["disable"])
}

/// Whether `workdir` is narrowed now.
pub fn is_sparse(workdir: &Path) -> bool {
    shell::program("git")
        .current_dir(workdir)
        .args(["config", "--get", "core.sparseCheckout"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "true")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;

    fn task(paths: &[&str], reads: &[&str]) -> Task {
        let mut task = Task::new(TaskId::new("TASK-0007"), "t", 0);
        task.allowed_paths = paths.iter().map(|p| p.to_string()).collect();
        task.reads = reads.iter().map(|p| p.to_string()).collect();
        task
    }

    #[test]
    fn a_slice_is_the_top_level_files_and_the_declared_paths() {
        assert_eq!(
            patterns(&task(
                &["src/auth/**", "./docs/"],
                &["/README.md", "src/auth/**"]
            )),
            vec!["/*", "!/*/", "/src/auth/**", "/docs/", "/README.md"]
        );
    }

    #[test]
    fn a_task_with_no_paths_works_in_the_whole_tree() {
        assert!(patterns(&task(&[], &["  "])).is_empty());
    }

    fn repository() -> Fixture {
        let fixture = Fixture::woven();
        fixture.write("docs/guide.md", "guide\n");
        fixture.write("web/app.ts", "app\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("More folders");
        fixture
    }

    #[test]
    fn a_slice_holds_its_paths_and_the_top_level_files_and_not_the_rest() {
        let fixture = repository();
        let provider = WorktreeSlice {
            repo: fixture.path().to_path_buf(),
            provider: AgentProvider::Codex,
        };
        let the_task = task(&["docs/**"], &[]);
        let workspace = provider
            .open(&the_task.id, "main", &patterns(&the_task))
            .unwrap();

        assert!(workspace.path.join("docs/guide.md").exists());
        assert!(
            workspace.path.join("core.txt").exists(),
            "top-level files stay"
        );
        assert!(
            !workspace.path.join("web/app.ts").exists(),
            "the rest is not checked out"
        );
        assert!(!workspace.path.join("src/deep/nested/main.rs").exists());
        assert!(is_sparse(&workspace.path));
        // The person's own checkout is whole.
        assert!(fixture.path().join("web/app.ts").exists());

        provider.close(&the_task.id, true).unwrap();
    }

    #[test]
    fn widening_gives_the_checks_the_whole_tree_and_narrowing_puts_the_slice_back() {
        let fixture = repository();
        let provider = WorktreeSlice {
            repo: fixture.path().to_path_buf(),
            provider: AgentProvider::Codex,
        };
        let the_task = task(&["docs/**"], &[]);
        let slice = patterns(&the_task);
        let workspace = provider.open(&the_task.id, "main", &slice).unwrap();
        // The agent's work in progress, which neither switch may lose.
        std::fs::write(workspace.path.join("docs/guide.md"), "edited\n").unwrap();

        widen(&workspace.path).unwrap();
        assert!(workspace.path.join("web/app.ts").exists());
        assert!(!is_sparse(&workspace.path));

        narrow(&workspace.path, &slice).unwrap();
        assert!(!workspace.path.join("web/app.ts").exists());
        assert_eq!(
            std::fs::read_to_string(workspace.path.join("docs/guide.md")).unwrap(),
            "edited\n"
        );

        provider.close(&the_task.id, true).unwrap();
    }

    #[test]
    fn widening_a_worktree_that_was_never_narrowed_changes_nothing() {
        let fixture = repository();
        assert!(widen(fixture.path()).is_ok());
        assert!(fixture.path().join("web/app.ts").exists());
    }
}

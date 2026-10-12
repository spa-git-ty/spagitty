// SPDX-License-Identifier: GPL-3.0-or-later

//! Naming the branch and the directory a task works in.
//!
//! Both names are derived, never chosen, and both are derived *here* — because
//! the branch name is how the Git graph, the worktree list and the Farm screen
//! find each other, and three functions that each build it from the same three
//! pieces would drift on the first rename.
//!
//! The shape is the plan's:
//!
//! ```text
//! spagitty-farm/TASK-0042/codex
//! ```
//!
//! A namespace nobody else uses, then the task, then who did the work. It sorts
//! usefully in `git branch`, it is obvious in the graph, and `git branch -d
//! spagitty-farm/...` with a glob cleans up a farm without touching anything a
//! person made.

use crate::model::{AgentProvider, TaskId};

/// The ref namespace every farm branch lives under.
pub const NAMESPACE: &str = "spagitty-farm";

/// The branch for one task, worked by one provider.
pub fn branch_name(task: &TaskId, provider: AgentProvider) -> String {
    format!(
        "{NAMESPACE}/{}/{}",
        sanitise(task.as_str()),
        provider.slug()
    )
}

/// The worktree directory name for one task.
///
/// Lower-case, because the farm's worktrees sit next to each other in one
/// directory and a case-insensitive filesystem would collapse `TASK-0001` and
/// `task-0001` into the same place. The repository has been bitten by exactly
/// this before, in a different form (BUG-010).
pub fn worktree_dir(task: &TaskId) -> String {
    sanitise(task.as_str()).to_lowercase()
}

/// True for a branch this farm made.
///
/// Used by cleanup and by the graph's decoration, and it is a prefix test
/// rather than a parse so that a branch from an older naming scheme is still
/// recognised as ours and still cleaned up.
pub fn is_farm_branch(name: &str) -> bool {
    name.starts_with(&format!("{NAMESPACE}/"))
        || name.starts_with(&format!("refs/heads/{NAMESPACE}/"))
}

/// The task a farm branch belongs to, if it is one.
///
/// The Git graph uses this to turn a branch chip into a link to the task, which
/// is the integration the plan asks for in §24.
pub fn task_of(branch: &str) -> Option<TaskId> {
    let rest = branch
        .strip_prefix("refs/heads/")
        .unwrap_or(branch)
        .strip_prefix(NAMESPACE)?
        .strip_prefix('/')?;
    let (task, _) = rest.split_once('/')?;
    (!task.is_empty()).then(|| TaskId::new(task))
}

/// The provider that worked a farm branch, if it is one.
pub fn provider_of(branch: &str) -> Option<AgentProvider> {
    let rest = branch
        .strip_prefix("refs/heads/")
        .unwrap_or(branch)
        .strip_prefix(NAMESPACE)?
        .strip_prefix('/')?;
    let (_, slug) = rest.split_once('/')?;
    [
        AgentProvider::ClaudeCode,
        AgentProvider::Codex,
        AgentProvider::Cursor,
        AgentProvider::OhMyPi,
        AgentProvider::Custom,
    ]
    .into_iter()
    .find(|provider| provider.slug() == slug)
}

/// Everything git refuses in a ref component, replaced with a hyphen.
///
/// Task identifiers are generated and would always be safe — but a farm can be
/// loaded from a hand-edited file, and a task whose identifier contains a space
/// would otherwise produce a `git branch` invocation that fails with git's own
/// message about ref names, three layers below where the problem is.
fn sanitise(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        let safe = character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.');
        out.push(if safe { character } else { '-' });
    }
    // git forbids `..` anywhere in a ref, a leading or trailing dot, and a
    // component ending in `.lock`. A dot on its own is legal — `v1.2.3` is a
    // ref — so they are collapsed rather than banned.
    while out.contains("..") {
        out = out.replace("..", ".");
    }
    let trimmed = out.trim_matches('.').trim_end_matches("-lock").to_string();
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branch_reads_as_the_plan_writes_it() {
        assert_eq!(
            branch_name(&TaskId::new("TASK-0042"), AgentProvider::Codex),
            "spagitty-farm/TASK-0042/codex"
        );
    }

    #[test]
    fn a_farm_branch_is_recognised_with_or_without_its_ref_prefix() {
        assert!(is_farm_branch("spagitty-farm/TASK-0001/claude"));
        assert!(is_farm_branch("refs/heads/spagitty-farm/TASK-0001/claude"));
        assert!(!is_farm_branch("feature/FEAT-073-agent-farm"));
        assert!(!is_farm_branch("main"));
    }

    #[test]
    fn a_branch_points_back_at_its_task_and_its_agent() {
        let branch = branch_name(&TaskId::new("TASK-0007"), AgentProvider::Cursor);
        assert_eq!(task_of(&branch), Some(TaskId::new("TASK-0007")));
        assert_eq!(provider_of(&branch), Some(AgentProvider::Cursor));
        assert_eq!(
            task_of("refs/heads/spagitty-farm/TASK-0007/cursor"),
            Some(TaskId::new("TASK-0007"))
        );
    }

    #[test]
    fn an_ordinary_branch_points_at_no_task() {
        assert_eq!(task_of("main"), None);
        assert_eq!(task_of("spagitty-farm/"), None);
        assert_eq!(task_of("spagitty-farm/TASK-0001"), None);
        assert_eq!(provider_of("main"), None);
    }

    #[test]
    fn an_unknown_provider_slug_is_not_invented() {
        assert_eq!(provider_of("spagitty-farm/TASK-0001/gemini"), None);
    }

    #[test]
    fn an_identifier_that_would_break_git_is_made_safe() {
        let branch = branch_name(&TaskId::new("TASK 1/../x"), AgentProvider::ClaudeCode);
        assert_eq!(branch, "spagitty-farm/TASK-1-.-x/claude");
        assert!(!branch.contains(' '));
        assert!(!branch.contains(".."));
    }

    #[test]
    fn an_identifier_with_nothing_usable_still_produces_a_ref() {
        assert_eq!(
            branch_name(&TaskId::new("..."), AgentProvider::ClaudeCode),
            "spagitty-farm/task/claude"
        );
    }

    #[test]
    fn worktree_directories_do_not_collide_on_a_case_insensitive_disk() {
        assert_eq!(worktree_dir(&TaskId::new("TASK-0042")), "task-0042");
        assert_eq!(
            worktree_dir(&TaskId::new("task-0042")),
            worktree_dir(&TaskId::new("TASK-0042"))
        );
    }
}

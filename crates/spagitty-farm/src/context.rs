// SPDX-License-Identifier: GPL-3.0-or-later

//! What an agent is told.
//!
//! # The rule this module exists to enforce
//!
//! *Do not dump the entire repository into every prompt.* It is expensive, it
//! is slow, and it is counter-productive: an agent given a hundred files reads
//! a hundred files, and the one it needed is diluted. The agents Spagitty
//! drives are all capable of reading the repository themselves — they are
//! standing in a checkout of it — so the prompt's job is not to *contain* the
//! context but to say precisely what the task is and where its edges are.
//!
//! So a prompt is short and structured: the goal, the task, what "done" means,
//! which files may be touched, the repository's own rules, what the tasks this
//! one depends on concluded, how it will be verified, and the shape of the
//! answer. Nothing else.
//!
//! # Why every section is always present
//!
//! Even when empty, with an explicit "none". A section that disappears when it
//! has no content teaches the agent nothing; a section that says "Allowed
//! files: the whole repository" tells it something true and important.

use crate::model::{Farm, Goal, GoalSource, Handoff, Task, WorkspaceMode};
use crate::policy::Policy;
use crate::review::Review;

/// What one dependency concluded, for the prompt of whatever comes next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyResult {
    pub task: String,
    pub title: String,
    pub summary: String,
    pub files_changed: Vec<String>,
}

/// Everything needed to write a prompt.
pub struct Context<'a> {
    pub farm: &'a Farm,
    pub task: &'a Task,
    pub policy: &'a Policy,
    pub dependencies: Vec<DependencyResult>,
    pub verification: Vec<String>,
    /// Set when this run is a second attempt after a review.
    pub changes_requested: Option<String>,
}

/// The prompt for an implementation run.
pub fn implementation(context: &Context<'_>) -> String {
    let mut out = String::new();
    goal_section(&mut out, &context.farm.goal);
    task_section(&mut out, context.task);
    criteria_section(&mut out, context.task);
    files_section(&mut out, context.task);
    rules_section(&mut out, context.policy);
    dependencies_section(&mut out, &context.dependencies);
    verification_section(&mut out, &context.verification);

    if let Some(requested) = &context.changes_requested {
        section(
            &mut out,
            "CHANGES REQUESTED",
            &format!(
                "A reviewer asked for the following before this task can be accepted. \
                 Address all of it.\n\n{}",
                requested.trim()
            ),
        );
    }

    slice_section(&mut out, context.farm, context.task);

    section(
        &mut out,
        "WORKING AGREEMENT",
        "You are working in a git worktree of your own, on a branch of your own. \
         Commit your work when you are finished — uncommitted changes are not \
         reviewed and not merged. Do not switch branches, do not merge, do not \
         push, and do not touch files outside the allowed list.",
    );

    section(&mut out, "EXPECTED HANDOFF", &Handoff::contract());
    out.trim_end().to_string()
}

/// The prompt for a review run.
///
/// The reviewer is told the acceptance criteria and *not* told that a
/// verification already passed. Saying so invites agreement — a model told the
/// tests are green will read the diff looking for reasons it is fine.
pub fn review(context: &Context<'_>, diff_summary: &str) -> String {
    let mut out = String::new();
    goal_section(&mut out, &context.farm.goal);
    task_section(&mut out, context.task);
    criteria_section(&mut out, context.task);
    rules_section(&mut out, context.policy);

    section(
        &mut out,
        "WHAT CHANGED",
        if diff_summary.trim().is_empty() {
            "Nothing was committed on this branch. That is itself worth reporting."
        } else {
            diff_summary.trim()
        },
    );

    section(
        &mut out,
        "YOUR JOB",
        "Review this change against the acceptance criteria and the repository's \
         rules. You did not write it. Read the diff on the current branch with \
         git, and read the files it touches. Do not change anything: this is a \
         review, and any edit you make will be discarded.",
    );

    section(&mut out, "EXPECTED VERDICT", &Review::contract());
    out.trim_end().to_string()
}

/// The prompt for a planning run.
///
/// `map` is the repository's layout, from [`repository_map`]: the planner
/// reads the repository itself, and the map tells it where to start (FEAT-141).
pub fn planning(farm: &Farm, policy: &Policy, map: &str) -> String {
    let mut out = String::new();
    goal_section(&mut out, &farm.goal);
    sources_section(&mut out, &farm.goal.sources);
    rules_section(&mut out, policy);
    section(
        &mut out,
        "REPOSITORY MAP",
        if map.trim().is_empty() {
            "No files are tracked yet."
        } else {
            map.trim_end()
        },
    );
    section(
        &mut out,
        "YOUR JOB",
        "Read enough of this repository to understand how the goal would be built, \
         then break it into tasks that can be worked independently. Each task will \
         be given to one agent, in a worktree of its own. Tasks whose allowed paths \
         do not overlap can run at the same time, so keep them apart where you can. \
         Do not change any files.",
    );
    section(
        &mut out,
        "VERIFICATION",
        if farm.verification.is_empty() {
            "This farm has no verification commands configured.".to_string()
        } else {
            format!(
                "Every task will be checked with:\n{}",
                bullets(&farm.verification)
            )
        }
        .as_str(),
    );
    section(
        &mut out,
        "EXPECTED PLAN",
        &crate::orchestrator::planner::Plan::contract(),
    );
    out.trim_end().to_string()
}

/// The prompt for breaking one task into smaller ones (FEAT-076).
///
/// The same shape as [`planning`] with the task in front of the goal, because
/// that is what the agent is being asked about: the goal is context for it, not
/// the subject. It keeps the goal all the same — a subtask that makes sense
/// against its parent and not against the goal is a subtask of the wrong thing.
pub fn decomposition(farm: &Farm, task: &Task, policy: &Policy) -> String {
    let mut out = String::new();
    goal_section(&mut out, &farm.goal);
    task_section(&mut out, task);
    criteria_section(&mut out, task);
    files_section(&mut out, task);
    rules_section(&mut out, policy);
    section(
        &mut out,
        "YOUR JOB",
        "This task is too large to be done in one go. Read enough of this \
         repository to understand what it involves, then break *this task* — not \
         the goal — into smaller tasks that can be worked independently. Each one \
         will be given to one agent, in a worktree of its own, so keep their \
         allowed paths apart where the work allows it. Do not restate the task \
         itself as one of them, and do not change any files.",
    );
    section(
        &mut out,
        "EXPECTED PLAN",
        &crate::orchestrator::planner::Plan::subtask_contract(),
    );
    out.trim_end().to_string()
}

/// Where the agent is standing, when the farm works in slices (FEAT-143):
/// which paths are checked out, and that the rest is in the repository, only
/// not here. Absent for a whole worktree, which needs no explaining.
fn slice_section(out: &mut String, farm: &Farm, task: &Task) {
    if farm.workspace == WorkspaceMode::Copy {
        copy_section(out);
        return;
    }
    if farm.workspace != WorkspaceMode::Slice {
        return;
    }
    let mut shown: Vec<String> = task
        .allowed_paths
        .iter()
        .chain(task.reads.iter())
        .filter(|path| !path.trim().is_empty())
        .cloned()
        .collect();
    shown.dedup();
    if shown.is_empty() {
        return;
    }
    section(
        out,
        "YOUR SLICE",
        &format!(
            "This worktree is a slice: only the repository's top-level files and these \
             paths are checked out.\n{}\n\nEverything else is still in the repository, \
             only not here. Read it with `git show HEAD:<path>` if you need to; do not \
             create files outside these paths. The checks run on the whole tree.",
            bullets(&shown)
        ),
    );
}

/// Where the agent is standing, when the farm gives each task a copy
/// (FEAT-147): a repository of its own with no remote, so commit and stop.
fn copy_section(out: &mut String) {
    section(
        out,
        "YOUR COPY",
        "This directory is a copy of the repository made for this task. It has no \
         remote and only your branch. Commit your work on this branch and stop: \
         Spagitty takes your commits from here when you finish, checks that they \
         change only the paths you may touch, and runs the checks. Do not push, \
         and do not add remotes. Uncommitted changes are not taken.",
    );
}

/// What an agent is told when the history tools are attached to its run
/// (FEAT-145).
pub fn history_note() -> String {
    let mut out = String::new();
    section(
        &mut out,
        "HISTORY",
        "The repository's history is a set of tools on the `spagitty` server: \
         `blame` (who changed these lines, and why), `history` (the commits that \
         changed a file), `search` (commit messages, changes or code at a commit) \
         and `show` (a file as a commit has it). Ask them rather than running \
         `git log` or `git blame` yourself: the answers are shorter, and cached.",
    );
    out.trim_end().to_string()
}

fn goal_section(out: &mut String, goal: &Goal) {
    section(out, "GOAL", &goal.brief());
}

/// What the person picked to start from (FEAT-141): each issue, each brief and
/// the branch in progress, in full. Absent when nothing was picked, because
/// here "none" would only repeat the goal.
fn sources_section(out: &mut String, sources: &[GoalSource]) {
    if sources.is_empty() {
        return;
    }
    let mut text = String::from(
        "The person picked these to start from. Plan from them as well as from the goal.",
    );
    for source in sources {
        text.push_str("\n\n");
        match source {
            GoalSource::Issue {
                number,
                title,
                body,
                labels,
                url,
            } => {
                text.push_str(&format!("## Issue #{number}: {title}"));
                if !labels.is_empty() {
                    text.push_str(&format!("\nLabels: {}", labels.join(", ")));
                }
                if !url.is_empty() {
                    text.push_str(&format!("\n{url}"));
                }
                if !body.trim().is_empty() {
                    text.push_str("\n\n");
                    text.push_str(body.trim());
                }
            }
            GoalSource::Brief { path, text: brief } => {
                text.push_str(&format!("## Brief: {path}\n\n{}", brief.trim()));
            }
            GoalSource::Branch { name } => {
                text.push_str(&format!(
                    "## Branch in progress: {name}\n\nThis work has started on {name}. \
                     Read what it already has (its commits since it left the default \
                     branch), then plan what finishes it. Every task starts from {name} \
                     and is merged back into it."
                ));
            }
        }
    }
    section(out, "STARTING POINTS", &text);
}

/// How many lines the repository map may take.
const MAP_LINES: usize = 80;

/// The repository's layout in a few dozen lines: every top-level file, and
/// every folder to two levels with how many tracked files it holds.
///
/// From the tracked paths alone, so it is the same on every machine and says
/// nothing about build output.
pub fn repository_map(paths: &[String]) -> String {
    use std::collections::BTreeMap;
    let mut files: Vec<&str> = Vec::new();
    let mut folders: BTreeMap<String, usize> = BTreeMap::new();
    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        match parts.len() {
            0 | 1 => files.push(path),
            2 => *folders.entry(format!("{}/", parts[0])).or_default() += 1,
            _ => {
                *folders.entry(format!("{}/", parts[0])).or_default() += 1;
                *folders
                    .entry(format!("{}/{}/", parts[0], parts[1]))
                    .or_default() += 1;
            }
        }
    }
    let mut lines: Vec<String> = files.iter().map(|f| f.to_string()).collect();
    for (folder, count) in &folders {
        let indent = if folder.matches('/').count() > 1 {
            "  "
        } else {
            ""
        };
        let noun = if *count == 1 { "file" } else { "files" };
        lines.push(format!("{indent}{folder} ({count} {noun})"));
    }
    if lines.len() > MAP_LINES {
        let rest = lines.len() - MAP_LINES;
        lines.truncate(MAP_LINES);
        lines.push(format!("… and {rest} more"));
    }
    lines.join("\n")
}

fn task_section(out: &mut String, task: &Task) {
    let mut text = format!("{} — {}", task.id, task.title);
    if !task.description.trim().is_empty() {
        text.push_str("\n\n");
        text.push_str(task.description.trim());
    }
    section(out, "CURRENT TASK", &text);
}

fn criteria_section(out: &mut String, task: &Task) {
    section(
        out,
        "ACCEPTANCE CRITERIA",
        if task.acceptance_criteria.is_empty() {
            "None were written. Use your judgement, and say in your handoff what \
             you took \"done\" to mean."
                .to_string()
        } else {
            bullets(&task.acceptance_criteria)
        }
        .as_str(),
    );
}

fn files_section(out: &mut String, task: &Task) {
    section(
        out,
        "ALLOWED FILES",
        if task.allowed_paths.is_empty() {
            "Anywhere in the repository. Keep the change as small as it can be.".to_string()
        } else {
            format!(
                "Only these paths. Editing anything else will be rejected at review.\n{}",
                bullets(&task.allowed_paths)
            )
        }
        .as_str(),
    );
}

fn rules_section(out: &mut String, policy: &Policy) {
    section(
        out,
        "REPOSITORY RULES",
        if policy.is_empty() {
            "This repository has no agent rules file. Follow the conventions you \
             find in the code."
        } else {
            policy.text.trim()
        },
    );
}

fn dependencies_section(out: &mut String, dependencies: &[DependencyResult]) {
    if dependencies.is_empty() {
        return;
    }
    let mut text = String::new();
    for dependency in dependencies {
        text.push_str(&format!(
            "### {} — {}\n\n",
            dependency.task, dependency.title
        ));
        if !dependency.summary.trim().is_empty() {
            text.push_str(dependency.summary.trim());
            text.push('\n');
        }
        if !dependency.files_changed.is_empty() {
            text.push_str("\nFiles it changed:\n");
            text.push_str(&bullets(&dependency.files_changed));
            text.push('\n');
        }
        text.push('\n');
    }
    section(out, "WHAT CAME BEFORE", text.trim_end());
}

fn verification_section(out: &mut String, commands: &[String]) {
    section(
        out,
        "VERIFICATION",
        if commands.is_empty() {
            "No commands are configured. Run whatever checks this repository has, \
             and report them in your handoff."
                .to_string()
        } else {
            format!(
                "When you are finished, Spagitty runs these against your worktree. \
                 The task is not accepted until they pass, so run them yourself first.\n{}",
                bullets(commands)
            )
        }
        .as_str(),
    );
}

fn section(out: &mut String, heading: &str, body: &str) {
    out.push_str("# ");
    out.push_str(heading);
    out.push_str("\n\n");
    out.push_str(body);
    out.push_str("\n\n");
}

fn bullets(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_id, FarmId, GoalId, TaskId};
    use crate::policy::Policy;

    fn farm() -> Farm {
        let mut farm = Farm::new(
            FarmId::new("f1"),
            "/repo",
            Goal::new(GoalId::new("g1"), "Add GitHub OAuth login", 0),
            0,
        );
        farm.verification = vec!["cargo test".into()];
        farm
    }

    fn task() -> Task {
        let mut task = Task::new(task_id(42), "Implement refresh-token rotation", 0);
        task.description = "Add single-use refresh-token rotation.".into();
        task.acceptance_criteria = vec!["Refresh tokens are single-use".into()];
        task.allowed_paths = vec!["backend/src/auth/**".into()];
        task
    }

    fn context<'a>(farm: &'a Farm, task: &'a Task, policy: &'a Policy) -> Context<'a> {
        Context {
            farm,
            task,
            policy,
            dependencies: Vec::new(),
            verification: vec!["cargo test".into()],
            changes_requested: None,
        }
    }

    #[test]
    fn every_section_the_plan_names_is_present() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = implementation(&context(&farm, &task, &policy));
        for heading in [
            "# GOAL",
            "# CURRENT TASK",
            "# ACCEPTANCE CRITERIA",
            "# ALLOWED FILES",
            "# REPOSITORY RULES",
            "# VERIFICATION",
            "# EXPECTED HANDOFF",
        ] {
            assert!(prompt.contains(heading), "{heading} is missing");
        }
    }

    #[test]
    fn the_prompt_does_not_contain_the_repository() {
        // The whole point: a prompt is a brief, not a copy of the codebase.
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.len() < 4_000, "the prompt is {} bytes", prompt.len());
    }

    #[test]
    fn a_task_with_no_criteria_is_told_so_rather_than_left_a_gap() {
        let (farm, policy) = (farm(), Policy::default());
        let mut task = task();
        task.acceptance_criteria.clear();
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.contains("# ACCEPTANCE CRITERIA"));
        assert!(prompt.contains("None were written"));
    }

    #[test]
    fn a_task_with_no_path_limit_is_told_the_limit_is_the_repository() {
        let (farm, policy) = (farm(), Policy::default());
        let mut task = task();
        task.allowed_paths.clear();
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.contains("Anywhere in the repository"));
    }

    #[test]
    fn declared_paths_reach_the_agent_verbatim() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.contains("- backend/src/auth/**"));
    }

    #[test]
    fn the_repositorys_rules_are_included_when_it_has_some() {
        let (farm, task) = (farm(), task());
        let policy = Policy {
            sources: Vec::new(),
            text: "## From AGENTS.md\n\nNo new dependencies.".into(),
        };
        assert!(implementation(&context(&farm, &task, &policy)).contains("No new dependencies."));
    }

    #[test]
    fn dependency_results_are_only_included_when_there_are_some() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        assert!(!implementation(&context(&farm, &task, &policy)).contains("WHAT CAME BEFORE"));

        let mut with = context(&farm, &task, &policy);
        with.dependencies = vec![DependencyResult {
            task: "TASK-0037".into(),
            title: "Add the token store".into(),
            summary: "Added TokenStore in backend/src/auth/store.rs.".into(),
            files_changed: vec!["backend/src/auth/store.rs".into()],
        }];
        let prompt = implementation(&with);
        assert!(prompt.contains("### TASK-0037 — Add the token store"));
        assert!(prompt.contains("Added TokenStore"));
        assert!(prompt.contains("- backend/src/auth/store.rs"));
    }

    #[test]
    fn a_second_attempt_carries_what_the_reviewer_asked_for() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let mut retry = context(&farm, &task, &policy);
        retry.changes_requested = Some("- src/auth.rs: the token is not invalidated".into());
        let prompt = implementation(&retry);
        assert!(prompt.contains("# CHANGES REQUESTED"));
        assert!(prompt.contains("the token is not invalidated"));
    }

    #[test]
    fn a_first_attempt_carries_no_change_request() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        assert!(!implementation(&context(&farm, &task, &policy)).contains("CHANGES REQUESTED"));
    }

    #[test]
    fn the_agent_is_told_to_commit_and_not_to_merge() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.contains("Commit your work"));
        assert!(prompt.contains("do not merge"));
        assert!(prompt.contains("do not push"));
    }

    #[test]
    fn a_reviewer_is_not_told_the_tests_already_passed() {
        // Telling it invites agreement rather than review.
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = review(&context(&farm, &task, &policy), "3 files changed");
        assert!(!prompt.to_lowercase().contains("verification passed"));
        assert!(prompt.contains("You did not write it"));
        assert!(prompt.contains("# EXPECTED VERDICT"));
    }

    #[test]
    fn a_review_of_an_empty_branch_says_so() {
        let (farm, task, policy) = (farm(), task(), Policy::default());
        let prompt = review(&context(&farm, &task, &policy), "   ");
        assert!(prompt.contains("Nothing was committed"));
    }

    #[test]
    fn a_planning_prompt_asks_for_disjoint_paths() {
        let prompt = planning(&farm(), &Policy::default(), "src/ (3 files)");
        assert!(prompt.contains("# GOAL"));
        assert!(prompt.contains("# REPOSITORY MAP\n\nsrc/ (3 files)"));
        // Nothing was picked to start from, so the section is not there.
        assert!(!prompt.contains("STARTING POINTS"));
        assert!(prompt.contains("do not overlap"));
        assert!(prompt.contains("Do not change any files"));
        assert!(prompt.contains("cargo test"));
    }

    #[test]
    fn a_planning_prompt_for_an_unverified_farm_says_so() {
        let mut farm = farm();
        farm.verification.clear();
        assert!(planning(&farm, &Policy::default(), "").contains("no verification commands"));
    }

    #[test]
    fn a_planning_prompt_carries_every_starting_point_in_full() {
        let mut farm = farm();
        farm.goal.sources = vec![
            GoalSource::Issue {
                number: 142,
                title: "Gitea: list and open pull requests".into(),
                body: "Same Host trait as GitHub.".into(),
                labels: vec!["feature".into()],
                url: "https://host/142".into(),
            },
            GoalSource::Brief {
                path: "docs/specs/gitea.md".into(),
                text: "# Gitea\n\nUse the v1 API.".into(),
            },
            GoalSource::Branch {
                name: "feature/gitea".into(),
            },
        ];
        let prompt = planning(&farm, &Policy::default(), "");
        assert!(prompt.contains("# STARTING POINTS"));
        assert!(
            prompt.contains("## Issue #142: Gitea: list and open pull requests\nLabels: feature")
        );
        assert!(prompt.contains("Same Host trait as GitHub."));
        assert!(prompt.contains("## Brief: docs/specs/gitea.md\n\n# Gitea\n\nUse the v1 API."));
        assert!(prompt.contains("Every task starts from feature/gitea"));
        assert!(prompt.contains("No files are tracked yet."));
    }

    #[test]
    fn a_slice_farm_tells_the_agent_what_is_checked_out() {
        let (mut farm, mut task, policy) = (farm(), task(), Policy::default());
        task.reads = vec!["backend/src/session/**".into()];
        let whole = implementation(&context(&farm, &task, &policy));
        assert!(!whole.contains("YOUR SLICE"));

        farm.workspace = WorkspaceMode::Slice;
        let sliced = implementation(&context(&farm, &task, &policy));
        assert!(sliced.contains("# YOUR SLICE"));
        assert!(sliced.contains("- backend/src/auth/**\n- backend/src/session/**"));
        assert!(sliced.contains("The checks run on the whole tree."));
    }

    #[test]
    fn the_history_note_names_the_four_tools() {
        let note = history_note();
        assert!(note.starts_with("# HISTORY"));
        for tool in ["`blame`", "`history`", "`search`", "`show`"] {
            assert!(note.contains(tool), "{tool}");
        }
    }

    #[test]
    fn a_copy_farm_tells_the_agent_to_commit_and_not_push() {
        let (mut farm, task, policy) = (farm(), task(), Policy::default());
        farm.workspace = WorkspaceMode::Copy;
        let prompt = implementation(&context(&farm, &task, &policy));
        assert!(prompt.contains("# YOUR COPY"));
        assert!(prompt.contains("Do not push"));
        assert!(!prompt.contains("YOUR SLICE"));
    }

    #[test]
    fn the_map_lists_top_level_files_and_two_levels_of_folders() {
        let paths: Vec<String> = [
            "Cargo.toml",
            "README.md",
            "src/lib.rs",
            "src/home/needs.ts",
            "src/home/shape.ts",
            "src/home/deep/x.ts",
        ]
        .iter()
        .map(|p| p.to_string())
        .collect();
        assert_eq!(
            repository_map(&paths),
            "Cargo.toml\nREADME.md\nsrc/ (4 files)\n  src/home/ (3 files)"
        );
    }

    #[test]
    fn a_long_map_is_cut_and_says_so() {
        let paths: Vec<String> = (0..200).map(|i| format!("f{i}.txt")).collect();
        let map = repository_map(&paths);
        assert_eq!(map.lines().count(), MAP_LINES + 1);
        assert!(map.ends_with("… and 120 more"));
    }

    #[test]
    fn the_task_identifier_is_in_the_prompt() {
        // The agent puts it in its commit message; the graph reads it back.
        let (farm, task, policy) = (farm(), task(), Policy::default());
        assert!(implementation(&context(&farm, &task, &policy))
            .contains(TaskId::new("TASK-0042").as_str()));
    }
}

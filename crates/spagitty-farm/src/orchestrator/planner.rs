// SPDX-License-Identifier: GPL-3.0-or-later

//! Turning a sentence into a plan.
//!
//! The user types "Add GitHub OAuth login" and gets a list of tasks. The plan
//! puts this at phase 7, after everything else works, and the ordering is the
//! point: a planner is only useful once there is a farm that can execute what
//! it plans, and a farm that plans badly but executes well is fixable by hand.
//!
//! # The agent proposes; Spagitty owns the graph
//!
//! A planning agent returns a *proposal* — titles, kinds, dependencies — and
//! nothing in it is trusted. [`adopt`] is what turns a proposal into tasks, and
//! it does four things the agent cannot be relied on to do:
//!
//! 1. Assigns real identifiers. The agent's `depends_on` refers to its own
//!    numbering, which is remapped rather than believed.
//! 2. Drops references to tasks that do not exist.
//! 3. Refuses a plan with a cycle in it, rather than importing one.
//! 4. Caps the length, because a model asked to decompose a goal will sometimes
//!    return sixty tasks and none of them are real.
//!
//! The user still approves the result before anything runs. That is a farm
//! setting, not a property of this module.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{now_ms, AgentId, Farm, Task, TaskId, TaskKind, TaskOrigin, TaskPriority};
use crate::orchestrator::dependency;

/// The fence a planning agent is asked to use.
pub const FENCE: &str = "spagitty-plan";

/// How many tasks a plan may contain.
///
/// Twenty-four. It was twelve, which was the right number when twelve was all a
/// plan could ever be: a goal larger than that had nowhere to go, because a
/// task could not be broken down (FEAT-076). Now that it can, the top-level
/// plan is allowed to be a real decomposition of a real piece of work, and the
/// thing that keeps a farm supervisable is `max_parallel` — how many run at
/// once — rather than how many exist.
pub const MAX_TASKS: usize = 24;

/// How many tasks one task may be broken into.
///
/// Eight. A task that needs more than eight pieces was not a task, and the
/// answer is to break the *goal* up differently rather than to grow a tree
/// nobody can hold in their head. Smaller than [`MAX_TASKS`] deliberately: this
/// is a subdivision, not a second plan.
pub const MAX_SUBTASKS: usize = 8;

/// One task as the planning agent proposed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedTask {
    /// The agent's own reference for this task, used only by `dependsOn`
    /// within the same plan.
    #[serde(default)]
    pub reference: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: TaskKind,
    #[serde(default)]
    pub priority: TaskPriority,
    /// References to other tasks *in this plan*.
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// What the task needs to read and may not change (FEAT-143).
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

/// A whole proposed plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    #[serde(default)]
    pub tasks: Vec<PlannedTask>,
    /// What the agent wants to know before the plan is trusted.
    #[serde(default)]
    pub questions: Vec<String>,
}

impl Plan {
    /// Find a plan in a transcript. Last block wins, as everywhere else.
    pub fn parse(transcript: &str) -> Plan {
        let opening = format!("```{FENCE}");
        let mut found: Option<Plan> = None;
        let mut rest = transcript;
        while let Some(start) = rest.find(&opening) {
            let after = &rest[start + opening.len()..];
            let Some(end) = after.find("```") else { break };
            if let Ok(plan) = serde_json::from_str::<Plan>(after[..end].trim()) {
                found = Some(plan);
            }
            rest = &after[end + 3..];
        }
        found.unwrap_or_default()
    }

    /// The instructions appended to a planning prompt.
    pub fn contract() -> String {
        Self::contract_for(MAX_TASKS, "the goal")
    }

    /// The same contract, for a run that is breaking one task down.
    pub fn subtask_contract() -> String {
        Self::contract_for(MAX_SUBTASKS, "the task above")
    }

    fn contract_for(limit: usize, subject: &str) -> String {
        format!(
            "Break {subject} into no more than {limit} tasks. End your reply with exactly one \
             block in this form:\n\
             \n\
             ```{FENCE}\n\
             {{\n  \
             \"tasks\": [\n    \
             {{\n      \
             \"reference\": \"t1\",\n      \
             \"title\": \"short imperative title\",\n      \
             \"description\": \"what the agent doing this needs to know\",\n      \
             \"kind\": \"architecture\" | \"backend\" | \"frontend\" | \"testing\" | \"documentation\" | \"research\" | \"review\" | \"integration\" | \"general\",\n      \
             \"priority\": \"high\" | \"normal\" | \"low\",\n      \
             \"dependsOn\": [\"t1\"],\n      \
             \"allowedPaths\": [\"src/auth/**\"],\n      \
             \"reads\": [\"src/session/**\"],\n      \
             \"acceptanceCriteria\": [\"what must be true when this is done\"]\n    \
             }}\n  \
             ],\n  \
             \"questions\": [\"anything you could not decide\"]\n\
             }}\n\
             ```\n\
             \n\
             `allowedPaths` matters: two tasks whose paths overlap cannot run at the \
             same time, so keep them disjoint where the work allows it. `reads` lists \
             what a task needs to read but not change; it may overlap freely."
        )
    }
}

/// Turn a proposal into real tasks on a farm.
///
/// Does not modify the farm — it returns the tasks, and the caller decides
/// whether the user has approved them. Identifiers are allocated from the farm,
/// which is why it is taken mutably.
pub fn adopt(farm: &mut Farm, agent: &AgentId, plan: &Plan) -> Result<Vec<Task>> {
    adopt_under(farm, agent, plan, None)
}

/// Turn a proposal into real tasks, cut out of `parent`.
///
/// The children are `Draft` like any other proposal, so the plan-review band
/// asks before anything runs. `parent` is what makes the task they came from a
/// container: it stops being runnable and starts following them (FEAT-076).
pub fn adopt_under(
    farm: &mut Farm,
    agent: &AgentId,
    plan: &Plan,
    parent: Option<&TaskId>,
) -> Result<Vec<Task>> {
    let now = now_ms();
    let limit = if parent.is_some() {
        MAX_SUBTASKS
    } else {
        MAX_TASKS
    };
    let proposed: Vec<&PlannedTask> = plan
        .tasks
        .iter()
        .filter(|task| !task.title.trim().is_empty())
        .take(limit)
        .collect();

    // Identifiers first, so dependencies can be remapped in one pass.
    let ids: Vec<TaskId> = proposed.iter().map(|_| farm.allocate_task_id()).collect();

    let mut tasks = Vec::new();
    for (index, planned) in proposed.iter().enumerate() {
        let mut task = Task::new(ids[index].clone(), planned.title.trim(), now);
        task.parent = parent.cloned();
        // Who asked for it, so a plan's tasks never read as somebody's own
        // decisions (FEAT-078).
        task.origin = match parent {
            Some(parent) => TaskOrigin::Subtask {
                agent: agent.clone(),
                parent: parent.clone(),
            },
            None => TaskOrigin::Planned {
                agent: agent.clone(),
            },
        };
        task.description = planned.description.clone();
        task.kind = planned.kind;
        task.priority = planned.priority;
        task.allowed_paths = planned.allowed_paths.clone();
        task.reads = planned.reads.clone();
        task.acceptance_criteria = planned.acceptance_criteria.clone();
        task.depends_on = planned
            .depends_on
            .iter()
            // A reference to itself, or to a task the plan did not define, is
            // dropped. Both are things a model does, and neither is a reason to
            // reject a plan that is otherwise sound.
            .filter_map(|reference| {
                proposed
                    .iter()
                    .position(|candidate| candidate.reference == *reference)
                    .filter(|position| *position != index)
                    .map(|position| ids[position].clone())
            })
            .collect();
        task.depends_on.dedup();
        tasks.push(task);
    }

    // A cycle from a planner is refused whole rather than repaired: repairing
    // it would mean guessing which edge the model meant, and the plan is
    // cheaper to regenerate than to second-guess.
    dependency::validate(&tasks)?;
    Ok(tasks)
}

/// A proposal an agent made while doing something else.
///
/// The plan's phase 8: an agent that finds missing work says so in its handoff,
/// and Spagitty turns it into a task nobody has approved yet — `Draft`, so it
/// sits in the list without being scheduled.
pub fn from_proposal(
    farm: &mut Farm,
    proposal: &crate::model::ProposedTask,
    from: &TaskId,
    agent: Option<&AgentId>,
) -> Task {
    let id = farm.allocate_task_id();
    let mut task = Task::new(id, proposal.title.trim(), now_ms());
    task.description = proposal.description.clone();
    task.origin = TaskOrigin::Proposed {
        agent: agent.cloned(),
        from: from.clone(),
    };
    // Draft, not Ready. An agent may propose; only a person may schedule.
    task.status = crate::model::TaskStatus::Draft;
    task.depends_on = proposal
        .depends_on
        .iter()
        .filter(|id| farm.task(id).is_some())
        .cloned()
        .collect();
    task
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_id, FarmId, Goal, GoalId, ProposedTask, TaskStatus};

    /// The agent every test in this module plans with. Which one it is does
    /// not matter here; that it is *recorded* is FEAT-078's own concern.
    fn planner_agent() -> AgentId {
        AgentId::new("claude")
    }

    fn farm() -> Farm {
        Farm::new(
            FarmId::new("f1"),
            "/repo",
            Goal::new(GoalId::new("g1"), "Add OAuth", 0),
            0,
        )
    }

    fn plan(json: &str) -> Plan {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_transcript_with_no_plan_yields_no_tasks() {
        assert!(Plan::parse("I thought about it.").tasks.is_empty());
    }

    #[test]
    fn a_plan_is_found_in_a_transcript() {
        let transcript = format!(
            "Reading the repository...\n```{FENCE}\n{{\"tasks\":[{{\"reference\":\"t1\",\"title\":\"Investigate auth\"}}]}}\n```"
        );
        let parsed = Plan::parse(&transcript);
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].title, "Investigate auth");
    }

    #[test]
    fn adopting_gives_every_task_a_real_identifier() {
        let mut farm = farm();
        let tasks = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"reference":"t1","title":"A"},{"reference":"t2","title":"B"}]}"#),
        )
        .unwrap();
        assert_eq!(tasks[0].id, task_id(1));
        assert_eq!(tasks[1].id, task_id(2));
    }

    #[test]
    fn the_agents_own_references_are_remapped_to_identifiers() {
        let mut farm = farm();
        let tasks = adopt(
            &mut farm,
            &planner_agent(),
            &plan(
                r#"{"tasks":[
                    {"reference":"t1","title":"Investigate"},
                    {"reference":"t2","title":"Implement","dependsOn":["t1"]}
                ]}"#,
            ),
        )
        .unwrap();
        assert_eq!(tasks[1].depends_on, [task_id(1)]);
    }

    #[test]
    fn a_reference_the_plan_never_defined_is_dropped() {
        let mut farm = farm();
        let tasks = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"reference":"t1","title":"A","dependsOn":["t9"]}]}"#),
        )
        .unwrap();
        assert!(tasks[0].depends_on.is_empty());
    }

    #[test]
    fn a_task_that_depends_on_itself_has_the_edge_dropped() {
        // Rather than failing validation, because the intent is obvious.
        let mut farm = farm();
        let tasks = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"reference":"t1","title":"A","dependsOn":["t1"]}]}"#),
        )
        .unwrap();
        assert!(tasks[0].depends_on.is_empty());
    }

    #[test]
    fn a_cycle_between_two_tasks_is_refused() {
        let mut farm = farm();
        let error = adopt(
            &mut farm,
            &planner_agent(),
            &plan(
                r#"{"tasks":[
                    {"reference":"t1","title":"A","dependsOn":["t2"]},
                    {"reference":"t2","title":"B","dependsOn":["t1"]}
                ]}"#,
            ),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "dependencyCycle");
    }

    #[test]
    fn a_plan_longer_than_the_cap_is_cut() {
        let tasks: Vec<String> = (0..30)
            .map(|n| format!(r#"{{"reference":"t{n}","title":"Task {n}"}}"#))
            .collect();
        let mut farm = farm();
        let adopted = adopt(
            &mut farm,
            &planner_agent(),
            &plan(&format!(r#"{{"tasks":[{}]}}"#, tasks.join(","))),
        )
        .unwrap();
        assert_eq!(adopted.len(), MAX_TASKS);
    }

    #[test]
    fn a_task_with_no_title_is_not_adopted() {
        let mut farm = farm();
        let adopted = adopt(
            &mut farm,
            &planner_agent(),
            &plan(
                r#"{"tasks":[{"reference":"t1","title":"  "},{"reference":"t2","title":"Real"}]}"#,
            ),
        )
        .unwrap();
        assert_eq!(adopted.len(), 1);
        assert_eq!(adopted[0].title, "Real");
    }

    #[test]
    fn adopted_tasks_start_as_drafts_until_somebody_says_otherwise() {
        let mut farm = farm();
        let adopted = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"title":"A"}]}"#),
        )
        .unwrap();
        assert_eq!(adopted[0].status, TaskStatus::Draft);
    }

    #[test]
    fn everything_the_planner_said_is_carried_over() {
        let mut farm = farm();
        let adopted = adopt(
            &mut farm,
            &planner_agent(),
            &plan(
                r#"{"tasks":[{
                    "title":"Implement OAuth backend",
                    "description":"Use the existing session store.",
                    "kind":"backend",
                    "priority":"high",
                    "allowedPaths":["src/auth/**"],
                    "acceptanceCriteria":["Login still works"]
                }]}"#,
            ),
        )
        .unwrap();
        let task = &adopted[0];
        assert_eq!(task.kind, TaskKind::Backend);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.allowed_paths, ["src/auth/**"]);
        assert_eq!(task.acceptance_criteria, ["Login still works"]);
        assert_eq!(task.description, "Use the existing session store.");
    }

    #[test]
    fn the_contract_names_every_field_the_parser_reads() {
        let contract = Plan::contract();
        assert!(contract.contains(&format!("```{FENCE}")));
        for field in [
            "reference",
            "title",
            "description",
            "kind",
            "priority",
            "dependsOn",
            "allowedPaths",
            "acceptanceCriteria",
            "questions",
        ] {
            assert!(
                contract.contains(field),
                "the contract never mentions {field}"
            );
        }
    }

    #[test]
    fn a_proposal_from_a_working_agent_becomes_a_draft() {
        let mut farm = farm();
        farm.tasks.push(Task::new(task_id(1), "existing", 0));
        let task = from_proposal(
            &mut farm,
            &ProposedTask {
                title: "Create RefreshTokenRepository".into(),
                description: "The persistence layer does not exist.".into(),
                depends_on: vec![task_id(1), task_id(99)],
            },
            &task_id(1),
            Some(&planner_agent()),
        );
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.title, "Create RefreshTokenRepository");
        // The real dependency is kept; the phantom is dropped.
        assert_eq!(task.depends_on, [task_id(1)]);
        // And it is attributable: nobody asked for this one (FEAT-078).
        assert_eq!(
            task.origin,
            TaskOrigin::Proposed {
                agent: Some(planner_agent()),
                from: task_id(1)
            }
        );
    }

    #[test]
    fn adopting_twice_does_not_reuse_identifiers() {
        let mut farm = farm();
        let first = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"title":"A"}]}"#),
        )
        .unwrap();
        farm.tasks.extend(first);
        let second = adopt(
            &mut farm,
            &planner_agent(),
            &plan(r#"{"tasks":[{"title":"B"}]}"#),
        )
        .unwrap();
        assert_eq!(second[0].id, task_id(2));
    }
}

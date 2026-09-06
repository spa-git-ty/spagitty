// SPDX-License-Identifier: GPL-3.0-or-later

//! The unit of work.
//!
//! A task is machine-readable on purpose. The plan's rule — "never rely
//! entirely on free-form prose" — is what makes the difference between a farm
//! and four terminal windows: the scheduler reads `depends_on`, the collision
//! check reads `allowed_paths`, the verifier reads `verification`, and the
//! reviewer reads `acceptance_criteria`. Prose is carried too, in
//! `description`, because that is what the agent actually reads — but nothing
//! in the engine parses it.

use serde::{Deserialize, Serialize};

use super::{AgentId, TaskId};

/// Where a task is in its life.
///
/// The order of the variants is the order of the happy path, and
/// [`TaskStatus::can_become`] is the whole state machine in one table. It is a
/// table rather than scattered `if` statements because every screen, command
/// and recovery path has to agree on it, and a transition allowed in one place
/// and refused in another is how a farm ends up with a task that is `Running`
/// and has no process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    /// Written, but not yet part of the plan the farm will execute.
    Draft,
    /// In the plan, dependencies not yet satisfied or no agent free.
    Ready,
    Assigned,
    Running,
    /// Finished its run and is waiting for something outside itself — a
    /// dependency, a lease, the human.
    Waiting,
    /// Cannot proceed and needs a person or a new task. Distinct from `Failed`:
    /// nothing went wrong, something is missing.
    Blocked,
    Review,
    Verification,
    Done,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn label(self) -> &'static str {
        match self {
            TaskStatus::Draft => "Draft",
            TaskStatus::Ready => "Ready",
            TaskStatus::Assigned => "Assigned",
            TaskStatus::Running => "Running",
            TaskStatus::Waiting => "Waiting",
            TaskStatus::Blocked => "Blocked",
            TaskStatus::Review => "Review",
            TaskStatus::Verification => "Verification",
            TaskStatus::Done => "Done",
            TaskStatus::Failed => "Failed",
            TaskStatus::Cancelled => "Cancelled",
        }
    }

    /// Nothing further will happen to this task on its own.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    /// In the queue: the scheduler would start this if it could.
    ///
    /// `Waiting` is in and `Draft` is not — a draft is not in the plan, which
    /// is a decision somebody has yet to make rather than a queue the farm is
    /// failing to serve.
    pub fn is_queued(self) -> bool {
        matches!(self, TaskStatus::Ready | TaskStatus::Waiting)
    }

    /// Something is running or waiting on this task, so its worktree must stay.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            TaskStatus::Assigned
                | TaskStatus::Running
                | TaskStatus::Waiting
                | TaskStatus::Review
                | TaskStatus::Verification
        )
    }

    /// The transition table.
    ///
    /// Cancellation is allowed from anywhere non-terminal, because a human
    /// pressing stop must never be refused on a technicality. Everything else
    /// is the path in the plan's lifecycle diagram, plus the two loops that
    /// diagram implies but does not draw: verification failing back to the
    /// implementer, and a review asking for changes.
    pub fn can_become(self, next: TaskStatus) -> bool {
        use TaskStatus::*;
        if self.is_terminal() {
            // A finished task is finished. Re-running one is a *new* run
            // against a task moved back by an explicit retry, which goes
            // through `Task::retry` rather than through here.
            return false;
        }
        if next == Cancelled {
            return true;
        }
        match self {
            Draft => matches!(next, Ready | Blocked),
            Ready => matches!(next, Assigned | Blocked | Draft),
            Assigned => matches!(next, Running | Ready | Blocked | Failed),
            Running => matches!(next, Verification | Review | Waiting | Blocked | Failed),
            // A task that came back for want of a dependency re-enters the
            // queue rather than jumping straight to a process.
            Waiting => matches!(next, Ready | Assigned | Blocked | Failed),
            Blocked => matches!(next, Ready | Draft | Failed | Verification),
            // `Blocked` is the common one: a verification that fails hands the
            // task to a person unless the farm is unattended, and `Waiting` is
            // the unattended path back into the queue.
            Verification => matches!(next, Review | Done | Failed | Assigned | Blocked | Waiting),
            // `Assigned` again is a review asking for changes: same task, same
            // agent, another run. `Waiting` is the same thing at a level of
            // autonomy that retries without asking.
            Review => matches!(
                next,
                Done | Assigned | Blocked | Failed | Waiting | Verification
            ),
            Done | Failed | Cancelled => false,
        }
    }

    /// What a *container* may become.
    ///
    /// Containers do not follow the machine above, because that machine
    /// describes the life of a task that runs: assigned to an agent, verified,
    /// reviewed. A container has no agent, no worktree and no run — it is a
    /// heading over the work, and it follows its children (FEAT-076). The two
    /// ends it can reach are the two its children can put it at.
    pub fn can_settle(self, next: TaskStatus) -> bool {
        !self.is_terminal() && matches!(next, TaskStatus::Done | TaskStatus::Blocked)
    }
}

/// What kind of work this is. Read by routing, and by nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    Architecture,
    Backend,
    Frontend,
    Testing,
    Documentation,
    Research,
    Review,
    Integration,
    #[default]
    General,
}

impl TaskKind {
    pub const ALL: [TaskKind; 9] = [
        TaskKind::Architecture,
        TaskKind::Backend,
        TaskKind::Frontend,
        TaskKind::Testing,
        TaskKind::Documentation,
        TaskKind::Research,
        TaskKind::Review,
        TaskKind::Integration,
        TaskKind::General,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TaskKind::Architecture => "Architecture",
            TaskKind::Backend => "Backend",
            TaskKind::Frontend => "Frontend",
            TaskKind::Testing => "Testing",
            TaskKind::Documentation => "Documentation",
            TaskKind::Research => "Research",
            TaskKind::Review => "Review",
            TaskKind::Integration => "Integration",
            TaskKind::General => "General",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    High,
    #[default]
    Normal,
    Low,
}

/// Who asked for a task, and why it exists (FEAT-078).
///
/// A farm mixes work a person decided on with work a model proposed, and by the
/// time there are twenty tasks they are indistinguishable — which matters,
/// because they are not owed the same trust. A task you wrote is a decision; a
/// task an agent cut out of another one is a suggestion that happened to be
/// convenient, and it is worth being able to see which is which at a glance.
///
/// Carried on the task rather than derived from the event log: the log is
/// bounded and a task outlives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TaskOrigin {
    /// Typed by a person, in the task editor.
    Person,
    /// Cut from the goal by a planning run.
    Planned { agent: AgentId },
    /// Cut out of another task, when it turned out to be too big.
    Subtask { agent: AgentId, parent: TaskId },
    /// Offered by an agent while it was doing something else, and never asked
    /// for by anybody.
    Proposed {
        agent: Option<AgentId>,
        from: TaskId,
    },
}

impl Default for TaskOrigin {
    /// A task with no recorded origin is a task from before this existed, and
    /// the honest reading of that is "a person put it there": the only way to
    /// create one then was to type it or to accept a plan, and a plan's tasks
    /// were accepted by a person too.
    fn default() -> Self {
        TaskOrigin::Person
    }
}

impl TaskOrigin {
    /// True when a model asked for this rather than a person.
    pub fn is_agents(&self) -> bool {
        !matches!(self, TaskOrigin::Person)
    }

    /// The agent that asked for it, if one did.
    pub fn agent(&self) -> Option<&AgentId> {
        match self {
            TaskOrigin::Person => None,
            TaskOrigin::Planned { agent } | TaskOrigin::Subtask { agent, .. } => Some(agent),
            TaskOrigin::Proposed { agent, .. } => agent.as_ref(),
        }
    }
}

/// One task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    /// What the agent is told, in prose. Nothing in the engine reads it.
    #[serde(default)]
    pub description: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub kind: TaskKind,
    #[serde(default)]
    pub priority: TaskPriority,
    /// Tasks that must be `Done` before this one may start.
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    /// The task this one was cut out of, when an agent broke a big one down.
    ///
    /// A task with children is a *container*: it is never run, it is done when
    /// its children are, and it exists so a week of work can be one line in the
    /// plan and five lines underneath it. Whether a task *is* a container is
    /// derived from whether anything names it here — there is no second field
    /// saying so, and therefore no way for the two to disagree (FEAT-076).
    #[serde(default)]
    pub parent: Option<TaskId>,
    /// Who asked for this task (FEAT-078).
    #[serde(default)]
    pub origin: TaskOrigin,
    /// Who is doing it. `None` until routing or the user picks.
    #[serde(default)]
    pub assigned_agent: Option<AgentId>,
    /// Who reviewed it, so the self-review rule can be enforced against the
    /// implementer even after reassignment.
    #[serde(default)]
    pub implemented_by: Option<AgentId>,
    /// Glob patterns the agent may edit. Empty means "no declared scope", which
    /// the collision check treats as *contending with everything* rather than
    /// as free rein — the safe reading of a missing answer.
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// Verification commands for this task specifically. Added to the farm's
    /// own list rather than replacing it, unless `verification_overrides`.
    #[serde(default)]
    pub verification: Vec<String>,
    #[serde(default)]
    pub verification_overrides: bool,
    /// The branch the work lives on, once a workspace exists.
    #[serde(default)]
    pub branch: Option<String>,
    /// Destination captured when the task first starts; absent in older farms.
    #[serde(default)]
    pub merge_target: Option<String>,
    /// The worktree directory, once one has been cut.
    #[serde(default)]
    pub worktree: Option<String>,
    /// How many times this task has been sent back for changes. The farm stops
    /// looping at [`Task::MAX_ATTEMPTS`] rather than burning tokens forever.
    #[serde(default)]
    pub attempts: u32,
    /// Why it is blocked or failed, in the words of whatever decided so.
    #[serde(default)]
    pub note: Option<String>,
    pub created_ms: u64,
    #[serde(default)]
    pub updated_ms: u64,
}

impl Task {
    /// The point at which a task that keeps coming back is handed to a person.
    ///
    /// Three, because the first failure is normal, the second is a bad prompt,
    /// and the third is a task nobody has understood yet. Looping past that
    /// spends money to produce the same review comment again.
    pub const MAX_ATTEMPTS: u32 = 3;

    pub fn new(id: TaskId, title: impl Into<String>, now: u64) -> Self {
        Task {
            id,
            title: title.into(),
            description: String::new(),
            status: TaskStatus::Draft,
            kind: TaskKind::default(),
            priority: TaskPriority::default(),
            depends_on: Vec::new(),
            parent: None,
            origin: TaskOrigin::default(),
            assigned_agent: None,
            implemented_by: None,
            allowed_paths: Vec::new(),
            acceptance_criteria: Vec::new(),
            verification: Vec::new(),
            verification_overrides: false,
            branch: None,
            merge_target: None,
            worktree: None,
            attempts: 0,
            note: None,
            created_ms: now,
            updated_ms: now,
        }
    }

    /// True when the task has been round the loop as many times as it may.
    ///
    /// The limit is the farm's, so a long piece of work can be given more rope
    /// than a routine one; [`Self::MAX_ATTEMPTS`] is the default it starts at.
    pub fn is_exhausted_at(&self, limit: u32) -> bool {
        self.attempts >= limit
    }

    /// True at the default limit. Kept for the callers that have no farm to
    /// hand — the tests of this module, and nothing else.
    pub fn is_exhausted(&self) -> bool {
        self.is_exhausted_at(Self::MAX_ATTEMPTS)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskStatus::*;
    use super::*;
    use crate::model::task_id;

    #[test]
    fn a_finished_task_does_not_move() {
        for terminal in [Done, Failed, Cancelled] {
            for next in [Ready, Assigned, Running, Done, Cancelled] {
                assert!(
                    !terminal.can_become(next),
                    "{terminal:?} should not become {next:?}"
                );
            }
        }
    }

    #[test]
    fn stop_is_never_refused_on_a_technicality() {
        for state in [
            Draft,
            Ready,
            Assigned,
            Running,
            Waiting,
            Blocked,
            Review,
            Verification,
        ] {
            assert!(state.can_become(Cancelled), "{state:?} must be cancellable");
        }
    }

    #[test]
    fn a_running_task_cannot_jump_straight_to_done() {
        // Agent self-report is not success. Verification or review sits between.
        assert!(!Running.can_become(Done));
        assert!(Running.can_become(Verification));
        assert!(Verification.can_become(Done));
    }

    #[test]
    fn a_review_asking_for_changes_reassigns_rather_than_failing() {
        assert!(Review.can_become(Assigned));
        assert!(Verification.can_become(Assigned));
    }

    #[test]
    fn a_rejection_can_reach_a_person_or_the_queue_from_either_gate() {
        // Both gates end the same two ways: `Blocked` when a person has to
        // look, `Waiting` when the farm is allowed to try again. Missing
        // either transition strands a task mid-pipeline.
        for gate in [Verification, Review] {
            assert!(gate.can_become(Blocked), "{gate:?} cannot block");
            assert!(
                gate.can_become(Waiting),
                "{gate:?} cannot go back to the queue"
            );
        }
    }

    #[test]
    fn a_waiting_task_re_enters_the_queue_before_it_runs_again() {
        assert!(Waiting.can_become(Ready));
        assert!(!Waiting.can_become(Running));
    }

    #[test]
    fn active_states_are_the_ones_holding_a_worktree() {
        assert!(Running.is_active());
        assert!(Review.is_active());
        assert!(!Ready.is_active());
        assert!(!Done.is_active());
    }

    #[test]
    fn a_task_is_exhausted_at_three_attempts() {
        let mut task = Task::new(task_id(1), "Do the thing", 0);
        assert!(!task.is_exhausted());
        task.attempts = Task::MAX_ATTEMPTS;
        assert!(task.is_exhausted());
    }

    #[test]
    fn a_task_written_with_only_its_required_fields_reads_back() {
        // The planner and the automatic-proposal path both write partial tasks.
        let json = r#"{
            "id": "TASK-0001",
            "title": "Investigate auth",
            "status": "draft",
            "createdMs": 1
        }"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.kind, TaskKind::General);
        assert_eq!(task.priority, TaskPriority::Normal);
        assert!(task.depends_on.is_empty());
        assert_eq!(task.attempts, 0);
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! The farm itself: one goal, a set of tasks, and the rules the engine runs
//! them under.
//!
//! A farm belongs to a repository. There is one open at a time in the
//! application, for the same reason there is one open repository: the whole
//! interface is about *this* codebase, and a second farm running against a
//! second repository in the background is a thing nobody could supervise.

use serde::{Deserialize, Serialize};

use super::{AgentId, FarmId, Goal, Task, TaskId, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FarmStatus {
    Idle,
    Planning,
    Running,
    Paused,
    /// Nothing can start: everything left is waiting on something that is not
    /// going to happen without a person.
    Blocked,
    Reviewing,
    Completed,
    Failed,
    Cancelled,
}

impl FarmStatus {
    pub fn label(self) -> &'static str {
        match self {
            FarmStatus::Idle => "Idle",
            FarmStatus::Planning => "Planning",
            FarmStatus::Running => "Running",
            FarmStatus::Paused => "Paused",
            FarmStatus::Blocked => "Blocked",
            FarmStatus::Reviewing => "Reviewing",
            FarmStatus::Completed => "Completed",
            FarmStatus::Failed => "Failed",
            FarmStatus::Cancelled => "Cancelled",
        }
    }

    /// The scheduler only starts work in these two.
    pub fn is_live(self) -> bool {
        matches!(self, FarmStatus::Running | FarmStatus::Reviewing)
    }
}

/// How much the farm may do without asking.
///
/// The plan's five levels, kept as they are written there. The default is
/// [`Autonomy::Assisted`] rather than the plan's "1 or 2", because the first
/// time somebody runs this they should see each task finish and decide, and a
/// tool that merged on its own the first time it was used would be turned off
/// rather than turned down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Autonomy {
    /// Spagitty organises tasks. Nothing runs by itself.
    Manual,
    /// Agents run; the human approves every task.
    #[default]
    Assisted,
    /// Agents run and review each other; the human approves merges.
    SemiAuto,
    /// Verified low-risk tasks may merge themselves.
    Auto,
    /// Everything, unattended.
    Yolo,
}

impl Autonomy {
    pub const ALL: [Autonomy; 5] = [
        Autonomy::Manual,
        Autonomy::Assisted,
        Autonomy::SemiAuto,
        Autonomy::Auto,
        Autonomy::Yolo,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Autonomy::Manual => "Manual",
            Autonomy::Assisted => "Assisted",
            Autonomy::SemiAuto => "Semi-automatic",
            Autonomy::Auto => "Automatic",
            Autonomy::Yolo => "Unattended",
        }
    }

    /// May the scheduler start a task without being asked?
    pub fn starts_tasks(self) -> bool {
        self >= Autonomy::Assisted
    }

    /// May a finished task be sent to a reviewing agent without being asked?
    pub fn reviews(self) -> bool {
        self >= Autonomy::SemiAuto
    }

    /// May a verified, reviewed task be merged without being asked?
    pub fn merges(self) -> bool {
        self >= Autonomy::Auto
    }

    /// May a *failed* verification be retried without being asked?
    ///
    /// Only at the top level. Below it a failure is where the human comes back,
    /// because retrying a red build unattended is the fastest way to spend a
    /// day's budget on the same mistake.
    pub fn retries(self) -> bool {
        self == Autonomy::Yolo
    }
}

/// What the agents in this farm are allowed to do.
///
/// Every one of these defaults to off except the two an agent cannot work
/// without. They are shown in the interface rather than buried, because an
/// agent is an arbitrary program running with the user's own credentials and
/// the honest thing is to say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// Read and write files inside the task's own worktree. Without it there is
    /// no farm, so it is on and not offered as a toggle in the interface.
    pub write_files: bool,
    /// Run commands. Same: an agent that cannot run its own tests is a chat
    /// window.
    pub run_commands: bool,
    /// Reach the network. Off by default — most tasks do not need it, and the
    /// ones that do are worth being asked about.
    pub network: bool,
    /// Commit inside its own worktree.
    pub commit: bool,
    /// Push to a remote. Off, and deliberately separate from `commit`: a farm
    /// that commits locally is recoverable, a farm that pushes is public.
    pub push: bool,
    /// Merge a finished branch back. Off; this is what the approve button does.
    pub merge: bool,
    /// Delete a branch — the worktree cleanup at the end of a task.
    pub delete_branch: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            write_files: true,
            run_commands: true,
            network: false,
            commit: true,
            push: false,
            merge: false,
            delete_branch: true,
        }
    }
}

/// One farm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Farm {
    pub id: FarmId,
    /// The repository this farm belongs to, as an absolute path. Recorded so a
    /// saved farm can refuse to resume against a different checkout.
    pub repository: String,
    pub status: FarmStatus,
    pub goal: Goal,
    #[serde(default)]
    pub autonomy: Autonomy,
    #[serde(default)]
    pub permissions: Permissions,
    /// The agents this farm may use, in preference order.
    #[serde(default)]
    pub agents: Vec<AgentId>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    /// Commands run against every task's worktree before it may be marked done.
    #[serde(default)]
    pub verification: Vec<String>,
    /// How many agents may run at once.
    #[serde(default = "default_parallelism")]
    pub max_parallel: usize,
    /// How many times a task may be sent back before it needs a person.
    ///
    /// A farm setting rather than a constant, because the right number depends
    /// on the work: a routine change that fails three times is a bad prompt,
    /// and a week-long piece of work may reasonably be sent back more often
    /// than that (FEAT-076).
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
    /// The highest task number handed out so far.
    ///
    /// Held rather than derived from the task list, because deleting a task
    /// must not hand its identifier to the next piece of work: identifiers
    /// appear in branch names, worktree directories and commit messages, and a
    /// reused one makes the history lie about which task produced a commit.
    /// This is the same rule `agile/` applies to the working record's own
    /// identifiers, for the same reason.
    #[serde(default)]
    task_sequence: u32,
    /// Additional review policy and durable evidence; old farms default to Off.
    #[serde(default)]
    pub supplemental: crate::supplemental::Policy,
    #[serde(default)]
    pub supplemental_evidence: std::collections::BTreeMap<TaskId, crate::supplemental::Evidence>,
    #[serde(default)]
    pub supplemental_repairs: std::collections::BTreeMap<TaskId, u32>,
    pub created_ms: u64,
    #[serde(default)]
    pub updated_ms: u64,
}

/// Two, not the core count.
///
/// The limit that bites is not CPU — an agent is mostly waiting on a model —
/// it is the human's ability to follow what is happening, and the provider's
/// rate limit. Two is enough to prove the parallelism works and few enough to
/// read. The user raises it.
fn default_parallelism() -> usize {
    2
}

/// Three: [`crate::model::Task::MAX_ATTEMPTS`]'s reasoning, as a default rather
/// than a law.
fn default_attempts() -> u32 {
    Task::MAX_ATTEMPTS
}

impl Farm {
    pub fn new(id: FarmId, repository: impl Into<String>, goal: Goal, now: u64) -> Self {
        Farm {
            id,
            repository: repository.into(),
            status: FarmStatus::Idle,
            goal,
            autonomy: Autonomy::default(),
            permissions: Permissions::default(),
            agents: Vec::new(),
            tasks: Vec::new(),
            verification: Vec::new(),
            max_parallel: default_parallelism(),
            max_attempts: default_attempts(),
            task_sequence: 0,
            supplemental: crate::supplemental::Policy::default(),
            supplemental_evidence: Default::default(),
            supplemental_repairs: Default::default(),
            created_ms: now,
            updated_ms: now,
        }
    }

    pub fn task(&self, id: &TaskId) -> Option<&Task> {
        self.tasks.iter().find(|task| &task.id == id)
    }

    pub fn task_mut(&mut self, id: &TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| &task.id == id)
    }

    /// How many tasks are holding a process right now.
    pub fn running(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Running)
            .count()
    }

    /// Take the next task identifier, and never give it out again.
    ///
    /// The counter is seeded from the tasks in hand as well as from itself, so
    /// a farm saved before the counter existed — or one whose tasks were edited
    /// by hand in the JSON — still cannot collide with what is already there.
    pub fn allocate_task_id(&mut self) -> TaskId {
        let in_use = self
            .tasks
            .iter()
            .filter_map(|task| task.id.as_str().strip_prefix("TASK-"))
            .filter_map(|digits| digits.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        self.task_sequence = self.task_sequence.max(in_use) + 1;
        super::task_id(self.task_sequence)
    }

    /// The status the farm should be in, given its tasks.
    ///
    /// Derived rather than stored-and-updated: a farm whose status is written
    /// in one place and its tasks in another goes out of step the first time a
    /// process dies between the two writes. `Paused` and `Cancelled` are the
    /// exceptions — they are decisions rather than consequences, so they are
    /// preserved.
    pub fn derived_status(&self) -> FarmStatus {
        if matches!(
            self.status,
            FarmStatus::Paused | FarmStatus::Cancelled | FarmStatus::Planning
        ) {
            return self.status;
        }
        if self.tasks.is_empty() {
            return FarmStatus::Idle;
        }
        if self
            .tasks
            .iter()
            .all(|task| task.status == TaskStatus::Done)
        {
            return FarmStatus::Completed;
        }
        if self
            .tasks
            .iter()
            .any(|task| task.status == TaskStatus::Running)
        {
            return FarmStatus::Running;
        }
        if self
            .tasks
            .iter()
            .any(|task| matches!(task.status, TaskStatus::Review | TaskStatus::Verification))
        {
            return FarmStatus::Reviewing;
        }
        // Nothing is moving. Either something is waiting for a person, or every
        // remaining task has failed.
        let unfinished: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| !task.status.is_terminal())
            .collect();
        if unfinished.is_empty() {
            return if self.tasks.iter().any(|t| t.status == TaskStatus::Failed) {
                FarmStatus::Failed
            } else {
                FarmStatus::Completed
            };
        }
        if unfinished
            .iter()
            .all(|task| task.status == TaskStatus::Blocked)
        {
            return FarmStatus::Blocked;
        }
        FarmStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_id, GoalId};

    fn farm() -> Farm {
        Farm::new(
            FarmId::new("f1"),
            "/repo",
            Goal::new(GoalId::new("g1"), "Do the thing", 0),
            0,
        )
    }

    fn with(statuses: &[TaskStatus]) -> Farm {
        let mut farm = farm();
        for (index, status) in statuses.iter().enumerate() {
            let mut task = Task::new(task_id(index as u32 + 1), "t", 0);
            task.status = *status;
            farm.tasks.push(task);
        }
        farm
    }

    #[test]
    fn autonomy_gates_stack_upwards() {
        assert!(!Autonomy::Manual.starts_tasks());
        assert!(Autonomy::Assisted.starts_tasks());
        assert!(!Autonomy::Assisted.reviews());
        assert!(Autonomy::SemiAuto.reviews());
        assert!(!Autonomy::SemiAuto.merges());
        assert!(Autonomy::Auto.merges());
        assert!(!Autonomy::Auto.retries());
        assert!(Autonomy::Yolo.retries());
    }

    #[test]
    fn the_default_farm_cannot_push_or_merge_on_its_own() {
        let permissions = Permissions::default();
        assert!(permissions.write_files);
        assert!(permissions.run_commands);
        assert!(!permissions.network);
        assert!(!permissions.push);
        assert!(!permissions.merge);
    }

    #[test]
    fn identifiers_are_never_handed_out_twice() {
        let mut farm = farm();
        for _ in 0..2 {
            let id = farm.allocate_task_id();
            farm.tasks.push(Task::new(id, "t", 0));
        }
        farm.tasks.remove(1);
        // TASK-0002 has been deleted, and it does not come back: the next task
        // is TASK-0003, because TASK-0002 named a different piece of work and
        // its branch may still be in the reflog.
        assert_eq!(farm.allocate_task_id(), task_id(3));
        assert_eq!(farm.allocate_task_id(), task_id(4));
    }

    #[test]
    fn a_farm_saved_before_the_counter_existed_does_not_collide() {
        let mut farm = with(&[TaskStatus::Done, TaskStatus::Done, TaskStatus::Ready]);
        // `taskSequence` is absent from the file, so it reads as zero. Seeding
        // from the tasks in hand is what stops the next identifier being
        // TASK-0001 all over again.
        assert_eq!(farm.task_sequence, 0);
        assert_eq!(farm.allocate_task_id(), task_id(4));
    }

    #[test]
    fn an_empty_farm_is_idle() {
        assert_eq!(farm().derived_status(), FarmStatus::Idle);
    }

    #[test]
    fn a_farm_with_everything_done_is_completed() {
        assert_eq!(
            with(&[TaskStatus::Done, TaskStatus::Done]).derived_status(),
            FarmStatus::Completed
        );
    }

    #[test]
    fn one_running_task_makes_the_farm_running() {
        assert_eq!(
            with(&[TaskStatus::Done, TaskStatus::Running, TaskStatus::Ready]).derived_status(),
            FarmStatus::Running
        );
    }

    #[test]
    fn review_shows_when_nothing_is_executing() {
        assert_eq!(
            with(&[TaskStatus::Done, TaskStatus::Review]).derived_status(),
            FarmStatus::Reviewing
        );
    }

    #[test]
    fn a_farm_whose_remaining_work_is_all_blocked_says_so() {
        assert_eq!(
            with(&[TaskStatus::Done, TaskStatus::Blocked]).derived_status(),
            FarmStatus::Blocked
        );
    }

    #[test]
    fn a_farm_that_ended_with_a_failure_is_failed_not_completed() {
        assert_eq!(
            with(&[TaskStatus::Done, TaskStatus::Failed]).derived_status(),
            FarmStatus::Failed
        );
    }

    #[test]
    fn a_paused_farm_stays_paused_whatever_its_tasks_say() {
        let mut farm = with(&[TaskStatus::Running]);
        farm.status = FarmStatus::Paused;
        assert_eq!(farm.derived_status(), FarmStatus::Paused);
    }

    #[test]
    fn running_counts_only_processes() {
        let farm = with(&[TaskStatus::Running, TaskStatus::Running, TaskStatus::Review]);
        assert_eq!(farm.running(), 2);
    }

    #[test]
    fn a_farm_written_before_max_parallel_existed_gets_the_default() {
        let json = r#"{
            "id": "f1", "repository": "/repo", "status": "idle",
            "goal": {"id": "g1", "title": "t", "createdMs": 0},
            "createdMs": 0
        }"#;
        let farm: Farm = serde_json::from_str(json).unwrap();
        assert_eq!(farm.max_parallel, 2);
        assert_eq!(farm.autonomy, Autonomy::Assisted);
        assert!(farm.permissions.write_files);
    }
}

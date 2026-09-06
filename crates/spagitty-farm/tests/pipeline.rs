// SPDX-License-Identifier: GPL-3.0-or-later

//! A task, end to end, against a real repository.
//!
//! # Why the agent is a shell script
//!
//! The point of these tests is the *pipeline* — worktree, run, handoff,
//! verification, review, merge, cleanup — and every one of those is real here:
//! a real git repository, a real worktree, a real child process, a real merge.
//! What is not real is the model, because a test that needs a model is a test
//! that does not run in CI, and the parts of this crate that talk to a model
//! are exactly the parts that are already unit-tested (the adapters build a
//! command line; the parsers read a transcript).
//!
//! So the "agent" is a script that writes a file, commits it, and prints a
//! handoff block. That is what a real agent does, minus the thinking.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use spagitty_core::fixture::Fixture;
use spagitty_farm::agent::adapters::custom::PROMPT_TOKEN;
use spagitty_farm::model::*;
use spagitty_farm::service::{FarmService, Recorder};

/// An agent that is `/bin/sh` running `body`, registered as a custom provider
/// so the whole registry and adapter path is exercised.
///
/// `/bin/sh` rather than a script written to disk, and the difference matters:
/// writing an executable and immediately running it fails intermittently with
/// `ETXTBSY` when a sibling test thread forks while the file is still open for
/// writing. That is not a defect in anything being tested, and it made this
/// suite fail about one run in five. `/bin/sh` is never written by us.
///
/// `sh -c BODY NAME PROMPT` puts the prompt in `$1`, which is where a real
/// `{prompt}` template would put it, and `sh --version` answers detection.
fn scripted_agent(_dir: &Path, name: &str, body: &str) -> AgentDefinition {
    AgentDefinition {
        id: AgentId::new(name),
        provider: AgentProvider::Custom,
        display_name: name.to_string(),
        executable: PathBuf::from("/bin/sh"),
        capabilities: [
            AgentCapability::Coding,
            AgentCapability::Review,
            AgentCapability::Planning,
        ]
        .into_iter()
        .collect(),
        input_mode: AgentInputMode::CliPrompt,
        traits: AgentTraits {
            resumable_sessions: false,
            streaming: true,
            structured_output: true,
            tool_use: true,
            headless: true,
        },
        role: AgentRole::General,
        // The prompt is written to a file by the body below, so a test can
        // assert what the agent was told — the only way to check the context
        // builder is actually wired to the runner.
        extra_args: vec![
            "-c".to_string(),
            body.to_string(),
            name.to_string(),
            PROMPT_TOKEN.to_string(),
        ],
        enabled: true,
    }
}

struct Harness {
    repo: Fixture,
    _bin: tempfile::TempDir,
    /// Behind an `Arc` so a test can hand the service to a second thread —
    /// which is the only way to prove a lock is *not* being held (BUG-020).
    /// Every other test reaches its methods through `Deref` and is unaffected.
    service: Arc<FarmService>,
    recorder: Arc<Recorder>,
}

impl Harness {
    fn new() -> Self {
        let repo = Fixture::woven();
        let bin = tempfile::tempdir().unwrap();
        let recorder = Arc::new(Recorder::default());
        let service = Arc::new(FarmService::open(repo.path(), recorder.clone()));
        Harness {
            repo,
            _bin: bin,
            service,
            recorder,
        }
    }

    fn bin(&self) -> &Path {
        self._bin.path()
    }

    /// An agent that does the work: writes a file, commits, reports completion.
    fn worker(&self, name: &str) -> AgentId {
        let definition = scripted_agent(
            self.bin(),
            name,
            &format!(
                r#"printf '%s' "$1" > prompt.txt
echo "working"
echo "made by {name}" > agent-output.txt
git add -A >/dev/null 2>&1
git -c user.name=Agent -c user.email=agent@example.com commit -q -m "TASK work by {name}" >/dev/null 2>&1
echo '```spagitty-handoff'
echo '{{"status":"completed","summary":"wrote agent-output.txt","filesChanged":["agent-output.txt"]}}'
echo '```'"#
            ),
        );
        let id = definition.id.clone();
        self.service.save_agent(definition).unwrap();
        id
    }

    /// An agent that reviews: prints a verdict and changes nothing.
    fn reviewer(&self, name: &str, decision: &str) -> AgentId {
        let definition = scripted_agent(
            self.bin(),
            name,
            &format!(
                r#"echo "reviewing"
echo '```spagitty-review'
echo '{{"decision":"{decision}","summary":"looked at it","issues":[{{"severity":"high","file":"agent-output.txt","message":"needs more"}}]}}'
echo '```'"#
            ),
        );
        let id = definition.id.clone();
        self.service.save_agent(definition).unwrap();
        id
    }

    fn task(&self, title: &str) -> TaskId {
        let mut task = Task::new(TaskId::new("unset"), title, 0);
        task.status = TaskStatus::Ready;
        self.service.add_task(task).unwrap().id
    }

    /// Run a task to completion, synchronously.
    fn run(&self, task: &TaskId, agent: &AgentId) {
        self.service.run_task(task, Some(agent.clone())).unwrap();
        self.service.await_task(task).unwrap();
    }

    fn status(&self, task: &TaskId) -> TaskStatus {
        self.service.farm().unwrap().task(task).unwrap().status
    }

    fn worktree(&self, task: &TaskId) -> PathBuf {
        PathBuf::from(
            self.service
                .farm()
                .unwrap()
                .task(task)
                .unwrap()
                .worktree
                .clone()
                .unwrap(),
        )
    }
}

/// Nothing here needs `PATH`.
///
/// The scripted agents are registered by absolute path, as a custom agent
/// added from the settings screen is, and `save_agent` probes that path
/// directly. An earlier version of this file put the scripts on `PATH` so
/// whole-machine detection would find them; that made every test wait for the
/// real agent CLIs installed on the machine to answer `--version`, and turned a
/// one-second suite into a ninety-second one.
fn arrange(_harness: &Harness) {}

#[test]
fn a_task_runs_in_its_own_worktree_and_leaves_the_repository_alone() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship the thing", "").unwrap();
    let agent = harness.worker("worker-alone");
    let task = harness.task("Write a file");

    harness.run(&task, &agent);

    let worktree = harness.worktree(&task);
    assert!(
        worktree.join("agent-output.txt").exists(),
        "the agent did no work"
    );
    // The user's own checkout is untouched. This is the whole reason for the
    // worktree isolation.
    assert!(!harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn the_agent_is_given_the_prompt_the_context_builder_wrote() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Add OAuth login", "").unwrap();
    let agent = harness.worker("worker-prompt");

    let mut task = Task::new(TaskId::new("unset"), "Implement the backend", 0);
    task.status = TaskStatus::Ready;
    task.acceptance_criteria = vec!["Tokens are single-use".into()];
    task.allowed_paths = vec!["backend/**".into()];
    let task = harness.service.add_task(task).unwrap().id;

    harness.run(&task, &agent);

    let prompt = std::fs::read_to_string(harness.worktree(&task).join("prompt.txt")).unwrap();
    assert!(prompt.contains("Add OAuth login"), "no goal in the prompt");
    assert!(prompt.contains("Tokens are single-use"), "no criteria");
    assert!(prompt.contains("- backend/**"), "no allowed paths");
    assert!(prompt.contains("spagitty-handoff"), "no handoff contract");
}

#[test]
fn a_task_with_no_verification_reaches_review_and_waits_for_a_person() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-noverify");
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    // Not Done. Nothing was checked and nobody has approved it.
    assert_eq!(harness.status(&task), TaskStatus::Review);
    let verification = harness.service.verification_of(&task).unwrap();
    assert!(verification.unverified);
}

#[test]
fn a_failing_verification_blocks_the_task_rather_than_passing_it() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    harness
        .service
        .configure(|farm| farm.verification = vec!["/bin/sh -c 'exit 1'".into()])
        .unwrap();
    let agent = harness.worker("worker-redtests");
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    assert_eq!(harness.status(&task), TaskStatus::Blocked);
    let note = harness
        .service
        .farm()
        .unwrap()
        .task(&task)
        .unwrap()
        .note
        .clone()
        .unwrap();
    assert!(note.contains("Verification failed"), "{note}");
}

#[test]
fn a_passing_verification_carries_the_task_to_review() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    harness
        .service
        .configure(|farm| farm.verification = vec!["/bin/sh -c 'test -f agent-output.txt'".into()])
        .unwrap();
    let agent = harness.worker("worker-greentests");
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(harness.service.verification_of(&task).unwrap().passed);
}

#[test]
fn the_verification_runs_against_the_worktree_not_the_repository() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    // `agent-output.txt` exists only inside the worktree the agent worked in.
    harness
        .service
        .configure(|farm| farm.verification = vec!["/bin/sh -c 'test -f agent-output.txt'".into()])
        .unwrap();
    let agent = harness.worker("worker-cwd");
    let task = harness.task("Do it");

    harness.run(&task, &agent);
    assert!(harness.service.verification_of(&task).unwrap().passed);
}

#[test]
fn an_agent_that_fails_fails_the_task() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let definition = scripted_agent(harness.bin(), "worker-crash", "echo 'boom' >&2\nexit 4");
    let agent = definition.id.clone();
    harness.service.save_agent(definition).unwrap();
    let task = harness.task("Do it");

    harness.run(&task, &agent);
    assert_eq!(harness.status(&task), TaskStatus::Failed);
}

#[test]
fn an_agent_that_reports_blocked_blocks_the_task_with_its_reason() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let definition = scripted_agent(
        harness.bin(),
        "worker-blocked",
        r#"echo '```spagitty-handoff'
echo '{"status":"blocked","summary":"The persistence layer does not exist."}'
echo '```'"#,
    );
    let agent = definition.id.clone();
    harness.service.save_agent(definition).unwrap();
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    assert_eq!(harness.status(&task), TaskStatus::Blocked);
    let note = harness
        .service
        .farm()
        .unwrap()
        .task(&task)
        .unwrap()
        .note
        .clone()
        .unwrap();
    assert!(note.contains("persistence layer"), "{note}");
}

#[test]
fn an_agent_proposal_becomes_a_draft_task_nobody_scheduled() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let definition = scripted_agent(
        harness.bin(),
        "worker-proposer",
        r#"echo '```spagitty-handoff'
echo '{"status":"completed","summary":"done","proposedTasks":[{"title":"Create RefreshTokenRepository"}]}'
echo '```'"#,
    );
    let agent = definition.id.clone();
    harness.service.save_agent(definition).unwrap();
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    let farm = harness.service.farm().unwrap();
    let proposed = farm
        .tasks
        .iter()
        .find(|candidate| candidate.title == "Create RefreshTokenRepository")
        .expect("the proposal was dropped");
    // Draft: the agent proposes, Spagitty owns the graph, a person schedules.
    assert_eq!(proposed.status, TaskStatus::Draft);
}

#[test]
fn a_reviewer_that_asks_for_changes_does_not_let_the_task_through() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    harness
        .service
        .configure(|farm| farm.autonomy = Autonomy::SemiAuto)
        .unwrap();
    let worker = harness.worker("worker-reviewed");
    harness.reviewer("reviewer-strict", "request_changes");
    let task = harness.task("Do it");

    harness.run(&task, &task_agent(&worker));
    // The review is a second run against the same task.
    harness.service.await_task(&task).unwrap();

    assert_ne!(harness.status(&task), TaskStatus::Done);
    let review = harness.service.review_of(&task).unwrap();
    assert!(!review.approved());
}

/// A tiny indirection so the call above reads as "run it with the worker".
fn task_agent(id: &AgentId) -> AgentId {
    id.clone()
}

#[test]
fn a_merge_puts_the_work_in_the_users_branch_and_removes_the_worktree() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-merge");
    let task = harness.task("Do it");
    harness.run(&task, &agent);
    let worktree = harness.worktree(&task);

    harness.service.merge(&task).unwrap();

    assert_eq!(harness.status(&task), TaskStatus::Done);
    assert!(
        harness.repo.path().join("agent-output.txt").exists(),
        "the work did not reach the repository"
    );
    assert!(!worktree.exists(), "the worktree was not cleaned up");
}

#[test]
fn a_merge_is_never_a_fast_forward_so_the_agents_branch_stays_visible() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-noff");
    let task = harness.task("Do it");
    harness.run(&task, &agent);
    harness.service.merge(&task).unwrap();

    let head = harness.repo.git(&["log", "-1", "--pretty=%P"]);
    assert_eq!(
        head.split_whitespace().count(),
        2,
        "the merge fast-forwarded and the branch disappeared from the history"
    );
}

#[test]
fn a_farm_survives_a_restart_and_does_not_resume_on_its_own() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    // Manual, so starting the farm does not also start an agent: what is being
    // tested is that a *live* farm comes back paused, not what it was running.
    harness
        .service
        .configure(|farm| farm.autonomy = Autonomy::Manual)
        .unwrap();
    let task = harness.task("Do it");
    harness.service.start_farm().unwrap();
    assert_eq!(harness.service.farm().unwrap().status, FarmStatus::Running);

    // A second service against the same repository is what a restart is.
    let recorder = Arc::new(Recorder::default());
    let reopened = FarmService::open(harness.repo.path(), recorder);
    let farm = reopened.farm().expect("the farm was not saved");

    assert_eq!(farm.goal.title, "Ship it");
    assert!(farm.task(&task).is_some());
    // Paused, not Running: restarting into agent runs nobody asked for is a
    // surprise, not a recovery.
    assert_eq!(farm.status, FarmStatus::Paused);
}

#[test]
fn a_task_that_was_running_when_the_application_died_comes_back_as_waiting() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let task = harness.task("Do it");
    harness
        .service
        .edit_task(&task, |task| task.status = TaskStatus::Running)
        .unwrap();

    let reopened = FarmService::open(harness.repo.path(), Arc::new(Recorder::default()));
    let status = reopened.farm().unwrap().task(&task).unwrap().status;
    // Not Failed — the work may be in the worktree — and certainly not Done.
    assert_eq!(status, TaskStatus::Waiting);
}

#[test]
fn the_activity_log_survives_a_restart() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    harness.task("Do it");

    let reopened = FarmService::open(harness.repo.path(), Arc::new(Recorder::default()));
    let events = reopened.events();
    assert!(
        events
            .iter()
            .any(|recorded| matches!(recorded.event, FarmEvent::TaskCreated { .. })),
        "{events:?}"
    );
}

#[test]
fn events_describe_what_happened_in_order() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-events");
    let task = harness.task("Do it");
    harness.run(&task, &agent);

    let events = harness.recorder.events();
    let kinds: Vec<&str> = events
        .iter()
        .map(|recorded| match recorded.event {
            FarmEvent::TaskCreated { .. } => "created",
            FarmEvent::TaskStatusChanged { .. } => "status",
            FarmEvent::AgentStarted { .. } => "started",
            FarmEvent::AgentOutput { .. } => "output",
            FarmEvent::AgentStopped { .. } => "stopped",
            FarmEvent::WorkspaceChanged { .. } => "workspace",
            _ => "other",
        })
        .collect();

    for expected in ["created", "workspace", "started", "output", "stopped"] {
        assert!(
            kinds.contains(&expected),
            "no {expected} event in {kinds:?}"
        );
    }
    assert!(
        kinds.iter().position(|k| *k == "started").unwrap()
            < kinds.iter().position(|k| *k == "stopped").unwrap()
    );
}

#[test]
fn a_task_cannot_run_before_what_it_depends_on() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-deps");
    let first = harness.task("First");
    let second = harness.task("Second");
    harness
        .service
        .edit_task(&second, |task| task.depends_on = vec![first.clone()])
        .unwrap();

    let error = harness.service.run_task(&second, Some(agent)).unwrap_err();
    assert_eq!(error.kind(), "refused");
    assert!(error.to_string().contains(first.as_str()));
}

#[test]
fn two_tasks_on_the_same_paths_cannot_run_at_once() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let slow = scripted_agent(harness.bin(), "worker-slow", "sleep 20");
    let agent = slow.id.clone();
    harness.service.save_agent(slow).unwrap();

    let first = harness.task("First");
    let second = harness.task("Second");
    for task in [&first, &second] {
        harness
            .service
            .edit_task(task, |task| task.allowed_paths = vec!["src/**".into()])
            .unwrap();
    }

    harness
        .service
        .run_task(&first, Some(agent.clone()))
        .unwrap();
    let error = harness.service.run_task(&second, Some(agent)).unwrap_err();
    assert_eq!(error.kind(), "pathContended");

    harness.service.cancel_task(&first).unwrap();
}

#[test]
fn stopping_a_task_stops_its_agent() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let slow = scripted_agent(harness.bin(), "worker-stoppable", "echo going\nsleep 20");
    let agent = slow.id.clone();
    harness.service.save_agent(slow).unwrap();
    let task = harness.task("Do it");

    harness.service.run_task(&task, Some(agent)).unwrap();
    let started = std::time::Instant::now();
    harness.service.cancel_task(&task).unwrap();
    harness.service.await_task(&task).unwrap();

    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert_eq!(harness.status(&task), TaskStatus::Cancelled);
}

#[test]
fn deleting_a_running_task_is_refused() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let slow = scripted_agent(harness.bin(), "worker-undeletable", "sleep 20");
    let agent = slow.id.clone();
    harness.service.save_agent(slow).unwrap();
    let task = harness.task("Do it");

    harness.service.run_task(&task, Some(agent)).unwrap();
    let error = harness.service.delete_task(&task).unwrap_err();
    assert_eq!(error.kind(), "refused");

    harness.service.cancel_task(&task).unwrap();
}

#[test]
fn a_stale_worktree_from_a_deleted_task_is_found_and_swept() {
    let harness = Harness::new();
    arrange(&harness);
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-stale");
    let task = harness.task("Do it");
    harness.run(&task, &agent);

    // Forget the farm entirely, leaving the worktree behind — which is what a
    // deleted `farm.json` or an interrupted uninstall leaves.
    spagitty_farm::persistence::forget(harness.repo.path());
    let reopened = FarmService::open(harness.repo.path(), Arc::new(Recorder::default()));

    let stale = reopened.stale_workspaces();
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert_eq!(stale[0].task, task);
}

#[test]
fn a_farm_reads_the_repositorys_agent_rules() {
    let harness = Harness::new();
    arrange(&harness);
    std::fs::write(
        harness.repo.path().join("AGENTS.md"),
        "# Agent rules\n\nNever add a dependency.",
    )
    .unwrap();
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-rules");
    let task = harness.task("Do it");

    harness.run(&task, &agent);

    let prompt = std::fs::read_to_string(harness.worktree(&task).join("prompt.txt")).unwrap();
    assert!(
        prompt.contains("Never add a dependency."),
        "the rules were not sent"
    );
}

#[test]
fn the_starter_rules_file_is_never_written_over_an_existing_one() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    std::fs::write(harness.repo.path().join("AGENTS.md"), "mine").unwrap();

    let error = harness.service.write_policy_template().unwrap_err();
    assert_eq!(error.kind(), "refused");
    assert_eq!(
        std::fs::read_to_string(harness.repo.path().join("AGENTS.md")).unwrap(),
        "mine"
    );
}

#[test]
fn a_task_cannot_be_moved_to_a_status_the_machine_does_not_have() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let task = harness.task("Do it");
    // Ready → Done would skip verification and review entirely.
    let error = harness
        .service
        .set_status(&task, TaskStatus::Done, None)
        .unwrap_err();
    assert_eq!(error.kind(), "badTransition");
}

#[test]
fn a_dependency_cycle_is_refused_at_the_point_it_is_created() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let first = harness.task("First");
    let second = harness.task("Second");
    harness
        .service
        .edit_task(&second, |task| task.depends_on = vec![first.clone()])
        .unwrap();

    let error = harness
        .service
        .edit_task(&first, |task| task.depends_on = vec![second.clone()])
        .unwrap_err();
    assert_eq!(error.kind(), "dependencyCycle");
    // And the edit did not land.
    assert!(harness
        .service
        .farm()
        .unwrap()
        .task(&first)
        .unwrap()
        .depends_on
        .is_empty());
}

/// BUG-020 — the window froze from the moment a farm started planning until the
/// planner had finished.
///
/// The cause was not the waiting, which is expected and happens on a thread of
/// its own: it was *where* the waiting happened. `collect_plan` took the
/// planning session out of the map inside an `if let` whose scrutinee was the
/// `sessions` mutex guard, so on edition 2021 that guard outlived the body and
/// the lock was held for the whole run. Every command that starts, stops or
/// schedules a task takes the same lock, and they run on the main thread.
///
/// So the assertion is not about output or state. It is that a second thread
/// can still take that lock while a planning run is in flight, and it fails —
/// by timing out — against the old shape.
#[test]
fn planning_does_not_hold_the_lock_that_starts_and_stops_tasks() {
    use std::sync::mpsc;
    use std::time::Duration;

    let harness = Harness::new();
    harness.service.create("Ship the thing", "").unwrap();

    // A planner that thinks for two seconds before answering. Long enough that
    // a held lock is unmistakable, short enough to leave in the suite.
    let planner = scripted_agent(
        harness.bin(),
        "planner-slow",
        r#"sleep 2
echo '```spagitty-plan'
echo '{"tasks":[{"reference":"t1","title":"Investigate"}]}'
echo '```'"#,
    );
    let planner_id = planner.id.clone();
    harness.service.save_agent(planner).unwrap();

    let run = harness.service.plan(Some(planner_id)).unwrap();

    let collecting = {
        let service = harness.service.clone();
        std::thread::spawn(move || service.collect_plan(&run).unwrap())
    };

    // The probe: any command that reaches the sessions map. Cancelling a task
    // that does not exist is the cheapest one — it takes the lock, finds
    // nothing, and only then fails on the status change, which is discarded.
    let (done, answered) = mpsc::channel();
    {
        let service = harness.service.clone();
        std::thread::spawn(move || {
            let _ = service.cancel_task(&TaskId::new("no-such-task"));
            let _ = done.send(());
        });
    }

    assert!(
        answered.recv_timeout(Duration::from_millis(750)).is_ok(),
        "a command needing the sessions lock waited for the planning run to \
         finish — the guard is being held across the wait (BUG-020)"
    );

    // And planning still works: the point is the lock, not skipping the wait.
    let tasks = collecting.join().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, TaskStatus::Draft);
}

/// TASK-030 — the activity history is answered from memory, not from disk.
///
/// The interface asks for a snapshot after every burst of events, and the
/// answer used to be "read and re-parse the whole log", which made the cost of
/// watching a farm proportional to how much it had already done. What is
/// asserted here is that the two agree: the in-memory history is not a cache
/// that can drift from the file, it is the file, kept.
#[test]
fn the_history_is_held_in_memory_and_matches_what_is_on_disk() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-history");
    let task = harness.task("Do it");
    harness.run(&task, &agent);

    let held = harness.service.events();
    let stored = spagitty_farm::persistence::store::load_events(harness.repo.path());
    assert!(!held.is_empty(), "a whole run produced no history");
    assert_eq!(held, stored);

    // Transcript lines are not history: one run produces thousands and they
    // would push everything else out.
    assert!(
        !held
            .iter()
            .any(|recorded| matches!(recorded.event, FarmEvent::AgentOutput { .. })),
        "transcript lines are being kept as history"
    );

    // And the tail is the end of it, not the start.
    let tail = harness.service.events_tail(3);
    assert_eq!(tail.len(), 3.min(held.len()));
    assert_eq!(tail.last(), held.last());
}

/// TASK-030 — a farm reopened on a repository remembers what happened.
#[test]
fn reopening_a_farm_reads_its_history_back() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = harness.worker("worker-reopen");
    let task = harness.task("Do it");
    harness.run(&task, &agent);
    let before = harness.service.events();

    let reopened = FarmService::open(harness.repo.path(), Arc::new(Recorder::default()));

    assert_eq!(reopened.events(), before);
}

// ── Large work: containers and decomposition (FEAT-076) ──────────────────

/// An agent that answers a decomposition with two subtasks.
fn splitter(harness: &Harness, name: &str) -> AgentId {
    let definition = scripted_agent(
        harness.bin(),
        name,
        r#"printf '%s' "$1" > /dev/null
printf '%s\n' '```spagitty-plan'
printf '%s\n' '{"tasks":[{"reference":"a","title":"First half","allowedPaths":["src/a/**"]},{"reference":"b","title":"Second half","dependsOn":["a"],"allowedPaths":["src/b/**"]}]}'
printf '%s\n' '```'"#,
    );
    let id = definition.id.clone();
    harness.service.save_agent(definition).unwrap();
    id
}

#[test]
fn breaking_a_task_down_produces_drafts_under_it() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-split");
    let big = harness.task("Rework the auth module");

    let run = harness.service.decompose(&big, Some(agent)).unwrap();
    let children = harness.service.collect_plan(&run).unwrap();

    assert_eq!(children.len(), 2);
    for child in &children {
        // Drafts, so the plan-review band asks before any of them runs.
        assert_eq!(child.status, TaskStatus::Draft);
        assert_eq!(child.parent.as_ref(), Some(&big));
    }
    // The agent's own references became real dependencies between the children.
    assert_eq!(children[1].depends_on, [children[0].id.clone()]);
}

#[test]
fn a_task_that_was_broken_down_is_not_run_itself() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-norun");
    let worker = harness.worker("worker-norun");
    let big = harness.task("Rework the auth module");

    let run = harness.service.decompose(&big, Some(agent)).unwrap();
    harness.service.collect_plan(&run).unwrap();

    let error = harness.service.run_task(&big, Some(worker)).unwrap_err();
    assert_eq!(error.kind(), "refused");
    assert!(error.to_string().contains("underneath"), "{error}");
}

#[test]
fn a_container_follows_its_children_to_done() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-follow");
    let big = harness.task("Rework the auth module");

    let run = harness.service.decompose(&big, Some(agent)).unwrap();
    let children = harness.service.collect_plan(&run).unwrap();

    // Nothing has finished: the container is still where it was.
    harness.service.tick();
    assert_eq!(harness.status(&big), TaskStatus::Ready);

    for child in &children {
        harness
            .service
            .edit_task(&child.id, |task| task.status = TaskStatus::Done)
            .unwrap();
    }
    harness.service.tick();

    assert_eq!(harness.status(&big), TaskStatus::Done);
}

#[test]
fn deleting_a_heading_does_not_delete_what_was_under_it() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-delete");
    let big = harness.task("Rework the auth module");
    let run = harness.service.decompose(&big, Some(agent)).unwrap();
    let children = harness.service.collect_plan(&run).unwrap();

    harness.service.delete_task(&big).unwrap();

    let farm = harness.service.farm().unwrap();
    assert!(farm.task(&big).is_none());
    for child in &children {
        let child = farm
            .task(&child.id)
            .expect("a child was deleted with its heading");
        // Work in its own right now, not an orphan pointing at nothing.
        assert_eq!(child.parent, None);
    }
}

#[test]
fn only_one_thing_is_planned_at_a_time() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-once");
    let first = harness.task("One");
    let second = harness.task("Two");

    let run = harness
        .service
        .decompose(&first, Some(agent.clone()))
        .unwrap();
    // The second would share the planning session slot and collect the first
    // one's transcript.
    let error = harness
        .service
        .decompose(&second, Some(agent))
        .expect_err("a second planning run was allowed to start");
    assert_eq!(error.kind(), "refused");

    harness.service.collect_plan(&run).unwrap();
}

// ── Who asked for a task (FEAT-078) ──────────────────────────────────────

#[test]
fn a_task_a_person_typed_is_recorded_as_theirs() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let mine = harness.task("Something I decided on");

    let task = harness.service.farm().unwrap().task(&mine).unwrap().clone();
    assert_eq!(task.origin, TaskOrigin::Person);
    assert!(!task.origin.is_agents());
}

#[test]
fn a_planned_task_names_the_agent_that_planned_it() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-origin");

    let run = harness.service.plan(Some(agent.clone())).unwrap();
    let planned = harness.service.collect_plan(&run).unwrap();

    assert_eq!(
        planned[0].origin,
        TaskOrigin::Planned {
            agent: agent.clone()
        }
    );
    assert!(planned[0].origin.is_agents());
    assert_eq!(planned[0].origin.agent(), Some(&agent));
}

#[test]
fn a_subtask_says_which_task_it_was_cut_out_of() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    let agent = splitter(&harness, "planner-origin-sub");
    let big = harness.task("Rework the auth module");

    let run = harness
        .service
        .decompose(&big, Some(agent.clone()))
        .unwrap();
    let children = harness.service.collect_plan(&run).unwrap();

    assert_eq!(
        children[0].origin,
        TaskOrigin::Subtask {
            agent,
            parent: big.clone()
        }
    );
}

#[test]
fn an_existing_farm_reads_as_the_persons_own_work() {
    // A task saved before origins existed has no `origin` field. The only ways
    // to make one then were typing it or accepting a plan, and both are a
    // person's decision — so the default is the honest reading, not a guess.
    let task: Task =
        serde_json::from_str(r#"{"id":"TASK-0001","title":"Old","status":"ready","createdMs":0}"#)
            .unwrap();
    assert_eq!(task.origin, TaskOrigin::Person);
}

// ── A long session stays cheap (TASK-031) ────────────────────────────────

/// TASK-031 — what a snapshot costs at a size nobody has tried yet.
///
/// A number rather than an opinion. It is asserted loosely, because the point
/// is to notice a change of *order* — a snapshot that starts carrying every
/// transcript, say — rather than to police a few bytes.
#[test]
fn a_farm_of_two_hundred_tasks_still_serialises_small() {
    let harness = Harness::new();
    harness.service.create("Ship it", "").unwrap();
    for index in 0..200 {
        harness.task(&format!("Task number {index}"));
    }

    let farm = harness.service.farm().unwrap();
    let json = serde_json::to_string(&farm).unwrap();
    let per_task = json.len() / 200;
    println!(
        "200 tasks serialise to {} bytes, {per_task} a task",
        json.len()
    );

    assert!(
        per_task < 1_200,
        "a task costs {per_task} bytes on the wire; it was about 400 when this was written"
    );
    assert!(
        json.len() < 250_000,
        "two hundred tasks serialise to {} bytes",
        json.len()
    );
}

#[test]
fn cancellation_reaches_a_session_already_owned_by_a_waiter() {
    let harness = Harness::new();
    harness.service.create("Cancel a watched run", "").unwrap();
    let slow = scripted_agent(
        harness.bin(),
        "watched-worker",
        "sleep 1; echo survived > survived.txt",
    );
    let agent = slow.id.clone();
    harness.service.save_agent(slow).unwrap();
    let task = harness.task("Stop this work");
    harness.service.run_task(&task, Some(agent)).unwrap();
    let worktree = harness.worktree(&task);
    let service = harness.service.clone();
    let waiting_task = task.clone();
    let waiter = std::thread::spawn(move || service.await_task(&waiting_task));
    std::thread::sleep(std::time::Duration::from_millis(100));
    harness.service.cancel_task(&task).unwrap();
    waiter.join().unwrap().unwrap();
    assert!(
        !worktree.join("survived.txt").exists(),
        "stopped agent kept writing"
    );
    assert_eq!(harness.status(&task), TaskStatus::Cancelled);
}

#[test]
fn cancelling_the_farm_stops_an_already_watched_agent() {
    let harness = Harness::new();
    harness.service.create("Stop everything", "").unwrap();
    let slow = scripted_agent(
        harness.bin(),
        "farm-stop-worker",
        "sleep 1; echo survived > survived.txt",
    );
    let agent = slow.id.clone();
    harness.service.save_agent(slow).unwrap();
    let task = harness.task("Stop this work");
    harness.service.run_task(&task, Some(agent)).unwrap();
    let worktree = harness.worktree(&task);
    let service = harness.service.clone();
    let waiting_task = task.clone();
    let waiter = std::thread::spawn(move || service.await_task(&waiting_task));
    std::thread::sleep(std::time::Duration::from_millis(100));
    harness.service.cancel_farm().unwrap();
    waiter.join().unwrap().unwrap();
    assert!(!worktree.join("survived.txt").exists());
    assert_eq!(harness.status(&task), TaskStatus::Cancelled);
    assert_eq!(
        harness.service.farm().unwrap().status,
        FarmStatus::Cancelled
    );
}

#[test]
fn a_planner_remains_cancellable_while_its_plan_is_being_collected() {
    let harness = Harness::new();
    harness.service.create("Stop planning", "").unwrap();
    let planner = scripted_agent(
        harness.bin(),
        "collected-planner",
        "sleep 1; echo survived > planner-survived.txt",
    );
    let agent = planner.id.clone();
    harness.service.save_agent(planner).unwrap();
    let run = harness.service.plan(Some(agent)).unwrap();
    let service = harness.service.clone();
    let waiter = std::thread::spawn(move || service.collect_plan(&run));
    std::thread::sleep(std::time::Duration::from_millis(100));
    harness.service.cancel_plan().unwrap();
    assert!(waiter.join().unwrap().unwrap().is_empty());
    assert!(!harness.repo.path().join("planner-survived.txt").exists());
    assert!(harness.service.farm().unwrap().tasks.is_empty());
}

#[test]
fn pending_watchers_are_claimed_once_and_cancelled_results_stay_cancelled() {
    let harness = Harness::new();
    harness.service.create("One watcher", "").unwrap();
    let worker = scripted_agent(harness.bin(), "watch-once", "sleep 2");
    let agent = worker.id.clone();
    harness.service.save_agent(worker).unwrap();
    let task = harness.task("Wait");
    harness.service.run_task(&task, Some(agent)).unwrap();
    assert_eq!(harness.service.watch_pending(), 1);
    for _ in 0..50 {
        assert_eq!(harness.service.watch_pending(), 0);
    }
    harness.service.cancel_task(&task).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while harness
        .service
        .runs()
        .iter()
        .any(|run| run.outcome == RunOutcome::Running)
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(harness
        .service
        .runs()
        .iter()
        .all(|run| run.outcome == RunOutcome::Cancelled));
    assert_eq!(harness.status(&task), TaskStatus::Cancelled);
}

#[test]
fn completion_watching_follows_the_implementation_into_review() {
    let harness = Harness::new();
    harness.service.create("Watch a review", "").unwrap();
    harness
        .service
        .configure(|farm| {
            farm.autonomy = Autonomy::SemiAuto;
            farm.verification = vec!["/bin/sh -c 'exit 0'".into()];
        })
        .unwrap();
    let worker = harness.worker("watched-implementation");
    harness.reviewer("watched-review", "approve");
    let task = harness.task("Do and review");
    harness.service.run_task(&task, Some(worker)).unwrap();
    assert_eq!(harness.service.watch_pending(), 1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while harness.service.review_of(&task).is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(harness
        .service
        .review_of(&task)
        .expect("review was never collected")
        .approved());
    assert_eq!(harness.status(&task), TaskStatus::Review);
    let runs = harness.service.runs();
    assert_eq!(runs.len(), 2);
    assert!(runs
        .iter()
        .all(|run| matches!(run.outcome, RunOutcome::Completed { .. })));
}

#[test]
fn cancellation_during_verification_does_not_start_review_or_later_checks() {
    let harness = Harness::new();
    harness.service.create("Cancel checks", "").unwrap();
    harness
        .service
        .configure(|farm| {
            farm.verification = vec![
                "/bin/sh -c 'echo started > check-started; sleep 2; echo bad > check-survived'"
                    .into(),
                "/bin/sh -c 'echo bad > later-check'".into(),
            ]
        })
        .unwrap();
    let worker = harness.worker("checked-worker");
    let task = harness.task("Do then check");
    harness.service.run_task(&task, Some(worker)).unwrap();
    let worktree = harness.worktree(&task);
    harness.service.watch_pending();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !worktree.join("check-started").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(worktree.join("check-started").exists());
    harness.service.cancel_task(&task).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert_eq!(harness.status(&task), TaskStatus::Cancelled);
    assert!(!worktree.join("check-survived").exists());
    assert!(!worktree.join("later-check").exists());
    assert!(harness.service.review_of(&task).is_none());
}

fn reviewed_task(harness: &Harness, checked: bool) -> TaskId {
    harness.service.create("Reviewed work", "").unwrap();
    harness
        .service
        .configure(|farm| {
            farm.autonomy = Autonomy::SemiAuto;
            if checked {
                farm.verification = vec!["/bin/sh -c 'exit 0'".into()];
            }
        })
        .unwrap();
    let worker = harness.worker("evidence-worker");
    harness.reviewer("evidence-reviewer", "approve");
    let task = harness.task("Change with evidence");
    harness.run(&task, &worker);
    harness.service.await_task(&task).unwrap();
    task
}

fn enable_merging(harness: &Harness, allowed: bool) {
    harness
        .service
        .configure(|farm| {
            farm.autonomy = Autonomy::Auto;
            farm.permissions.merge = allowed;
        })
        .unwrap();
}

#[test]
fn automatic_merging_refuses_a_task_with_no_checks() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, false);
    enable_merging(&harness, true);
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(!harness.repo.path().join("agent-output.txt").exists());
    assert!(harness
        .service
        .farm()
        .unwrap()
        .task(&task)
        .unwrap()
        .note
        .as_deref()
        .unwrap()
        .contains("verification"));
}

#[test]
fn automatic_merging_accepts_current_checks_and_independent_review() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, true);
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Done);
    assert!(harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn automatic_merging_requires_explicit_merge_permission() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, false);
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(!harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn dirty_or_changed_work_invalidates_merge_evidence() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, true);
    let worktree = harness.worktree(&task);
    std::fs::write(worktree.join("agent-output.txt"), "changed after review").unwrap();
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Review);
    harness
        .repo
        .git(&["-C", worktree.to_str().unwrap(), "add", "agent-output.txt"]);
    harness.repo.git(&[
        "-C",
        worktree.to_str().unwrap(),
        "commit",
        "-qm",
        "Later edit",
    ]);
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(!harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn changing_verification_commands_invalidates_previous_evidence() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, true);
    harness
        .service
        .configure(|farm| farm.verification.push("/bin/sh -c 'exit 1'".into()))
        .unwrap();
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(!harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn a_branch_switch_does_not_redirect_a_task_merge() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, true);
    harness.repo.git(&["switch", "-c", "unrelated"]);
    let before = harness.repo.git(&["rev-parse", "HEAD"]);
    harness.service.approve(&task).unwrap();
    assert_eq!(harness.repo.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(harness.status(&task), TaskStatus::Review);
    assert!(harness
        .service
        .farm()
        .unwrap()
        .task(&task)
        .unwrap()
        .note
        .as_deref()
        .unwrap()
        .contains("Switch back"));
}

#[test]
fn merge_rejects_an_invalid_status_before_touching_git() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    harness
        .service
        .edit_task(&task, |task| task.status = TaskStatus::Ready)
        .unwrap();
    let before = harness.repo.git(&["rev-parse", "HEAD"]);
    assert!(harness.service.merge(&task).is_err());
    assert_eq!(harness.repo.git(&["rev-parse", "HEAD"]), before);
}

#[test]
fn restart_never_restores_unproven_merge_evidence() {
    let harness = Harness::new();
    let task = reviewed_task(&harness, true);
    enable_merging(&harness, true);
    let reopened = FarmService::open(harness.repo.path(), Arc::new(Recorder::default()));
    reopened.approve(&task).unwrap();
    assert_eq!(
        reopened.farm().unwrap().task(&task).unwrap().status,
        TaskStatus::Review
    );
    assert!(!harness.repo.path().join("agent-output.txt").exists());
}

#[test]
fn a_failed_reviewer_cannot_approve_with_its_partial_output() {
    let harness = Harness::new();
    harness.service.create("Failed review", "").unwrap();
    harness
        .service
        .configure(|farm| {
            farm.autonomy = Autonomy::Auto;
            farm.permissions.merge = true;
            farm.verification = vec!["/bin/sh -c 'exit 0'".into()];
        })
        .unwrap();
    let worker = harness.worker("review-crash-worker");
    let reviewer = scripted_agent(harness.bin(), "review-crash", "echo '```spagitty-review'; echo '{\"decision\":\"approve\",\"summary\":\"looks fine\",\"issues\":[]}'; echo '```'; exit 7");
    harness.service.save_agent(reviewer).unwrap();
    let task = harness.task("Do not merge");
    harness.run(&task, &worker);
    harness.service.await_task(&task).unwrap();
    assert_eq!(harness.status(&task), TaskStatus::Blocked);
    assert!(!harness.repo.path().join("agent-output.txt").exists());
    assert!(harness.service.review_of(&task).is_none());
}

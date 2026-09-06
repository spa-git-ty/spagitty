// SPDX-License-Identifier: GPL-3.0-or-later

//! The farm's API, and the only place that changes anything.
//!
//! Every other module in this crate is either data or a pure function over data.
//! This one holds the state, spawns the processes, writes the files and moves
//! the tasks — and it is the only one that does, which is what makes the rest
//! testable without a repository or a model.
//!
//! # The shape of a task's life
//!
//! ```text
//! Ready → (scheduler) → Assigned → worktree cut → Running → agent
//!                                                    ↓
//!                                          handoff parsed, verified
//!                                                    ↓
//!                              Verification → passed? → Review → approved? → Done
//!                                    ↓ no                  ↓ no
//!                                 Assigned ←───────────────┘   (up to three attempts)
//! ```
//!
//! Nothing skips a step. In particular, no path from an agent's own report
//! reaches `Done` — the plan's rule that agent success is not task success is
//! enforced by there being no such transition, not by remembering to check.
//!
//! # Locking
//!
//! One mutex around the whole farm, held only while state is being read or
//! written and never across a process spawn. That is the same shape
//! `commands.rs` uses for the open repository, and for the same reason: the
//! alternative is a lock per field and a deadlock the first time two of them
//! are needed together. Agent runs happen on their own threads with no lock
//! held; they take it again to record what happened.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use spagitty_core::shell;

use crate::agent::{adapter::AgentRunRequest, adapter_for, AgentRegistry};
use crate::context::{self, Context, DependencyResult};
use crate::error::{Error, Result};
use crate::execution::log::TranscriptWriter;
use crate::execution::{self, process::Sink, Session};
use crate::model::*;
use crate::orchestrator::{planner, router, scheduler, Scoreboard};
use crate::persistence::store;
use crate::policy::{self, Policy};
use crate::review::{reviewer, Review};
use crate::verification::{evidence, verifier, CommandResult, Verification};
use crate::workspace::{self, Leases};

/// Where farm events go.
///
/// A trait so the crate has no opinion about Tauri. The Tauri layer emits; the
/// tests collect.
pub trait Observer: Send + Sync + std::fmt::Debug {
    fn event(&self, event: Recorded);
}

/// An observer that keeps everything, for tests and headless use.
#[derive(Debug, Default)]
pub struct Recorder {
    events: Mutex<Vec<Recorded>>,
}

impl Recorder {
    pub fn events(&self) -> Vec<Recorded> {
        self.events.lock().expect("recorded events").clone()
    }

    /// Every event of one shape, for a test that wants to assert on a
    /// transition without depending on what else happened.
    pub fn statuses(&self) -> Vec<(TaskId, TaskStatus)> {
        self.events()
            .into_iter()
            .filter_map(|recorded| match recorded.event {
                FarmEvent::TaskStatusChanged { task, status, .. } => Some((task, status)),
                _ => None,
            })
            .collect()
    }
}

impl Observer for Recorder {
    fn event(&self, event: Recorded) {
        self.events.lock().expect("recorded events").push(event);
    }
}

/// A sink that turns agent output lines into farm events.
#[derive(Debug)]
struct RunSink {
    observer: Arc<dyn Observer>,
    run: RunId,
    task: TaskId,
    /// When this run last said something, shared with the service (FEAT-077).
    ///
    /// An atomic rather than a lock: this is written once per line of output,
    /// which for a talkative agent is thousands of times, and it is read when
    /// somebody asks for the runs. Nothing waits on it.
    heard: Arc<AtomicU64>,
}

impl Sink for RunSink {
    fn line(&self, text: &str) {
        self.heard.store(now_ms(), Ordering::Relaxed);
        // Stamped here rather than when it reaches a screen: this is the moment
        // the agent said it, and a transcript timed by when the webview
        // happened to render it would be a transcript of the interface.
        self.observer.event(Recorded::now(FarmEvent::AgentOutput {
            run: self.run.clone(),
            task: self.task.clone(),
            line: text.to_string(),
        }));
    }
}

/// One process the farm is about to start.
///
/// A struct rather than seven parameters. They arrive together, they are all
/// required, and half of them are identifiers of the same shape — which is
/// exactly the argument list where a caller silently swaps two and nothing
/// complains.
struct Launch {
    run: RunId,
    task: TaskId,
    agent: AgentId,
    phase: RunPhase,
    command: crate::agent::AgentCommand,
    /// Where the process runs: a task's own worktree, never the user's checkout.
    workdir: PathBuf,
    /// How to read what it prints. See [`crate::execution::narrate`].
    narrator: Box<dyn execution::narrate::Narrator>,
}

/// How many runs are kept in memory.
///
/// Two hundred, *plus* the most recent run of every task however old it is —
/// see [`State::remember_run`]. A long session used to grow this list forever
/// and clone the whole of it into every snapshot, so the cost of watching a
/// farm rose with everything it had already done (TASK-031). What a person
/// reads is a task's own runs and the recent ones; the transcripts stay on disk
/// and outlive both.
const MAX_RUNS: usize = 200;

/// The mutable heart of a farm.
#[derive(Debug, Default)]
struct State {
    farm: Option<Farm>,
    leases: Leases,
    runs: Vec<AgentRun>,
    scoreboard: Scoreboard,
    /// The handoff each task's last implementation run produced, so a dependent
    /// task can be told what came before.
    handoffs: HashMap<TaskId, Handoff>,
    /// What a reviewer asked for, carried into the next attempt's prompt.
    change_requests: HashMap<TaskId, String>,
    verifications: HashMap<TaskId, Verification>,
    reviews: HashMap<TaskId, Review>,
    verified_commits: HashMap<TaskId, evidence::CheckedCommit>,
    reviewed_commits: HashMap<TaskId, evidence::ReviewedCommit>,
    review_inputs: HashMap<RunId, String>,
    /// When each run last said something, by run (FEAT-077). Written by the
    /// sink on the reader thread, read when the runs are asked for.
    heard: HashMap<RunId, Arc<AtomicU64>>,
    /// The task a planning run is breaking down, when it is breaking one down
    /// rather than planning the goal (FEAT-076).
    ///
    /// Held here rather than passed to `collect_plan`, because the collector
    /// runs on a thread the Tauri layer spawned from a command that has already
    /// returned: there is nobody left holding the argument.
    planning_parent: Option<TaskId>,
}

impl State {
    /// Record a run, and forget the ones nobody will ask for (TASK-031).
    ///
    /// Two things are kept: the most recent [`MAX_RUNS`], and the newest run of
    /// every task the farm still has. The second half is what makes the first
    /// safe — a task's own panel shows its runs, and a farm where task three
    /// has been quiet for a day would otherwise open on an empty panel because
    /// two hundred other runs happened since.
    ///
    /// Both are bounded by something a person can see: a number, and how many
    /// tasks they are looking at. A run that is dropped has not lost its
    /// transcript — that is on disk, and outlives the farm.
    fn remember_run(&mut self, run: AgentRun) {
        self.runs.push(run);
        if self.runs.len() <= MAX_RUNS {
            return;
        }

        // The newest run of each task that still exists. A run belonging to a
        // deleted task is history nobody can navigate to.
        let live: std::collections::HashSet<TaskId> = self
            .farm
            .as_ref()
            .map(|farm| farm.tasks.iter().map(|task| task.id.clone()).collect())
            .unwrap_or_default();
        let mut newest: HashMap<TaskId, RunId> = HashMap::new();
        for entry in &self.runs {
            if live.contains(&entry.task) {
                newest.insert(entry.task.clone(), entry.id.clone());
            }
        }

        let keep_from = self.runs.len() - MAX_RUNS;
        let mut seen = 0;
        self.runs.retain(|entry| {
            let recent = seen >= keep_from;
            seen += 1;
            recent || newest.get(&entry.task) == Some(&entry.id)
        });

        // The clocks of runs nobody holds any more go with them.
        let held: std::collections::HashSet<RunId> =
            self.runs.iter().map(|entry| entry.id.clone()).collect();
        self.heard.retain(|id, _| held.contains(id));
    }
}

/// The only owner allowed to reap and complete a particular run.
struct PendingRun {
    task: TaskId,
    run: RunId,
    phase: RunPhase,
    session: Session,
}

/// One repository's farm.
pub struct FarmService {
    repo: PathBuf,
    state: Mutex<State>,
    registry: Mutex<AgentRegistry>,
    observer: Arc<dyn Observer>,
    /// The agent processes running right now, by task.
    sessions: Mutex<HashMap<RunId, PendingRun>>,
    cancellations: Mutex<HashMap<TaskId, (RunId, execution::Cancellation)>>,
    /// The event history, in memory.
    ///
    /// The log on disk is still the record — this is loaded from it once, when
    /// the farm opens, and appended to as events are emitted. It exists because
    /// the interface asks for the history after every burst of events, and
    /// answering by reading and re-parsing two thousand JSON objects made the
    /// cost of watching a farm proportional to how much it had already done
    /// (TASK-030).
    recent: Mutex<VecDeque<Recorded>>,
    verification_stops: Mutex<HashMap<TaskId, Arc<AtomicBool>>>,
    merge_lock: Mutex<()>,
}

impl FarmService {
    /// Open the farm for `repo`, loading whatever was saved.
    ///
    /// **Nothing is detected here.** Probing the machine means running four
    /// agent CLIs and waiting for each to print a version, and a Node-based CLI
    /// behind a version-manager shim takes a noticeable fraction of a second to
    /// do it. Doing that inside `open` makes opening a repository wait for it,
    /// on the thread that opened it.
    ///
    /// So detection is a separate call — [`Self::detect_agents`] — which the
    /// Tauri layer makes on a thread of its own as the repository opens, and
    /// which the Farm settings screen makes again when the user asks. Until it
    /// has run, every agent reads as unavailable, which is honest: nothing has
    /// checked yet.
    pub fn open(repo: impl Into<PathBuf>, observer: Arc<dyn Observer>) -> Self {
        let repo = repo.into();
        let repo_for_events = repo.clone();
        let registry: AgentRegistry = store::load_registry(&repo).unwrap_or_default();
        let farm = store::load_farm(&repo);
        let service = FarmService {
            repo,
            state: Mutex::new(State {
                farm,
                ..State::default()
            }),
            registry: Mutex::new(registry),
            observer,
            sessions: Mutex::new(HashMap::new()),
            cancellations: Mutex::new(HashMap::new()),
            recent: Mutex::new(store::load_events(&repo_for_events).into()),
            verification_stops: Mutex::new(HashMap::new()),
            merge_lock: Mutex::new(()),
        };
        service.recover();
        service
    }

    // ── Agents ───────────────────────────────────────────────────────────

    /// Look for installed agents again, and save what the user has configured.
    pub fn detect_agents(&self) -> Vec<crate::agent::AgentStatus> {
        let mut registry = self.registry.lock().expect("registry lock");
        registry.detect_all();
        let _ = store::save_registry(&self.repo, &*registry);
        registry.statuses()
    }

    pub fn agents(&self) -> Vec<crate::agent::AgentStatus> {
        self.registry.lock().expect("registry lock").statuses()
    }

    /// Providers with no definition yet, so the settings screen can offer them.
    pub fn undetected(&self) -> Vec<AgentProvider> {
        self.registry.lock().expect("registry lock").undetected()
    }

    /// Save one agent's configuration, and check that one agent if its executable
    /// path changed.
    ///
    /// Only that one: see [`AgentRegistry::probe`] for why saving a row does
    /// not re-run every agent CLI on the machine. Toggling roles or capabilities
    /// leaves the executable alone and skips probing.
    pub fn save_agent(&self, definition: AgentDefinition) -> Result<()> {
        let id = definition.id.clone();
        let mut registry = self.registry.lock().expect("registry lock");
        let executable_changed = registry
            .get(&id)
            .map(|existing| existing.executable != definition.executable)
            .unwrap_or(true);
        registry.put(definition);
        if executable_changed {
            registry.probe(&id);
        }
        store::save_registry(&self.repo, &*registry)
    }

    pub fn remove_agent(&self, id: &AgentId) -> Result<()> {
        let mut registry = self.registry.lock().expect("registry lock");
        registry.remove(id);
        store::save_registry(&self.repo, &*registry)
    }

    // ── The farm ─────────────────────────────────────────────────────────

    pub fn farm(&self) -> Option<Farm> {
        self.state.lock().expect("farm lock").farm.clone()
    }

    pub fn policy(&self) -> Policy {
        policy::read(&self.repo)
    }

    /// Write a starter `AGENTS.md`, refusing to overwrite one that exists.
    pub fn write_policy_template(&self) -> Result<PathBuf> {
        let path = self.repo.join("AGENTS.md");
        if path.exists() {
            return Err(Error::Refused(
                "AGENTS.md already exists; it was not overwritten".into(),
            ));
        }
        let goal = self
            .farm()
            .map(|farm| farm.goal.title)
            .unwrap_or_else(|| "Describe what this project is for.".into());
        std::fs::write(&path, policy::template(&goal))?;
        Ok(path)
    }

    /// Start a new farm against a goal, replacing any previous one.
    pub fn create(&self, goal_title: &str, description: &str) -> Result<Farm> {
        let now = now_ms();
        let mut goal = Goal::new(GoalId::new(format!("goal-{now}")), goal_title, now);
        goal.description = description.to_string();

        let mut farm = Farm::new(
            FarmId::new(format!("farm-{now}")),
            self.repo.to_string_lossy(),
            goal,
            now,
        );
        // A farm that has never been told what to check inherits nothing, and
        // the interface says so rather than pretending it will verify.
        farm.verification = Vec::new();

        let mut state = self.state.lock().expect("farm lock");
        state.farm = Some(farm.clone());
        state.leases = Leases::default();
        self.persist(&state)?;
        drop(state);

        self.emit(FarmEvent::FarmStatusChanged {
            status: FarmStatus::Idle,
        });
        Ok(farm)
    }

    /// Replace the farm's settings — autonomy, permissions, parallelism,
    /// verification commands.
    pub fn configure(&self, apply: impl FnOnce(&mut Farm)) -> Result<Farm> {
        let mut state = self.state.lock().expect("farm lock");
        let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
        apply(farm);
        farm.updated_ms = now_ms();
        let updated = farm.clone();
        self.persist(&state)?;
        Ok(updated)
    }

    /// The whole history the farm is holding, oldest first.
    pub fn events(&self) -> Vec<Recorded> {
        self.recent
            .lock()
            .expect("recent events lock")
            .iter()
            .cloned()
            .collect()
    }

    /// The last `limit` events, oldest first.
    ///
    /// What a screen actually renders is a screenful and a scrollback, not the
    /// whole history, and every event sent is one serialised across the bridge.
    pub fn events_tail(&self, limit: usize) -> Vec<Recorded> {
        let recent = self.recent.lock().expect("recent events lock");
        recent
            .iter()
            .skip(recent.len().saturating_sub(limit))
            .cloned()
            .collect()
    }

    /// Every run, with when each last spoke folded in.
    pub fn runs(&self) -> Vec<AgentRun> {
        let state = self.state.lock().expect("farm lock");
        state
            .runs
            .iter()
            .map(|run| {
                let mut run = run.clone();
                if let Some(heard) = state.heard.get(&run.id) {
                    let at = heard.load(Ordering::Relaxed);
                    run.last_output_ms = (at > 0).then_some(at);
                }
                run
            })
            .collect()
    }

    /// Why each queued task is not running, by task.
    ///
    /// The scheduler's own answer rather than a second opinion — see
    /// [`scheduler::why_waiting`].
    pub fn waiting_reasons(&self) -> Vec<(TaskId, String)> {
        let state = self.state.lock().expect("farm lock");
        let Some(farm) = state.farm.as_ref() else {
            return Vec::new();
        };
        let registry = self.registry.lock().expect("registry lock");
        scheduler::why_waiting(farm, &registry, &state.leases)
    }

    /// Move several drafts into the plan at once.
    ///
    /// One call rather than one per task, because accepting a plan is one
    /// decision: eight round trips would be eight writes of the farm file and
    /// eight chances to land half a plan.
    pub fn ready_all(&self, ids: &[TaskId]) -> Result<()> {
        for id in ids {
            // A task that has already moved is not an error here: accepting a
            // plan twice is a double click, not a failure.
            if let Some(status) = self
                .farm()
                .and_then(|farm| farm.task(id).map(|task| task.status))
            {
                if status == TaskStatus::Draft {
                    self.ready(id)?;
                }
            }
        }
        Ok(())
    }

    /// Throw several drafts away.
    pub fn discard_all(&self, ids: &[TaskId]) -> Result<()> {
        for id in ids {
            self.delete_task(id)?;
        }
        Ok(())
    }

    pub fn scoreboard(&self) -> Vec<(AgentId, router::Record)> {
        self.state.lock().expect("farm lock").scoreboard.all()
    }

    pub fn verification_of(&self, task: &TaskId) -> Option<Verification> {
        self.state
            .lock()
            .expect("farm lock")
            .verifications
            .get(task)
            .cloned()
    }

    pub fn review_of(&self, task: &TaskId) -> Option<Review> {
        self.state
            .lock()
            .expect("farm lock")
            .reviews
            .get(task)
            .cloned()
    }

    pub fn handoff_of(&self, task: &TaskId) -> Option<Handoff> {
        self.state
            .lock()
            .expect("farm lock")
            .handoffs
            .get(task)
            .cloned()
    }

    /// The tail of a run's transcript.
    pub fn transcript(&self, run: &RunId, task: &TaskId) -> String {
        execution::log::tail(&execution::log::log_path(&self.repo, task, run))
    }

    // ── Tasks ────────────────────────────────────────────────────────────

    pub fn add_task(&self, mut task: Task) -> Result<Task> {
        let mut state = self.state.lock().expect("farm lock");
        let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
        task.id = farm.allocate_task_id();
        task.created_ms = now_ms();
        task.updated_ms = task.created_ms;

        let mut proposed = farm.tasks.clone();
        proposed.push(task.clone());
        crate::orchestrator::validate(&proposed)?;

        farm.tasks.push(task.clone());
        self.persist(&state)?;
        drop(state);

        self.emit(FarmEvent::TaskCreated {
            task: task.id.clone(),
            title: task.title.clone(),
        });
        Ok(task)
    }

    /// Edit a task, refusing an edit that would create a cycle.
    pub fn edit_task(&self, id: &TaskId, apply: impl FnOnce(&mut Task)) -> Result<Task> {
        let mut state = self.state.lock().expect("farm lock");
        let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;

        let mut proposed = farm.tasks.clone();
        let target = proposed
            .iter_mut()
            .find(|task| &task.id == id)
            .ok_or_else(|| Error::NoSuchTask(id.clone()))?;
        apply(target);
        target.updated_ms = now_ms();
        let updated = target.clone();
        crate::orchestrator::validate(&proposed)?;

        farm.tasks = proposed;
        self.persist(&state)?;
        Ok(updated)
    }

    /// Move a task to a new status, refusing a transition the machine does not
    /// have.
    pub fn set_status(&self, id: &TaskId, next: TaskStatus, note: Option<String>) -> Result<()> {
        {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
            let task = farm
                .task_mut(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            if task.status == next && task.note == note {
                return Ok(());
            }
            if task.status != next && !task.status.can_become(next) {
                return Err(Error::BadTransition {
                    from: task.status.label().to_string(),
                    to: next.label().to_string(),
                });
            }
            task.status = next;
            task.note = note.clone();
            task.updated_ms = now_ms();
            if next.is_terminal() || next == TaskStatus::Blocked {
                state.leases.release(id);
            }
            self.persist(&state)?;
        }
        self.emit(FarmEvent::TaskStatusChanged {
            task: id.clone(),
            status: next,
            note,
        });
        self.emit_farm_status();
        Ok(())
    }

    /// Put a `Draft` task into the plan.
    pub fn ready(&self, id: &TaskId) -> Result<()> {
        self.set_status(id, TaskStatus::Ready, None)
    }

    /// Stop whatever is running for this task and cancel it.
    pub fn cancel_task(&self, id: &TaskId) -> Result<()> {
        self.set_status(id, TaskStatus::Cancelled, Some("Stopped by hand.".into()))?;
        if let Some(stop) = self
            .verification_stops
            .lock()
            .expect("verification stop lock")
            .get(id)
        {
            stop.store(true, Ordering::Release);
        }
        self.stop_session(id);
        Ok(())
    }

    fn stop_session(&self, id: &TaskId) {
        let cancellation = self
            .cancellations
            .lock()
            .expect("cancellation lock")
            .get(id)
            .cloned();
        if let Some((_, cancellation)) = cancellation {
            cancellation.cancel();
        }
        // An unclaimed session still needs its readers joined and child reaped.
        // The option is dropped outside the map lock.
        let run = self
            .cancellations
            .lock()
            .expect("cancellation lock")
            .get(id)
            .map(|(run, _)| run.clone());
        if let Some(run) = run {
            let pending = self.sessions.lock().expect("sessions lock").remove(&run);
            if pending.is_some() {
                drop(pending);
                self.forget_cancellation(id, &run);
            }
        }
    }

    fn forget_cancellation(&self, task: &TaskId, run: &RunId) {
        let mut handles = self.cancellations.lock().expect("cancellation lock");
        if handles.get(task).map(|(id, _)| id) == Some(run) {
            handles.remove(task);
        }
    }

    /// Put a failed or blocked task back in the queue for another attempt.
    ///
    /// The attempt counter is *not* reset. A person retrying a task three times
    /// is making a decision; a farm that forgot each time would loop.
    pub fn retry_task(&self, id: &TaskId) -> Result<()> {
        if self
            .verification_stops
            .lock()
            .expect("verification stop lock")
            .contains_key(id)
        {
            return Err(Error::Refused("Verification is still stopping.".into()));
        }
        if self
            .cancellations
            .lock()
            .expect("cancellation lock")
            .contains_key(id)
        {
            return Err(Error::Refused("The previous run is still stopping.".into()));
        }
        let mut state = self.state.lock().expect("farm lock");
        let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
        let task = farm
            .task_mut(id)
            .ok_or_else(|| Error::NoSuchTask(id.clone()))?;
        task.status = TaskStatus::Ready;
        task.note = None;
        task.updated_ms = now_ms();
        self.persist(&state)?;
        drop(state);

        self.emit(FarmEvent::TaskStatusChanged {
            task: id.clone(),
            status: TaskStatus::Ready,
            note: None,
        });
        Ok(())
    }

    /// Give a task to a particular agent.
    pub fn assign(&self, id: &TaskId, agent: &AgentId) -> Result<()> {
        self.registry
            .lock()
            .expect("registry lock")
            .runnable(agent)?;
        self.edit_task(id, |task| task.assigned_agent = Some(agent.clone()))?;
        self.emit(FarmEvent::TaskAssigned {
            task: id.clone(),
            agent: agent.clone(),
        });
        Ok(())
    }

    /// Remove a task, and its workspace with it.
    pub fn delete_task(&self, id: &TaskId) -> Result<()> {
        let provider = {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
            let task = farm
                .task(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?
                .clone();
            if task.status.is_active() {
                return Err(Error::Refused(
                    "This task is running. Stop it before deleting it.".into(),
                ));
            }
            farm.tasks.retain(|candidate| &candidate.id != id);
            // Anything cut out of it becomes work in its own right rather than
            // an orphan pointing at a task that is gone. Deleting a heading is
            // not deleting what was under it (FEAT-076).
            for child in farm.tasks.iter_mut() {
                if child.parent.as_ref() == Some(id) {
                    child.parent = None;
                }
            }
            state.leases.release(id);
            self.persist(&state)?;
            self.provider_of(&task)
        };
        // Never forced: a task with uncommitted work keeps its worktree, and
        // the Worktrees screen shows it.
        let _ = workspace::remove(&self.repo, id, provider, false);
        Ok(())
    }

    // ── Running ──────────────────────────────────────────────────────────

    pub fn start_farm(&self) -> Result<()> {
        self.set_farm_status(FarmStatus::Running)?;
        self.tick();
        Ok(())
    }

    pub fn pause_farm(&self) -> Result<()> {
        self.set_farm_status(FarmStatus::Paused)
    }

    /// Stop every running agent and cancel every unfinished task.
    pub fn cancel_farm(&self) -> Result<()> {
        let unfinished: Vec<TaskId> = self
            .farm()
            .map(|farm| {
                farm.tasks
                    .iter()
                    .filter(|task| !task.status.is_terminal())
                    .map(|task| task.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        for task in unfinished {
            let _ = self.set_status(
                &task,
                TaskStatus::Cancelled,
                Some("The farm was stopped.".into()),
            );
        }
        for stop in self
            .verification_stops
            .lock()
            .expect("verification stop lock")
            .values()
        {
            stop.store(true, Ordering::Release);
        }
        let running: Vec<TaskId> = self
            .cancellations
            .lock()
            .expect("cancellation lock")
            .keys()
            .cloned()
            .collect();
        for task in running {
            self.stop_session(&task);
        }
        self.set_farm_status(FarmStatus::Cancelled)
    }

    /// Ask the scheduler what should happen, and make it happen.
    ///
    /// Called after every state change rather than on a timer, for the reason
    /// in [`crate::orchestrator::scheduler`]. Safe to call at any time: a farm
    /// with nothing to do does nothing.
    pub fn tick(&self) {
        let decisions = {
            let state = self.state.lock().expect("farm lock");
            let Some(farm) = state.farm.as_ref() else {
                return;
            };
            let registry = self.registry.lock().expect("registry lock");
            scheduler::decide(farm, &registry, &state.leases)
        };

        for decision in decisions {
            match decision {
                scheduler::Decision::Block { task, why } => {
                    let _ = self.set_status(&task, TaskStatus::Blocked, Some(why));
                }
                scheduler::Decision::Start { task, agent } => {
                    if let Err(error) = self.start_task(&task, &agent) {
                        let _ =
                            self.set_status(&task, TaskStatus::Blocked, Some(error.to_string()));
                    }
                }
                scheduler::Decision::Settle { task, status } => {
                    let _ = self.settle(&task, status);
                }
            }
        }
        self.emit_farm_status();
    }

    /// Move a container to where its children have put it (FEAT-076).
    ///
    /// Deliberately not [`Self::set_status`]: that enforces the machine a task
    /// with a *run* follows — assigned, verified, reviewed — and a container has
    /// no run to follow it with. `TaskStatus::can_settle` is the rule that
    /// applies instead, and it is in the model beside the other one.
    fn settle(&self, id: &TaskId, status: TaskStatus) -> Result<()> {
        {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
            let task = farm
                .task_mut(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            if !task.status.can_settle(status) {
                return Ok(());
            }
            task.status = status;
            task.note = match status {
                TaskStatus::Blocked => Some("Something underneath this did not finish.".into()),
                _ => None,
            };
            task.updated_ms = now_ms();
            self.persist(&state)?;
        }
        self.emit(FarmEvent::TaskStatusChanged {
            task: id.clone(),
            status,
            note: None,
        });
        Ok(())
    }

    /// Run one task now, whatever the scheduler thinks.
    ///
    /// This is the button on the task detail screen. It still refuses a task
    /// whose dependencies are unmet — "run it anyway" would produce a change
    /// built against work that does not exist yet.
    pub fn run_task(&self, id: &TaskId, agent: Option<AgentId>) -> Result<()> {
        let (task, chosen) = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm
                .task(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?
                .clone();

            let graph = crate::orchestrator::Graph::new(&farm.tasks);
            if graph.is_container(id) {
                // Running the heading would cut a worktree for a task whose
                // whole content is "these five things" (FEAT-076).
                return Err(Error::Refused(
                    "This task was broken down. Run the tasks underneath it instead.".into(),
                ));
            }
            let unmet = graph.unmet(&task);
            if !unmet.is_empty() {
                return Err(Error::Refused(format!(
                    "{} has not finished.",
                    unmet
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }

            let registry = self.registry.lock().expect("registry lock");
            let chosen = match agent {
                Some(id) => registry.runnable(&id)?.id,
                None => task
                    .assigned_agent
                    .clone()
                    .filter(|id| registry.runnable(id).is_ok())
                    .or_else(|| router::pick(&registry, task.kind).map(|d| d.id))
                    .ok_or_else(|| {
                        Error::AgentUnavailable("no agent is available for this task".into())
                    })?,
            };
            (task, chosen)
        };

        if let Some(holder) = {
            let state = self.state.lock().expect("farm lock");
            state.leases.blocked_by(&task.id, &task.allowed_paths)
        } {
            return Err(Error::PathContended {
                task: task.id.clone(),
                holder,
                path: task
                    .allowed_paths
                    .first()
                    .cloned()
                    .unwrap_or_else(|| workspace::lock::EVERYTHING.to_string()),
            });
        }

        self.start_task(id, &chosen)
    }

    /// Cut a workspace, build the prompt, and start the agent.
    fn start_task(&self, id: &TaskId, agent_id: &AgentId) -> Result<()> {
        let definition = self
            .registry
            .lock()
            .expect("registry lock")
            .runnable(agent_id)?;

        if !definition.traits.headless {
            return Err(Error::Refused(format!(
                "{} needs a terminal, so it cannot be run by the farm",
                definition.display_name
            )));
        }

        // Everything read under the lock, released before anything is spawned.
        let (task, prompt, unattended) = {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm
                .task(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?
                .clone();

            let policy = policy::read(&self.repo);
            let verification = verifier::commands_for(&farm.verification, &task);
            let dependencies = self.dependency_results(farm, &state.handoffs, &task);
            let prompt = context::implementation(&Context {
                farm,
                task: &task,
                policy: &policy,
                dependencies,
                verification,
                changes_requested: state.change_requests.get(id).cloned(),
            });
            let unattended = farm.autonomy >= Autonomy::Assisted;
            state.leases.acquire(&task.id, &task.allowed_paths)?;
            state.verified_commits.remove(id);
            state.reviewed_commits.remove(id);
            (task, prompt, unattended)
        };

        let destination = task
            .merge_target
            .clone()
            .map(Ok)
            .unwrap_or_else(|| evidence::branch(&self.repo))?;
        self.set_status(id, TaskStatus::Assigned, None)?;

        // The worktree is cut from the branch the user is on, so a farm run
        // starts from what they can see rather than from `main` on a checkout
        // that has moved on.
        let workspace = match workspace::create(&self.repo, id, definition.provider, &destination) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.state.lock().expect("farm lock").leases.release(id);
                return Err(error);
            }
        };

        self.emit(FarmEvent::WorkspaceChanged {
            task: id.clone(),
            path: workspace.path.to_string_lossy().into_owned(),
            created: true,
        });

        self.edit_task(id, |task| {
            if task.merge_target.is_none() {
                task.merge_target = Some(destination);
            }
            task.branch = Some(workspace.branch.clone());
            task.worktree = Some(workspace.path.to_string_lossy().into_owned());
            task.assigned_agent = Some(definition.id.clone());
            task.implemented_by = Some(definition.id.clone());
            task.attempts += 1;
        })?;

        let run = RunId::new(format!("{}-{}", id.as_str().to_lowercase(), now_ms()));
        let command = adapter_for(definition.provider).command(
            &definition,
            &AgentRunRequest {
                workdir: workspace.path.clone(),
                prompt,
                unattended,
            },
        );

        self.spawn_run(Launch {
            run,
            task: task.id,
            phase: RunPhase::Implementation,
            command,
            workdir: workspace.path,
            narrator: adapter_for(definition.provider).narrator(),
            agent: definition.id,
        })
    }

    /// Start a process and the thread that waits for it.
    fn spawn_run(&self, launch: Launch) -> Result<()> {
        let Launch {
            run,
            task,
            agent,
            phase,
            command,
            workdir,
            narrator,
        } = launch;
        let log = execution::log::log_path(&self.repo, &task, &run);
        let transcript = TranscriptWriter::create(&log)?;

        // One clock per run, shared between the sink that writes it and the
        // state that reads it (FEAT-077).
        let heard = Arc::new(AtomicU64::new(0));
        let sink = Arc::new(RunSink {
            observer: self.observer.clone(),
            run: run.clone(),
            task: task.clone(),
            heard: heard.clone(),
        });

        let session = execution::start(&command, &workdir, transcript, sink, narrator)?;

        {
            let mut state = self.state.lock().expect("farm lock");
            state.remember_run(AgentRun {
                id: run.clone(),
                task: task.clone(),
                agent: agent.clone(),
                phase,
                outcome: RunOutcome::Running,
                command: std::iter::once(command.program.to_string_lossy().into_owned())
                    .chain(command.args.iter().cloned())
                    .collect(),
                started_ms: now_ms(),
                ended_ms: None,
                log_file: Some(log.to_string_lossy().into_owned()),
                last_output_ms: None,
            });
            state.heard.insert(run.clone(), heard);
        }

        self.emit(FarmEvent::AgentStarted {
            run: run.clone(),
            task: task.clone(),
            agent: agent.clone(),
            command: command.display(),
        });
        // Only an implementation run moves the task. A review run has an agent
        // executing against a task that is *in review*, and saying `Running`
        // there would both lie about what is happening and undo the status the
        // reviewer was asked for.
        if phase == RunPhase::Implementation {
            self.set_status(&task, TaskStatus::Running, None)?;
        }

        self.cancellations
            .lock()
            .expect("cancellation lock")
            .insert(task.clone(), (run.clone(), session.cancellation()));
        self.sessions.lock().expect("sessions lock").insert(
            run.clone(),
            PendingRun {
                task: task.clone(),
                run,
                phase,
                session,
            },
        );
        Ok(())
    }

    /// Wait for a task's agent to finish, then take it through the pipeline.
    ///
    /// Separate from [`Self::spawn_run`] and *blocking*, so the caller decides
    /// where the waiting happens. The Tauri layer puts it on a thread; the
    /// tests call it directly, which is what makes the whole pipeline testable
    /// without a scheduler thread to synchronise against.
    pub fn await_task(&self, id: &TaskId) -> Result<()> {
        let run = self
            .sessions
            .lock()
            .expect("sessions lock")
            .values()
            .find(|pending| &pending.task == id)
            .map(|pending| pending.run.clone());
        let Some(run) = run else {
            return Ok(());
        };
        let pending = self.sessions.lock().expect("sessions lock").remove(&run);
        if let Some(pending) = pending {
            self.await_claimed(pending)?;
        }
        Ok(())
    }

    fn await_claimed(&self, pending: PendingRun) -> Result<()> {
        let ended = pending.session.wait();
        let result = self.finish_run(&pending.task, &pending.run, ended);
        self.forget_cancellation(&pending.task, &pending.run);
        result
    }

    /// Claim before spawning. Concurrent callers cannot create duplicate
    /// waiters, and review runs are discovered by phase rather than UI status.
    pub fn watch_pending(self: &Arc<Self>) -> usize {
        let mut started = 0;
        let runs: Vec<RunId> = self
            .sessions
            .lock()
            .expect("sessions lock")
            .values()
            .filter(|pending| pending.phase != RunPhase::Planning)
            .map(|pending| pending.run.clone())
            .collect();
        for run in runs {
            let pending = self.sessions.lock().expect("sessions lock").remove(&run);
            let Some(pending) = pending else {
                continue;
            };
            let task = pending.task.clone();
            let service = self.clone();
            let report_task = task.clone();
            let report_run = run.clone();
            let spawned = std::thread::Builder::new()
                .name(format!("spagitty-farm-{run}"))
                .spawn(move || {
                    if let Err(error) = service.await_claimed(pending) {
                        service.report_run_error(&report_task, &report_run, error.to_string());
                    }
                    service.tick();
                    service.watch_pending();
                });
            if let Err(error) = spawned {
                self.forget_cancellation(&task, &run);
                self.report_run_error(&task, &run, format!("Could not watch the run: {error}"));
            } else {
                started += 1;
            }
        }
        started
    }

    fn report_run_error(&self, task: &TaskId, run: &RunId, message: String) {
        let current = self
            .state
            .lock()
            .expect("farm lock")
            .runs
            .iter()
            .rev()
            .find(|entry| &entry.task == task)
            .map(|entry| entry.id.clone());
        if current.as_ref() != Some(run) {
            return;
        }
        if self
            .farm()
            .and_then(|farm| farm.task(task).map(|task| task.status.is_terminal()))
            .unwrap_or(true)
        {
            return;
        }
        let _ = self.set_status(task, TaskStatus::Blocked, Some(message));
    }

    /// Record how a run ended and decide what happens next.
    fn finish_run(&self, id: &TaskId, run_id: &RunId, ended: execution::Ended) -> Result<()> {
        let ended = if self
            .farm()
            .and_then(|farm| farm.task(id).map(|task| task.status))
            == Some(TaskStatus::Cancelled)
        {
            execution::Ended::Cancelled
        } else {
            ended
        };
        let (run, agent, phase, duration) = {
            let mut state = self.state.lock().expect("farm lock");
            if state
                .runs
                .iter()
                .rev()
                .find(|run| &run.task == id)
                .map(|run| &run.id)
                != Some(run_id)
            {
                return Ok(());
            }
            let run = state
                .runs
                .iter_mut()
                .rev()
                .find(|run| &run.id == run_id && run.outcome == RunOutcome::Running);
            let Some(run) = run else {
                return Ok(());
            };
            run.ended_ms = Some(now_ms());
            run.outcome = match &ended {
                execution::Ended::Ok => RunOutcome::Completed { exit_code: 0 },
                execution::Ended::Failed { code, message } => RunOutcome::Failed {
                    exit_code: *code,
                    reason: message.clone(),
                },
                execution::Ended::Cancelled => RunOutcome::Cancelled,
            };
            (
                run.id.clone(),
                run.agent.clone(),
                run.phase,
                run.duration_ms().unwrap_or(0),
            )
        };

        self.emit(FarmEvent::AgentStopped {
            run: run.clone(),
            task: id.clone(),
            ok: ended == execution::Ended::Ok,
            reason: match &ended {
                execution::Ended::Failed { message, .. } => Some(message.clone()),
                execution::Ended::Cancelled => Some("Stopped by hand.".into()),
                execution::Ended::Ok => None,
            },
        });

        if ended == execution::Ended::Cancelled {
            let mut state = self.state.lock().expect("farm lock");
            state.leases.release(id);
            state.review_inputs.remove(run_id);
            return Ok(());
        }

        let transcript = execution::log::read(&execution::log::log_path(&self.repo, id, &run));

        match phase {
            RunPhase::Review => {
                if let execution::Ended::Failed { message, .. } = &ended {
                    {
                        let mut state = self.state.lock().expect("farm lock");
                        state.reviewed_commits.remove(id);
                        state.review_inputs.remove(run_id);
                    }
                    return self.after_rejection(id, format!("Reviewer failed: {message}"));
                }
                self.conclude_review(id, run_id, &agent, &transcript)
            }
            RunPhase::Planning => Ok(()),
            RunPhase::Implementation => {
                let handoff = Handoff::parse(&transcript);
                {
                    let mut state = self.state.lock().expect("farm lock");
                    state.handoffs.insert(id.clone(), handoff.clone());
                    if ended == execution::Ended::Ok {
                        state.scoreboard.completed(&agent, duration);
                    } else {
                        state.scoreboard.failed(&agent, duration);
                    }
                }
                self.propose_tasks(id, &handoff);

                if let execution::Ended::Failed { message, .. } = &ended {
                    self.state.lock().expect("farm lock").leases.release(id);
                    return self.set_status(id, TaskStatus::Failed, Some(message.clone()));
                }
                if handoff.is_blocked() {
                    self.state.lock().expect("farm lock").leases.release(id);
                    let why = if handoff.summary.trim().is_empty() {
                        "The agent reported it could not finish.".to_string()
                    } else {
                        handoff.summary.clone()
                    };
                    return self.set_status(id, TaskStatus::Blocked, Some(why));
                }
                self.verify(id)
            }
        }
    }

    // ── Verification ─────────────────────────────────────────────────────

    /// Run the configured checks against the task's worktree.
    ///
    /// Blocking, like [`Self::await_task`], and for the same reason.
    pub fn verify(&self, id: &TaskId) -> Result<()> {
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut stops = self
                .verification_stops
                .lock()
                .expect("verification stop lock");
            if stops.contains_key(id) {
                return Err(Error::Refused("Verification is already running.".into()));
            }
            stops.insert(id.clone(), stop.clone());
        }
        let result = self.verify_with_stop(id, &stop);
        self.verification_stops
            .lock()
            .expect("verification stop lock")
            .remove(id);
        result
    }

    fn verify_with_stop(&self, id: &TaskId, stop: &AtomicBool) -> Result<()> {
        let (workdir, commands) = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm.task(id).ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            let workdir = task
                .worktree
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace::worktree_path(&self.repo, id));
            (workdir, verifier::commands_for(&farm.verification, task))
        };

        self.set_status(id, TaskStatus::Verification, None)?;
        let before = evidence::clean_commit(&workdir).ok();
        {
            let mut state = self.state.lock().expect("farm lock");
            state.verified_commits.remove(id);
            state.reviewed_commits.remove(id);
            state.reviews.remove(id);
        }

        // Through `emit`, not straight to the observer. These two used to go
        // directly to the interface, which meant a verification was on screen
        // while it happened and in no record afterwards: it was in neither the
        // log on disk nor the history a reopened farm reads back.
        let task_id = id.clone();
        let mut starting = |command: &str| {
            self.emit(FarmEvent::VerificationStarted {
                task: task_id.clone(),
                command: command.to_string(),
            });
        };
        let mut report = |result: &CommandResult| {
            self.emit(FarmEvent::VerificationFinished {
                task: task_id.clone(),
                command: result.command.clone(),
                passed: result.passed,
                output: result.output.clone(),
            });
        };
        let verification =
            verifier::run_cancellable(&workdir, &commands, stop, &mut starting, &mut report);
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }

        self.state
            .lock()
            .expect("farm lock")
            .verifications
            .insert(id.clone(), verification.clone());

        if verification.passed {
            if let Some(commit) = before {
                if evidence::clean_commit(&workdir).ok().as_ref() == Some(&commit) {
                    self.state
                        .lock()
                        .expect("farm lock")
                        .verified_commits
                        .insert(
                            id.clone(),
                            evidence::CheckedCommit {
                                commit,
                                commands: commands.clone(),
                            },
                        );
                }
            }
        }
        if !verification.passed && !verification.unverified {
            return self.after_rejection(id, verification.summary());
        }
        self.request_review(id)
    }

    // ── Review ───────────────────────────────────────────────────────────

    /// Send a verified task to a different agent.
    ///
    /// A farm below `SemiAuto`, or with nobody else installed, stops here with
    /// the task in `Review` and waits for a person. That is not a failure: the
    /// human *is* the reviewer at those levels.
    pub fn request_review(&self, id: &TaskId) -> Result<()> {
        self.set_status(id, TaskStatus::Review, None)?;

        let (autonomy, implementer, task) = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm
                .task(id)
                .ok_or_else(|| Error::NoSuchTask(id.clone()))?
                .clone();
            (farm.autonomy, task.implemented_by.clone(), task)
        };

        if !autonomy.reviews() {
            return Ok(());
        }
        let Some(implementer) = implementer else {
            return Ok(());
        };
        let Some(reviewer) =
            reviewer::pick(&self.registry.lock().expect("registry lock"), &implementer)
        else {
            return Ok(());
        };
        reviewer::check(&implementer, &reviewer.id)?;

        let workdir = task
            .worktree
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace::worktree_path(&self.repo, id));

        let prompt = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let policy = policy::read(&self.repo);
            context::review(
                &Context {
                    farm,
                    task: &task,
                    policy: &policy,
                    dependencies: Vec::new(),
                    verification: Vec::new(),
                    changes_requested: None,
                },
                &self.diff_summary(&workdir),
            )
        };

        self.emit(FarmEvent::ReviewRequested {
            task: id.clone(),
            reviewer: reviewer.id.clone(),
        });

        let run = RunId::new(format!(
            "{}-review-{}",
            id.as_str().to_lowercase(),
            now_ms()
        ));
        if let Ok(commit) = evidence::clean_commit(&workdir) {
            self.state
                .lock()
                .expect("farm lock")
                .review_inputs
                .insert(run.clone(), commit);
        }
        let command = adapter_for(reviewer.provider).command(
            &reviewer,
            &AgentRunRequest {
                workdir: workdir.clone(),
                prompt,
                unattended: true,
            },
        );
        // `spawn_run` sets the task to Running, which is what the interface
        // should show: an agent is executing against this task.
        self.spawn_run(Launch {
            run,
            task: id.clone(),
            phase: RunPhase::Review,
            command,
            workdir,
            narrator: adapter_for(reviewer.provider).narrator(),
            agent: reviewer.id,
        })
    }

    fn conclude_review(
        &self,
        id: &TaskId,
        run: &RunId,
        reviewer: &AgentId,
        transcript: &str,
    ) -> Result<()> {
        let input = self
            .state
            .lock()
            .expect("farm lock")
            .review_inputs
            .remove(run);
        let review = Review::parse(transcript);
        self.state
            .lock()
            .expect("farm lock")
            .reviews
            .insert(id.clone(), review.clone());

        self.emit(FarmEvent::ReviewCompleted {
            task: id.clone(),
            reviewer: reviewer.clone(),
            approved: review.approved(),
            summary: review.summary.clone(),
        });

        if !review.approved() {
            let request = review.change_request();
            self.state
                .lock()
                .expect("farm lock")
                .change_requests
                .insert(id.clone(), request.clone());
            if let Some(implementer) = self
                .farm()
                .and_then(|farm| farm.task(id).and_then(|task| task.implemented_by.clone()))
            {
                self.state
                    .lock()
                    .expect("farm lock")
                    .scoreboard
                    .changes_requested(&implementer);
            }
            return self.after_rejection(id, review.summary.clone());
        }

        let task = self
            .farm()
            .and_then(|farm| farm.task(id).cloned())
            .ok_or_else(|| Error::NoSuchTask(id.clone()))?;
        if let Some(implementer) = &task.implemented_by {
            reviewer::check(implementer, reviewer)?;
        }
        if let (Some(commit), Some(path)) = (input, &task.worktree) {
            if evidence::clean_commit(Path::new(path)).ok().as_ref() == Some(&commit) {
                self.state
                    .lock()
                    .expect("farm lock")
                    .reviewed_commits
                    .insert(
                        id.clone(),
                        evidence::ReviewedCommit {
                            commit,
                            reviewer: reviewer.clone(),
                        },
                    );
            }
        }
        self.approve(id)
    }

    /// Accept a task: merge if the farm may, otherwise wait for a person.
    pub fn approve(&self, id: &TaskId) -> Result<()> {
        let autonomy = self.farm().map(|farm| farm.autonomy).unwrap_or_default();
        if autonomy.merges() {
            match self.merge_checked(id, true) {
                Ok(()) => Ok(()),
                Err(error) => self.set_status(id, TaskStatus::Review, Some(error.to_string())),
            }
        } else {
            // Stays in Review with nothing running: the human's queue.
            Ok(())
        }
    }

    /// Merge a task's branch into the branch the user is on, then clean up.
    pub fn merge(&self, id: &TaskId) -> Result<()> {
        self.merge_checked(id, false)
    }

    fn merge_checked(&self, id: &TaskId, automatic: bool) -> Result<()> {
        let merge_guard = self.merge_lock.lock().expect("merge lock");
        let (task, provider) = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm.task(id).ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            if task.status != TaskStatus::Review
                || state
                    .runs
                    .iter()
                    .any(|run| &run.task == id && run.outcome == RunOutcome::Running)
            {
                return Err(Error::Refused(
                    "Wait for the task and its review to finish before merging.".into(),
                ));
            }
            (task.clone(), self.provider_of(task))
        };
        let branch = task
            .branch
            .clone()
            .ok_or_else(|| Error::Refused("This task has no branch to merge.".into()))?;
        let workdir = task
            .worktree
            .as_deref()
            .map(Path::new)
            .ok_or_else(|| Error::Refused("This task has no worktree.".into()))?;
        let commit = evidence::clean_commit(workdir)?;
        let destination = evidence::branch(&self.repo)?;
        if let Some(expected) = &task.merge_target {
            if expected != &destination {
                return Err(Error::Refused(format!(
                    "Switch back to {expected} before merging this task."
                )));
            }
        } else if automatic {
            return Err(Error::Refused(
                "This older task has no recorded merge destination; merge it by hand.".into(),
            ));
        }
        if automatic {
            self.check_merge_evidence(&task, &commit)?;
        }

        self.emit(FarmEvent::MergeRequested {
            task: id.clone(),
            branch: branch.clone(),
        });

        // `--no-ff`, always. A farm branch that fast-forwards disappears from
        // the history, and "which agent wrote this" stops having an answer.
        match shell::merge_into(&self.repo, &commit, &destination) {
            Ok(()) => {
                self.emit(FarmEvent::MergeCompleted {
                    task: id.clone(),
                    branch: branch.clone(),
                    ok: true,
                    error: None,
                });
                self.set_status(id, TaskStatus::Done, None)?;
                let _ = workspace::remove(&self.repo, id, provider, false);
                self.emit(FarmEvent::WorkspaceChanged {
                    task: id.clone(),
                    path: workspace::worktree_path(&self.repo, id)
                        .to_string_lossy()
                        .into_owned(),
                    created: false,
                });
                drop(merge_guard);
                self.tick();
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                self.emit(FarmEvent::MergeCompleted {
                    task: id.clone(),
                    branch,
                    ok: false,
                    error: Some(message.clone()),
                });
                // A conflict is never resolved silently by an agent. The task
                // is blocked, the conflict is in the working copy, and the
                // Conflicts screen is where it is dealt with.
                self.set_status(
                    id,
                    TaskStatus::Blocked,
                    Some(format!("The merge did not apply cleanly. {message}")),
                )
            }
        }
    }

    fn check_merge_evidence(&self, task: &Task, commit: &str) -> Result<()> {
        let state = self.state.lock().expect("farm lock");
        let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
        if !farm.autonomy.merges() || !farm.permissions.merge {
            return Err(Error::Refused(
                "Automatic merging is not enabled. Choose the autonomy level again to allow it."
                    .into(),
            ));
        }
        let checked = state.verified_commits.get(&task.id).filter(|checked| {
            checked.commit == commit
                && checked.commands == verifier::commands_for(&farm.verification, task)
        });
        if checked.is_none() {
            return Err(Error::Refused(
                "Run passing verification checks for the current commit before automatic merging."
                    .into(),
            ));
        }
        let reviewed = state
            .reviewed_commits
            .get(&task.id)
            .filter(|review| review.commit == commit);
        let Some(reviewed) = reviewed else {
            return Err(Error::Refused(
                "The current commit needs a successful independent review.".into(),
            ));
        };
        let implementer = task
            .implemented_by
            .as_ref()
            .ok_or_else(|| Error::Refused("The task's implementer is unknown.".into()))?;
        reviewer::check(implementer, &reviewed.reviewer)?;
        Ok(())
    }

    /// A verification failure or a requested change: try again, or give up.
    fn after_rejection(&self, id: &TaskId, why: String) -> Result<()> {
        let (exhausted, autonomy) = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm.task(id).ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            (task.is_exhausted_at(farm.max_attempts), farm.autonomy)
        };

        self.state.lock().expect("farm lock").leases.release(id);

        if exhausted || !autonomy.retries() {
            // The default. A red build is where a person comes back, and a farm
            // that quietly rewrites it three times has spent money to produce
            // the same failure with a different diff.
            return self.set_status(id, TaskStatus::Blocked, Some(why));
        }
        self.set_status(id, TaskStatus::Waiting, Some(why))?;
        self.tick();
        Ok(())
    }

    // ── Planning ─────────────────────────────────────────────────────────

    /// Ask an agent to break the goal into tasks.
    ///
    /// Returns the run identifier; the plan arrives when the run finishes and
    /// is read with [`Self::collect_plan`]. Two steps rather than one because
    /// the planning run streams like any other and the interface shows it.
    pub fn plan(&self, agent: Option<AgentId>) -> Result<RunId> {
        let prompt = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            context::planning(farm, &policy::read(&self.repo))
        };
        self.start_planning(prompt, None, agent)
    }

    /// Ask an agent to break one task into smaller ones (FEAT-076).
    ///
    /// The same machinery as [`Self::plan`] pointed at a task instead of the
    /// goal. The children arrive as drafts, and the task they came out of
    /// becomes a container the moment they are accepted.
    pub fn decompose(&self, id: &TaskId, agent: Option<AgentId>) -> Result<RunId> {
        let prompt = {
            let state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            let task = farm.task(id).ok_or_else(|| Error::NoSuchTask(id.clone()))?;
            if task.status.is_terminal() {
                return Err(Error::Refused(
                    "That task is finished. Breaking it down would produce work nobody asked for."
                        .into(),
                ));
            }
            context::decomposition(farm, task, &policy::read(&self.repo))
        };
        self.start_planning(prompt, Some(id.clone()), agent)
    }

    /// Start a planning run, whatever it is planning.
    fn start_planning(
        &self,
        prompt: String,
        parent: Option<TaskId>,
        agent: Option<AgentId>,
    ) -> Result<RunId> {
        let definition = {
            let registry = self.registry.lock().expect("registry lock");
            match agent {
                Some(id) => registry.runnable(&id)?,
                None => registry
                    .for_role(AgentRole::Architect, AgentCapability::Planning)
                    .ok_or_else(|| Error::AgentUnavailable("no planning agent".into()))?,
            }
        };

        // One planning run at a time. Two would share the `planning` session
        // slot and the second would collect the first one's transcript, which
        // is the kind of fault that shows up as a plan that makes no sense.
        {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_ref().ok_or(Error::NoFarm)?;
            if farm.status == FarmStatus::Planning {
                return Err(Error::Refused(
                    "Something is already being planned. Wait for it, or stop it.".into(),
                ));
            }
            state.planning_parent = parent;
        }

        self.set_farm_status(FarmStatus::Planning)?;

        let planning_task = TaskId::new("planning");
        let run = RunId::new(format!("plan-{}", now_ms()));
        let command = adapter_for(definition.provider).command(
            &definition,
            &AgentRunRequest {
                workdir: self.repo.clone(),
                prompt,
                unattended: false,
            },
        );

        let log = execution::log::log_path(&self.repo, &planning_task, &run);
        let transcript = TranscriptWriter::create(&log)?;
        let heard = Arc::new(AtomicU64::new(0));
        let sink = Arc::new(RunSink {
            observer: self.observer.clone(),
            run: run.clone(),
            task: planning_task.clone(),
            heard: heard.clone(),
        });
        // The planner runs in the repository itself, read-only. It is told not
        // to change anything, and it has no worktree of its own because it is
        // not producing a change to review.
        let session = execution::start(
            &command,
            &self.repo,
            transcript,
            sink,
            adapter_for(definition.provider).narrator(),
        )?;

        self.emit(FarmEvent::AgentStarted {
            run: run.clone(),
            task: planning_task.clone(),
            agent: definition.id.clone(),
            command: command.display(),
        });

        {
            let mut state = self.state.lock().expect("farm lock");
            state.remember_run(AgentRun {
                id: run.clone(),
                task: planning_task.clone(),
                agent: definition.id,
                phase: RunPhase::Planning,
                outcome: RunOutcome::Running,
                command: vec![command.display()],
                started_ms: now_ms(),
                ended_ms: None,
                log_file: Some(log.to_string_lossy().into_owned()),
                last_output_ms: None,
            });
            state.heard.insert(run.clone(), heard);
        }
        self.cancellations
            .lock()
            .expect("cancellation lock")
            .insert(planning_task.clone(), (run.clone(), session.cancellation()));
        self.sessions.lock().expect("sessions lock").insert(
            run.clone(),
            PendingRun {
                task: planning_task,
                run: run.clone(),
                phase: RunPhase::Planning,
                session,
            },
        );
        Ok(run)
    }

    /// Wait for the planning run and turn what it said into draft tasks.
    ///
    /// The tasks are `Draft`: the user approves the plan before anything runs,
    /// which is what the plan asks for and what stops a bad decomposition
    /// becoming five agent runs.
    pub fn collect_plan(&self, run: &RunId) -> Result<Vec<Task>> {
        let planning_task = TaskId::new("planning");
        // Taken out of the map on its own line, deliberately. Written as
        // `if let Some(session) = self.sessions.lock()…remove(&planning_task)`
        // the guard is a temporary of the scrutinee, so on edition 2021 it
        // lives to the end of the `if let` — and `session.wait()` then holds
        // the sessions lock for the whole planning run. Every command that
        // starts, stops or schedules a task takes that lock, and they run on
        // the main thread, so the window froze until the planner finished
        // (BUG-020). `await_task` has always had the shape this now copies.
        let waiting = self.sessions.lock().expect("sessions lock").remove(run);
        let ended = match waiting {
            Some(pending) => pending.session.wait(),
            // Nothing to wait for: the run is already over, or was never
            // started. Treated as a clean end so the transcript is still read.
            None => execution::Ended::Ok,
        };

        self.forget_cancellation(&planning_task, run);
        let outcome = match &ended {
            execution::Ended::Ok => RunOutcome::Completed { exit_code: 0 },
            execution::Ended::Cancelled => RunOutcome::Cancelled,
            execution::Ended::Failed { code, message } => RunOutcome::Failed {
                exit_code: *code,
                reason: message.clone(),
            },
        };
        {
            let mut state = self.state.lock().expect("farm lock");
            if let Some(entry) = state.runs.iter_mut().find(|entry| &entry.id == run) {
                entry.outcome = outcome;
                entry.ended_ms = Some(now_ms());
            }
        }

        self.emit(FarmEvent::AgentStopped {
            run: run.clone(),
            task: planning_task.clone(),
            ok: ended == execution::Ended::Ok,
            reason: match &ended {
                execution::Ended::Failed { message, .. } => Some(message.clone()),
                execution::Ended::Cancelled => Some("Stopped by hand.".into()),
                execution::Ended::Ok => None,
            },
        });

        // A cancelled planner has said half of something. Adopting that would
        // put an arbitrary prefix of a decomposition into the plan, which is
        // worse than nothing: the tasks look deliberate.
        if ended == execution::Ended::Cancelled {
            self.state.lock().expect("farm lock").planning_parent = None;
            self.set_farm_status(FarmStatus::Idle)?;
            return Ok(Vec::new());
        }

        let transcript =
            execution::log::read(&execution::log::log_path(&self.repo, &planning_task, run));
        let plan = planner::Plan::parse(&transcript);

        let tasks = {
            let mut state = self.state.lock().expect("farm lock");
            let parent = state.planning_parent.take();
            // Which agent produced this plan, from the run itself rather than
            // remembered separately: the run is the record of who was asked
            // (FEAT-078).
            let agent = state
                .runs
                .iter()
                .find(|entry| &entry.id == run)
                .map(|entry| entry.agent.clone())
                .unwrap_or_else(|| AgentId::new("an agent"));
            let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
            let tasks = planner::adopt_under(farm, &agent, &plan, parent.as_ref())?;
            farm.tasks.extend(tasks.clone());
            farm.status = FarmStatus::Idle;
            self.persist(&state)?;
            tasks
        };

        // A planning run that produced nothing used to end in silence: the
        // status chip went back to Idle and the plan stayed empty, with no way
        // to tell that from a planner that had not been asked. Whatever went
        // wrong — a refusal, a rate limit, an answer with no plan block — the
        // transcript has it, and the screen now says where to look.
        if tasks.is_empty() {
            self.emit(FarmEvent::Failed {
                message: match &ended {
                    execution::Ended::Failed { message, .. } => {
                        format!("The planner did not finish: {message}")
                    }
                    _ => "The planner produced no tasks. Its transcript says what it did instead."
                        .to_string(),
                },
            });
        }

        for task in &tasks {
            self.emit(FarmEvent::TaskCreated {
                task: task.id.clone(),
                title: task.title.clone(),
            });
        }
        self.emit_farm_status();
        Ok(tasks)
    }

    /// Stop a planning run.
    ///
    /// The session is signalled but *not* removed: the thread inside
    /// [`Self::collect_plan`] owns it and is waiting on it, and it is that
    /// thread which decides what a cancelled plan means. Taking it away here
    /// would leave the collector reading a half-written transcript with nothing
    /// telling it the run had been stopped.
    ///
    /// The lock is held across `cancel`, which sends a signal and returns; it
    /// never waits for the process, which is the distinction BUG-020 turned on.
    pub fn cancel_plan(&self) -> Result<()> {
        let cancellation = self
            .cancellations
            .lock()
            .expect("cancellation lock")
            .get(&TaskId::new("planning"))
            .cloned();
        if let Some((_, cancellation)) = cancellation {
            cancellation.cancel();
        }
        Ok(())
    }

    // ── Recovery ─────────────────────────────────────────────────────────

    /// Reconcile the saved farm with what is actually on disk.
    ///
    /// A task recorded as `Running` has no process — this is a fresh start, so
    /// whatever was running died with the last session. Marking it `Waiting`
    /// rather than `Failed` is the honest reading: the work may well be there
    /// in the worktree, and the plan is explicit that a task must never become
    /// `Done` because a process exited.
    fn recover(&self) {
        let stale: Vec<TaskId> = {
            let state = self.state.lock().expect("farm lock");
            let Some(farm) = state.farm.as_ref() else {
                return;
            };
            farm.tasks
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::Running | TaskStatus::Assigned))
                .map(|task| task.id.clone())
                .collect()
        };

        for id in stale {
            let _ = self.set_status(
                &id,
                TaskStatus::Waiting,
                Some("Spagitty restarted while this was running.".into()),
            );
        }

        // A farm that was running when the application closed does not resume
        // on its own. Restarting into four agent runs the user did not ask for
        // is not a recovery, it is a surprise.
        let mut state = self.state.lock().expect("farm lock");
        if let Some(farm) = state.farm.as_mut() {
            if farm.status.is_live() {
                farm.status = FarmStatus::Paused;
            }
        }
        let _ = self.persist(&state);
    }

    /// Worktrees left behind by tasks no farm claims.
    pub fn stale_workspaces(&self) -> Vec<workspace::cleanup::Stale> {
        let known: Vec<TaskId> = self
            .farm()
            .map(|farm| farm.tasks.iter().map(|task| task.id.clone()).collect())
            .unwrap_or_default();
        workspace::cleanup::farm_worktrees(&self.repo)
            .into_iter()
            .filter(|stale| !known.contains(&stale.task))
            .collect()
    }

    /// Remove them.
    pub fn sweep(&self) -> Result<Vec<workspace::cleanup::Stale>> {
        let known: Vec<TaskId> = self
            .farm()
            .map(|farm| farm.tasks.iter().map(|task| task.id.clone()).collect())
            .unwrap_or_default();
        workspace::cleanup::sweep(&self.repo, &known)
    }

    // ── Plumbing ─────────────────────────────────────────────────────────

    fn set_farm_status(&self, status: FarmStatus) -> Result<()> {
        {
            let mut state = self.state.lock().expect("farm lock");
            let farm = state.farm.as_mut().ok_or(Error::NoFarm)?;
            farm.status = status;
            farm.updated_ms = now_ms();
            self.persist(&state)?;
        }
        self.emit(FarmEvent::FarmStatusChanged { status });
        Ok(())
    }

    /// Recompute and announce the farm's status if it has moved.
    fn emit_farm_status(&self) {
        let derived = {
            let mut state = self.state.lock().expect("farm lock");
            let Some(farm) = state.farm.as_mut() else {
                return;
            };
            let derived = farm.derived_status();
            if derived == farm.status {
                return;
            }
            farm.status = derived;
            let _ = self.persist(&state);
            derived
        };
        self.emit(FarmEvent::FarmStatusChanged { status: derived });
    }

    /// Save, and append the event.
    ///
    /// Takes the guard rather than locking again: every caller already holds
    /// it, and re-entering the mutex would deadlock.
    fn persist(&self, state: &State) -> Result<()> {
        let Some(farm) = state.farm.as_ref() else {
            return Ok(());
        };
        store::save_farm(&self.repo, farm)?;
        let _ = store::trim_events(&self.repo);
        Ok(())
    }

    fn emit(&self, event: FarmEvent) {
        let event = Recorded::now(event);
        let _ = store::append_event(&self.repo, &event);
        // Transcript lines are deliberately not kept, here or on disk: one run
        // produces thousands, and they would push the history out within a
        // single agent run. They reach the interface as events and are read
        // back from the run's own log.
        if !event.is_transcript() {
            let mut recent = self.recent.lock().expect("recent events lock");
            recent.push_back(event.clone());
            while recent.len() > store::MAX_EVENTS {
                recent.pop_front();
            }
        }
        self.observer.event(event);
    }

    /// Turn an agent's proposals into draft tasks.
    fn propose_tasks(&self, from: &TaskId, handoff: &Handoff) {
        for proposal in &handoff.proposed_tasks {
            if proposal.title.trim().is_empty() {
                continue;
            }
            let created = {
                let mut state = self.state.lock().expect("farm lock");
                let Some(farm) = state.farm.as_mut() else {
                    continue;
                };
                // The agent that was working when it thought of this, so a
                // proposal is attributable rather than anonymous (FEAT-078).
                let agent = farm.task(from).and_then(|task| task.implemented_by.clone());
                let task = planner::from_proposal(farm, proposal, from, agent.as_ref());
                farm.tasks.push(task.clone());
                let _ = self.persist(&state);
                task
            };
            self.emit(FarmEvent::TaskProposed {
                from: from.clone(),
                title: created.title.clone(),
            });
        }
    }

    /// What the tasks this one depends on concluded.
    fn dependency_results(
        &self,
        farm: &Farm,
        handoffs: &HashMap<TaskId, Handoff>,
        task: &Task,
    ) -> Vec<DependencyResult> {
        task.depends_on
            .iter()
            .filter_map(|id| farm.task(id))
            .map(|dependency| {
                let handoff = handoffs.get(&dependency.id);
                DependencyResult {
                    task: dependency.id.to_string(),
                    title: dependency.title.clone(),
                    summary: handoff.map(|h| h.summary.clone()).unwrap_or_default(),
                    files_changed: handoff.map(|h| h.files_changed.clone()).unwrap_or_default(),
                }
            })
            .collect()
    }

    /// A short description of what a branch changed, for the reviewer's prompt.
    ///
    /// `git diff --stat` against the merge base, so the reviewer is told the
    /// size and shape of the change and reads the rest itself. Embedding the
    /// whole diff would be the "dump the repository into the prompt" mistake in
    /// a smaller costume.
    fn diff_summary(&self, workdir: &Path) -> String {
        shell::version(workdir)
            .ok()
            .and_then(|_| {
                std::process::Command::new("git")
                    .current_dir(workdir)
                    .args(["diff", "--stat", "HEAD~1..HEAD"])
                    .output()
                    .ok()
            })
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_default()
    }

    fn provider_of(&self, task: &Task) -> AgentProvider {
        task.branch
            .as_deref()
            .and_then(workspace::provider_of)
            .or_else(|| {
                task.implemented_by.as_ref().and_then(|id| {
                    self.registry
                        .lock()
                        .expect("registry lock")
                        .get(id)
                        .map(|definition| definition.provider)
                })
            })
            .unwrap_or(AgentProvider::Custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FarmId, Goal, GoalId};

    fn run(number: u32, task: u32) -> AgentRun {
        AgentRun {
            id: RunId::new(format!("run-{number}")),
            task: TaskId::new(format!("TASK-{task:04}")),
            agent: AgentId::new("claude"),
            phase: RunPhase::Implementation,
            outcome: RunOutcome::Completed { exit_code: 0 },
            command: vec!["claude".into()],
            started_ms: number as u64,
            ended_ms: Some(number as u64 + 1),
            log_file: None,
            last_output_ms: None,
        }
    }

    /// A state holding a farm with `tasks` numbered from one.
    fn state_with(tasks: u32) -> State {
        let mut farm = Farm::new(
            FarmId::new("f1"),
            "/repo",
            Goal::new(GoalId::new("g1"), "Ship it", 0),
            0,
        );
        farm.tasks = (1..=tasks)
            .map(|number| Task::new(TaskId::new(format!("TASK-{number:04}")), "t", 0))
            .collect();
        State {
            farm: Some(farm),
            ..State::default()
        }
    }

    /// TASK-031 — a long session stops growing.
    #[test]
    fn the_history_is_bounded() {
        let mut state = State::default();
        for number in 0..(MAX_RUNS as u32 * 3) {
            state.remember_run(run(number, number));
        }

        assert!(
            state.runs.len() <= MAX_RUNS,
            "the history grew to {}",
            state.runs.len()
        );
        // And it is the *recent* end that survives.
        assert!(state
            .runs
            .iter()
            .any(|entry| entry.id == RunId::new(format!("run-{}", MAX_RUNS * 3 - 1))));
    }

    /// TASK-031 — and a task the farm still has never loses its newest run.
    ///
    /// Otherwise a task that has been quiet for a day opens on an empty panel
    /// because two hundred other runs happened since.
    #[test]
    fn a_task_the_farm_still_has_keeps_its_newest_run() {
        let mut state = state_with(1);
        state.remember_run(run(0, 1));
        for number in 1..(MAX_RUNS as u32 * 2) {
            // Every other run belongs to a task the farm does not have.
            state.remember_run(run(number, number + 100));
        }

        assert!(
            state
                .runs
                .iter()
                .any(|entry| entry.task == TaskId::new("TASK-0001")),
            "the quiet task lost its only run"
        );
    }

    #[test]
    fn forgetting_a_run_forgets_its_clock_too() {
        // Otherwise the map of "when did this last speak" grows forever beside
        // a list that does not (FEAT-077 meets TASK-031).
        let mut state = State::default();
        for number in 0..(MAX_RUNS as u32 * 2) {
            state.heard.insert(
                RunId::new(format!("run-{number}")),
                Arc::new(AtomicU64::new(1)),
            );
            state.remember_run(run(number, number));
        }

        assert_eq!(state.heard.len(), state.runs.len());
    }
}

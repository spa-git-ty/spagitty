// SPDX-License-Identifier: GPL-3.0-or-later

//! The Farm's Tauri layer.
//!
//! Thin, like [`crate::commands`]: it holds the open farm, forwards to
//! `spagitty-farm`, and turns the crate's events into ones the webview can
//! subscribe to. No orchestration logic lives here — the plan's rule that
//! "Tauri commands should call services, not implement business logic" is what
//! keeps the farm testable without a window.
//!
//! # Why this is its own file rather than more of `commands.rs`
//!
//! `commands.rs` is already the longest file in the crate, and the farm adds
//! about thirty commands to it. More usefully: the farm has state of its own
//! with a different lifetime from the graph session — it survives a repository
//! being closed and reopened, because it is stored in the repository — so
//! keeping it in a separate managed state rather than inside `Session` is what
//! stops the two getting entangled.
//!
//! # Where the waiting happens
//!
//! An agent run takes minutes. Nothing here waits for one: a command starts a
//! run and returns, a thread waits for it, and the webview learns what happened
//! from events. That is the same shape as the clone, rebase and network
//! workers, and it is why the window does not freeze while a farm runs.
//!
//! # Why every command is `#[tauri::command(async)]`
//!
//! A plain `#[tauri::command]` is `ExecutionContext::Blocking`: Tauri runs it
//! on the main thread, so anything it waits on — a mutex another thread holds,
//! a `git` subprocess, a file — is time the window is not painting. Returning
//! quickly is not enough, because "quickly" here depends on what a lock's
//! current holder is doing, and one of them used to be a whole planning run
//! (BUG-020). `(async)` puts these on the runtime's pool instead, where a
//! command that does have to wait costs a thread rather than the window.
//!
//! They stay ordinary synchronous functions: an `async fn` may not borrow
//! `State`, and there is nothing here to await.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use spagitty_farm::agent::AgentStatus;
use spagitty_farm::model::*;
use spagitty_farm::orchestrator::Record;
use spagitty_farm::persistence::store;
use spagitty_farm::policy::Policy;
use spagitty_farm::review::Review;
use spagitty_farm::service::{FarmService, Observer};
use spagitty_farm::verification::Verification;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// The one event the Farm screen subscribes to.
pub const EVENT: &str = "farm-event";

/// Turns farm events into Tauri events.
///
/// Held by the service, which is held by the managed state, which lives as long
/// as the application — so the handle inside it never dangles.
#[derive(Debug)]
struct Emit<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> Observer for Emit<R> {
    fn event(&self, event: Recorded) {
        // A webview that has gone away is not an error worth propagating into
        // the farm: the run continues and the events are on disk.
        let _ = self.app.emit(EVENT, &event);
    }
}

/// The open farm, if a repository is open.
#[derive(Default)]
pub struct FarmState {
    service: Mutex<Option<Arc<FarmService>>>,
    /// The repository the service belongs to, so opening the same one twice
    /// does not throw away a farm mid-run.
    path: Mutex<Option<PathBuf>>,
}

impl FarmState {
    fn service(&self) -> Result<Arc<FarmService>> {
        self.service
            .lock()
            .expect("farm service lock")
            .clone()
            .ok_or_else(|| Failure::from(spagitty_farm::Error::NoFarm))
    }
}

/// What the webview gets when something goes wrong.
///
/// The farm's own error type already serialises as `{ kind, message }`, and the
/// kind is what the interface branches on — an unavailable agent gets an
/// install hint, a contended path gets "wait or reassign". A plain string would
/// throw that away.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    kind: String,
    message: String,
}

impl From<spagitty_farm::Error> for Failure {
    fn from(error: spagitty_farm::Error) -> Self {
        Failure {
            kind: error.kind().to_string(),
            message: error.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, Failure>;

/// Everything the Farm screen needs in one round trip.
///
/// One command rather than six, because the screen needs all of it to render
/// anything and six round trips over the Tauri bridge is six chances to render
/// a half-populated screen.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmSnapshot {
    pub farm: Option<Farm>,
    pub agents: Vec<AgentStatus>,
    /// Providers with no definition, so the settings screen can offer them.
    pub undetected: Vec<AgentProvider>,
    /// The tail of the activity history — [`SNAPSHOT_EVENTS`] of it.
    pub events: Vec<Recorded>,
    pub runs: Vec<AgentRun>,
    pub policy: Policy,
    pub scoreboard: Vec<AgentScore>,
    /// Why each queued task is not running, by task.
    ///
    /// Only the reasons the interface cannot work out for itself — a contended
    /// path, a full parallelism limit, no agent for the work. Unmet
    /// dependencies are not here: the screen has the task list.
    pub waiting: HashMap<String, String>,
}

/// How much history a snapshot carries.
///
/// Two hundred: more than the activity list renders, so nothing is missing on
/// screen, and few enough that a refresh after every burst of events stays
/// cheap. Anything older is asked for by name — see [`farm_events`] — because a
/// person scrolling back is a rare act and a refresh is a constant one.
pub const SNAPSHOT_EVENTS: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleWorkspace {
    pub task: TaskId,
    pub path: String,
    pub branch: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentScore {
    pub agent: AgentId,
    pub completed: u32,
    pub failed: u32,
    pub changes_requested: u32,
    /// `null` rather than zero when nothing has been attempted: "no data" and
    /// "always fails" must not look the same.
    pub success_rate: Option<f32>,
    pub average_ms: Option<u64>,
}

impl From<(AgentId, Record)> for AgentScore {
    fn from((agent, record): (AgentId, Record)) -> Self {
        AgentScore {
            agent,
            completed: record.completed,
            failed: record.failed,
            changes_requested: record.changes_requested,
            success_rate: record.success_rate(),
            average_ms: record.average_ms(),
        }
    }
}

/// Everything the task detail panel needs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub verification: Option<Verification>,
    pub review: Option<Review>,
    pub handoff: Option<Handoff>,
    pub runs: Vec<AgentRun>,
}

/// Point the farm at a repository.
///
/// Called by the frontend as a repository opens. Detection runs on a thread of
/// its own: probing four agent CLIs takes long enough to be visible, and it is
/// not something opening a repository should wait for.
#[tauri::command(async)]
pub fn farm_open<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FarmState>,
    path: PathBuf,
) -> Result<FarmSnapshot> {
    let already = state.path.lock().expect("farm path lock").clone();
    if already.as_deref() == Some(path.as_path()) {
        // The same repository. Keeping the service alive matters: closing and
        // reopening a tab must not orphan a running agent.
        return snapshot(&*state.service()?);
    }

    let service = Arc::new(FarmService::open(
        path.clone(),
        Arc::new(Emit { app: app.clone() }),
    ));

    *state.service.lock().expect("farm service lock") = Some(service.clone());
    *state.path.lock().expect("farm path lock") = Some(path);

    std::thread::Builder::new()
        .name("spagitty-farm-detect".into())
        .spawn({
            let service = service.clone();
            move || {
                service.detect_agents();
                // Stamped like every other event, because the webview reads
                // one channel and a line with no time in a log that has times
                // reads as a bug in the log.
                let _ = app.emit(
                    EVENT,
                    &Recorded::now(FarmEvent::FarmStatusChanged {
                        status: service
                            .farm()
                            .map(|farm| farm.status)
                            .unwrap_or(FarmStatus::Idle),
                    }),
                );
            }
        })
        .ok();

    snapshot(&service)
}

#[tauri::command(async)]
pub fn farm_snapshot(state: State<'_, FarmState>) -> Result<FarmSnapshot> {
    snapshot(&*state.service()?)
}

/// More history than a snapshot carries, for a reader scrolling back.
#[tauri::command(async)]
pub fn farm_events(state: State<'_, FarmState>, limit: Option<usize>) -> Result<Vec<Recorded>> {
    Ok(state
        .service()?
        .events_tail(limit.unwrap_or(usize::MAX).min(store::MAX_EVENTS)))
}

/// Worktrees left behind by tasks no farm claims.
///
/// Its own command rather than part of the snapshot, because answering it means
/// running `git worktree list` — and the snapshot is taken after every burst of
/// events, which made watching a farm run a `git` process every quarter of a
/// second (TASK-030). Nothing on the screen shows leftovers *while* a run is in
/// flight; they are read when the farm opens and when the housekeeping panel
/// asks.
#[tauri::command(async)]
pub fn farm_stale(state: State<'_, FarmState>) -> Result<Vec<StaleWorkspace>> {
    Ok(state
        .service()?
        .stale_workspaces()
        .into_iter()
        .map(|stale| StaleWorkspace {
            task: stale.task,
            path: stale.path,
            branch: stale.branch,
        })
        .collect())
}

fn snapshot(service: &FarmService) -> Result<FarmSnapshot> {
    Ok(FarmSnapshot {
        farm: service.farm(),
        agents: service.agents(),
        undetected: service.undetected(),
        events: service.events_tail(SNAPSHOT_EVENTS),
        runs: service.runs(),
        policy: service.policy(),
        scoreboard: service
            .scoreboard()
            .into_iter()
            .map(AgentScore::from)
            .collect(),
        waiting: service
            .waiting_reasons()
            .into_iter()
            .map(|(task, why)| (task.as_str().to_string(), why))
            .collect(),
    })
}

// ── Agents ───────────────────────────────────────────────────────────────

/// Look for installed agents again.
#[tauri::command(async)]
pub fn farm_detect_agents(state: State<'_, FarmState>) -> Result<Vec<AgentStatus>> {
    Ok(state.service()?.detect_agents())
}

#[tauri::command(async)]
pub fn farm_save_agent(state: State<'_, FarmState>, agent: AgentDefinition) -> Result<()> {
    state.service()?.save_agent(agent)?;
    Ok(())
}

#[tauri::command(async)]
pub fn farm_remove_agent(state: State<'_, FarmState>, id: AgentId) -> Result<()> {
    state.service()?.remove_agent(&id)?;
    Ok(())
}

// ── The farm ─────────────────────────────────────────────────────────────

#[tauri::command(async)]
pub fn farm_create(
    state: State<'_, FarmState>,
    title: String,
    description: String,
) -> Result<Farm> {
    Ok(state.service()?.create(&title, &description)?)
}

/// The settings the Farm screen can change.
///
/// One command with an optional field per setting rather than one command each:
/// the screen saves the whole panel, and six commands would be six chances for
/// half of it to land.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmSettings {
    pub autonomy: Option<Autonomy>,
    pub permissions: Option<Permissions>,
    pub max_parallel: Option<usize>,
    pub max_attempts: Option<u32>,
    pub verification: Option<Vec<String>>,
    pub agents: Option<Vec<AgentId>>,
    pub goal_title: Option<String>,
    pub goal_description: Option<String>,
}

#[tauri::command(async)]
pub fn farm_configure(state: State<'_, FarmState>, settings: FarmSettings) -> Result<Farm> {
    Ok(state.service()?.configure(|farm| {
        if let Some(autonomy) = settings.autonomy {
            farm.autonomy = autonomy;
        }
        if let Some(permissions) = settings.permissions {
            farm.permissions = permissions;
        }
        if let Some(attempts) = settings.max_attempts {
            // One at least — a farm that may not attempt anything is a farm
            // that does nothing — and ten at most, which is already more
            // rounds than a task nobody has understood deserves.
            farm.max_attempts = attempts.clamp(1, 10);
        }
        if let Some(max) = settings.max_parallel {
            // One at least, and a ceiling: a farm told to run fifty agents
            // would exhaust the machine and every provider's rate limit at once.
            farm.max_parallel = max.clamp(1, 8);
        }
        if let Some(verification) = settings.verification {
            farm.verification = verification
                .into_iter()
                .map(|command| command.trim().to_string())
                .filter(|command| !command.is_empty())
                .collect();
        }
        if let Some(agents) = settings.agents {
            farm.agents = agents;
        }
        if let Some(title) = settings.goal_title {
            farm.goal.title = title;
        }
        if let Some(description) = settings.goal_description {
            farm.goal.description = description;
        }
    })?)
}

#[tauri::command(async)]
pub fn farm_start(state: State<'_, FarmState>) -> Result<()> {
    let service = state.service()?;
    service.start_farm()?;
    watch_all(&service);
    Ok(())
}

#[tauri::command(async)]
pub fn farm_pause(state: State<'_, FarmState>) -> Result<()> {
    state.service()?.pause_farm()?;
    Ok(())
}

#[tauri::command(async)]
pub fn farm_cancel(state: State<'_, FarmState>) -> Result<()> {
    state.service()?.cancel_farm()?;
    Ok(())
}

/// Write the starter `AGENTS.md`.
#[tauri::command(async)]
pub fn farm_write_policy(state: State<'_, FarmState>) -> Result<String> {
    Ok(state
        .service()?
        .write_policy_template()?
        .to_string_lossy()
        .into_owned())
}

// ── Tasks ────────────────────────────────────────────────────────────────

/// What the task editor sends. A `Task` with the fields the interface owns, and
/// without the ones the service does — identifier, timestamps, branch.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDraft {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: TaskKind,
    #[serde(default)]
    pub priority: TaskPriority,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub verification: Vec<String>,
    #[serde(default)]
    pub verification_overrides: bool,
    #[serde(default)]
    pub assigned_agent: Option<AgentId>,
    /// True to put it straight in the plan rather than leaving it a draft.
    #[serde(default)]
    pub ready: bool,
}

impl TaskDraft {
    fn apply(self, task: &mut Task) {
        task.title = self.title;
        task.description = self.description;
        task.kind = self.kind;
        task.priority = self.priority;
        task.depends_on = self.depends_on;
        task.allowed_paths = clean(self.allowed_paths);
        task.acceptance_criteria = clean(self.acceptance_criteria);
        task.verification = clean(self.verification);
        task.verification_overrides = self.verification_overrides;
        task.assigned_agent = self.assigned_agent;
    }
}

fn clean(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[tauri::command(async)]
pub fn farm_add_task(state: State<'_, FarmState>, draft: TaskDraft) -> Result<Task> {
    let service = state.service()?;
    let ready = draft.ready;
    let mut task = Task::new(TaskId::new("pending"), String::new(), 0);
    draft.apply(&mut task);
    let created = service.add_task(task)?;
    if ready {
        service.ready(&created.id)?;
    }
    Ok(service
        .farm()
        .and_then(|farm| farm.task(&created.id).cloned())
        .unwrap_or(created))
}

#[tauri::command(async)]
pub fn farm_edit_task(state: State<'_, FarmState>, id: TaskId, draft: TaskDraft) -> Result<Task> {
    Ok(state.service()?.edit_task(&id, |task| draft.apply(task))?)
}

#[tauri::command(async)]
pub fn farm_delete_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    state.service()?.delete_task(&id)?;
    Ok(())
}

/// Move a draft into the plan.
#[tauri::command(async)]
pub fn farm_ready_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    let service = state.service()?;
    service.ready(&id)?;
    service.tick();
    watch_all(&service);
    Ok(())
}

/// Accept a plan: move several drafts into it at once.
#[tauri::command(async)]
pub fn farm_ready_tasks(state: State<'_, FarmState>, ids: Vec<TaskId>) -> Result<()> {
    let service = state.service()?;
    service.ready_all(&ids)?;
    service.tick();
    watch_all(&service);
    Ok(())
}

/// Discard a plan, or the part of it nobody wants.
#[tauri::command(async)]
pub fn farm_discard_tasks(state: State<'_, FarmState>, ids: Vec<TaskId>) -> Result<()> {
    state.service()?.discard_all(&ids)?;
    Ok(())
}

#[tauri::command(async)]
pub fn farm_assign_task(state: State<'_, FarmState>, id: TaskId, agent: AgentId) -> Result<()> {
    state.service()?.assign(&id, &agent)?;
    Ok(())
}

#[tauri::command(async)]
pub fn farm_cancel_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    state.service()?.cancel_task(&id)?;
    Ok(())
}

#[tauri::command(async)]
pub fn farm_retry_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    let service = state.service()?;
    service.retry_task(&id)?;
    service.tick();
    watch_all(&service);
    Ok(())
}

/// Start one task now.
#[tauri::command(async)]
pub fn farm_run_task(
    state: State<'_, FarmState>,
    id: TaskId,
    agent: Option<AgentId>,
) -> Result<()> {
    let service = state.service()?;
    service.run_task(&id, agent)?;
    watch_all(&service);
    Ok(())
}

#[tauri::command(async)]
pub fn farm_task_detail(state: State<'_, FarmState>, id: TaskId) -> Result<TaskDetail> {
    let service = state.service()?;
    let task = service
        .farm()
        .and_then(|farm| farm.task(&id).cloned())
        .ok_or_else(|| Failure::from(spagitty_farm::Error::NoSuchTask(id.clone())))?;
    Ok(TaskDetail {
        verification: service.verification_of(&id),
        review: service.review_of(&id),
        handoff: service.handoff_of(&id),
        runs: service
            .runs()
            .into_iter()
            .filter(|run| run.task == id)
            .collect(),
        task,
    })
}

/// The tail of a run's transcript.
#[tauri::command(async)]
pub fn farm_transcript(state: State<'_, FarmState>, run: RunId, task: TaskId) -> Result<String> {
    Ok(state.service()?.transcript(&run, &task))
}

/// Accept a reviewed task and merge it.
#[tauri::command(async)]
pub fn farm_merge_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    let service = state.service()?;
    service.merge(&id)?;
    watch_all(&service);
    Ok(())
}

/// Send a task to a reviewing agent by hand.
#[tauri::command(async)]
pub fn farm_review_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    let service = state.service()?;
    service.request_review(&id)?;
    watch_all(&service);
    Ok(())
}

/// Run the verification commands against a task's worktree.
#[tauri::command(async)]
pub fn farm_verify_task(state: State<'_, FarmState>, id: TaskId) -> Result<()> {
    let service = state.service()?;
    // Verification runs test suites, which take minutes. Off the caller's
    // thread, or the window freezes for as long as the suite takes.
    std::thread::Builder::new()
        .name("spagitty-farm-verify".into())
        .spawn(move || {
            if let Err(error) = service.verify(&id) {
                let _ = service.set_status(&id, TaskStatus::Blocked, Some(error.to_string()));
            }
            watch_all(&service);
        })
        .ok();
    Ok(())
}

// ── Planning ─────────────────────────────────────────────────────────────

/// Ask an agent to break the goal into tasks.
///
/// Returns immediately. The plan arrives as `TaskCreated` events when the
/// planning run finishes.
#[tauri::command(async)]
pub fn farm_plan(state: State<'_, FarmState>, agent: Option<AgentId>) -> Result<RunId> {
    let service = state.service()?;
    let run = service.plan(agent)?;
    collect(&service, run.clone());
    Ok(run)
}

/// Ask an agent to break one task into smaller ones.
///
/// The same run, pointed at a task. Its children arrive as drafts, and the task
/// becomes a container once they are accepted.
#[tauri::command(async)]
pub fn farm_decompose(
    state: State<'_, FarmState>,
    id: TaskId,
    agent: Option<AgentId>,
) -> Result<RunId> {
    let service = state.service()?;
    let run = service.decompose(&id, agent)?;
    collect(&service, run.clone());
    Ok(run)
}

/// Wait for a planning run on a thread and adopt what it produced.
fn collect(service: &Arc<FarmService>, run: RunId) {
    let service = service.clone();
    std::thread::Builder::new()
        .name("spagitty-farm-plan".into())
        .spawn(move || {
            let _ = service.collect_plan(&run);
        })
        .ok();
}

/// Stop a planning run.
///
/// Separate from `farm_cancel`, which cancels every task in the farm. A person
/// who has watched a planner for two minutes and changed their mind wants the
/// planner stopped, not the farm emptied.
#[tauri::command(async)]
pub fn farm_cancel_plan(state: State<'_, FarmState>) -> Result<()> {
    state.service()?.cancel_plan()?;
    Ok(())
}

// ── Housekeeping ─────────────────────────────────────────────────────────

/// Remove worktrees for tasks no farm claims.
#[tauri::command(async)]
pub fn farm_sweep(state: State<'_, FarmState>) -> Result<Vec<StaleWorkspace>> {
    Ok(state
        .service()?
        .sweep()?
        .into_iter()
        .map(|stale| StaleWorkspace {
            task: stale.task,
            path: stale.path,
            branch: stale.branch,
        })
        .collect())
}

/// The service claims each pending run before creating its completion thread.
/// This includes review runs, whose task status remains Review.
fn watch_all(service: &Arc<FarmService>) {
    service.watch_pending();
}

/// Close the farm when the repository closes.
///
/// Running agents are *not* stopped: closing a tab is not a decision to throw
/// away an agent's work, and the farm is on disk. Dropping the service would
/// kill them, so the `Arc` is simply forgotten here and the sessions live until
/// something asks them to stop.
#[tauri::command(async)]
pub fn farm_close(state: State<'_, FarmState>) -> Result<()> {
    *state.path.lock().expect("farm path lock") = None;
    Ok(())
}

/// Register the farm's state on the application.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(FarmState::default());
}

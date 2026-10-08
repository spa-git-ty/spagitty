// SPDX-License-Identifier: GPL-3.0-or-later

//! The assignment engine: one agent, one job, step by step.
//!
//! Spagitty drives. It decides the steps — read, plan, each file, the verdict;
//! or each conflict, the checks, the landing — and runs the agent once per
//! step through a [`Driver`]. Between steps it checks what no level lifts,
//! looks at the controls the person pressed, and asks [`super::level`] what
//! happens at the gate it has reached. The agent is never asked whether it may
//! go on.
//!
//! One thread per assignment. The person's controls arrive through a
//! [`Handle`] on any thread; *Stop* also cancels the step that is running,
//! everything else is taken at the next checkpoint, because cutting a step in
//! half leaves nothing usable.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use spagitty_core::diff::DiffLine;

use super::classify::{self, Choice};
use super::level::{self, AtGate, Circumstances, Gate, Level};
use super::protocol::{Answer, Finding, Plan, PlanItem, Summary, Verdict};
use super::record::{
    now, Assignment, CheckRun, LastAct, LevelChange, Proposal, ProposalBody, ProposalState, State,
    Step, StepKind, StepState, Store, Target, Tokens, Who, STEP_EVENTS,
};
use super::rules::RepoRules;
use super::{guard, prompt};

/// After this long with nothing from the agent, the sentence says so rather
/// than going on claiming it is reading — the farm's quiet-run rule.
pub const QUIET: Duration = Duration::from_secs(180);

// ── What the engine is given ─────────────────────────────────────────────

/// A thread on the pull request, as the agent is told about it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadBrief {
    pub path: Option<String>,
    pub line: Option<u32>,
    pub author: String,
    pub body: String,
    pub resolved: bool,
}

/// What a review is handed, beyond the diff it reads itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewWork {
    pub description: String,
    pub threads: Vec<ThreadBrief>,
    /// The checks as the host reports them, in a line.
    pub checks: String,
    /// Paths with conflict fixes in them (FEAT-092).
    pub conflict_fixes: Vec<String>,
}

/// One conflict region, as the resolver reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MergeRegion {
    pub index: usize,
    /// The marker lines, counted from 0 in the merged text.
    pub start: usize,
    pub end: usize,
    pub a: Vec<String>,
    pub b: Vec<String>,
    pub base: Option<Vec<String>>,
    pub a_from: Option<String>,
    pub b_from: Option<String>,
}

/// One conflicted file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MergeFile {
    pub path: String,
    pub merged: Vec<String>,
    pub eol: bool,
    /// The file conflicts as a whole: its one region is each side entire.
    pub whole: bool,
    pub regions: Vec<MergeRegion>,
}

/// What a merge is handed: the conflicts, already read by the resolver.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MergeWork {
    pub files: Vec<MergeFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "job", rename_all = "camelCase")]
pub enum Work {
    Review(ReviewWork),
    Merge(MergeWork),
}

/// Limits no level lifts. Reaching one stops the agent at the end of its
/// step and keeps everything it made.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Limits {
    pub tokens: Option<u64>,
    pub minutes: Option<u64>,
}

/// One changed file of a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub binary: bool,
}

/// The repository, as the engine reaches it. The real one is
/// [`super::world::Repository`]; tests use a fake.
pub trait World: Send {
    /// Where the agent runs: a scratch worktree, never the person's.
    fn workdir(&self) -> PathBuf;
    /// The repository's rules for agents (`AGENTS.md` and the others).
    fn policy(&self) -> String;
    fn changed_files(&self) -> Result<Vec<Changed>, String>;
    fn diff(&self, path: &str) -> Result<Vec<DiffLine>, String>;
    /// A file's text at the base (`false`) or the head (`true`).
    fn read(&self, path: &str, head: bool) -> Result<String, String>;
    /// Lines matching `pattern` at the head, `path:line: text`.
    fn search(&self, pattern: &str) -> Result<String, String>;
    /// Has a branch the plan read moved since?
    fn moved(&self) -> bool;
    /// The checks configured for this repository.
    fn checks(&self) -> Vec<String>;
    /// Run them on a worktree holding the merge with `resolved` written in.
    fn run_checks(
        &self,
        resolved: &[(String, String)],
        cancel: &AtomicBool,
        hear: &mut dyn FnMut(&str),
    ) -> Result<Vec<CheckRun>, String>;
    /// What a step left on disk, put back. See [`guard`].
    fn snapshot(&self) -> guard::Snapshot {
        guard::Snapshot::take(&self.workdir())
    }
    fn restore(&self, snapshot: &guard::Snapshot) -> Result<Vec<String>, String> {
        snapshot.restore(&self.workdir())
    }
}

/// One step, asked of an agent.
#[derive(Debug, Clone)]
pub struct Ask {
    pub kind: StepKind,
    /// The whole prompt, for an agent that reads one.
    pub prompt: String,
    pub workdir: PathBuf,
}

/// What came back from a step.
#[derive(Debug, Clone, Default)]
pub struct Reply {
    pub answer: Option<Answer>,
    /// The last thing it said, for a failure's sentence.
    pub last_words: String,
    pub tokens: Tokens,
    pub command: Option<String>,
    /// Files whose content was sent to a remote agent.
    pub sent: Vec<String>,
}

/// An agent, local or remote, as the engine runs it.
pub trait Driver: Send {
    fn run(
        &mut self,
        ask: &Ask,
        world: &dyn World,
        hear: &mut dyn FnMut(&str),
        cancel: &Arc<AtomicBool>,
    ) -> Result<Reply, String>;
}

/// Where changes go: the webview, as events.
pub trait Sink: Send + Sync {
    /// The assignment changed. Called at most a few times a second for the
    /// sentence, and at once for anything else.
    fn changed(&self, assignment: &Assignment);
    /// One line of raw output.
    fn line(&self, id: &str, line: &str);
}

/// What the person can do while an agent works.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Control {
    /// Stop at the end of this step.
    Pause,
    Resume,
    /// End the run now. Everything proposed stays.
    Stop,
    /// Stop, and the screen is the person's.
    TakeOver,
    Level {
        level: Level,
    },
    /// A line for the agent, delivered at the next step.
    Tell {
        text: String,
    },
    /// Let the agent go on from the gate it waits at.
    Continue,
    /// Run the waiting step again, with what was wrong.
    Redo {
        note: String,
    },
    AskWhy {
        proposal: String,
    },
    /// The person decided a proposal. `choice` is what they chose instead,
    /// for a resolution they changed.
    Decide {
        proposal: String,
        state: ProposalState,
        #[serde(default)]
        choice: Option<Choice>,
    },
    /// The reading plan, reordered or cut.
    Plan {
        files: Vec<String>,
    },
    /// The head or a branch moved; seen by the webview.
    Moved,
    /// Finish against the old head, and say so on every finding.
    CarryOn,
    /// The last act was done — by the webview for the agent, or by the
    /// person — or could not be.
    Acted {
        ok: bool,
        #[serde(default)]
        message: String,
    },
}

// ── Running ──────────────────────────────────────────────────────────────

struct Inner {
    assignment: Assignment,
    controls: VecDeque<Control>,
    last_heard: Instant,
    last_sent: Instant,
    dirty: bool,
}

struct Shared {
    inner: Mutex<Inner>,
    wake: Condvar,
    cancel: Arc<AtomicBool>,
    store: Store,
    sink: Arc<dyn Sink>,
    /// The repository's highest level: a change is capped by it.
    highest: Level,
}

impl Shared {
    fn save_and_send(&self, inner: &mut Inner) {
        let _ = self.store.save(&inner.assignment);
        self.sink.changed(&inner.assignment);
        inner.last_sent = Instant::now();
        inner.dirty = false;
    }
}

/// The person's side of a running assignment.
#[derive(Clone)]
pub struct Handle {
    shared: Arc<Shared>,
}

impl Handle {
    pub fn send(&self, control: Control) {
        if matches!(control, Control::Stop | Control::TakeOver) {
            self.shared.cancel.store(true, Ordering::Release);
        }
        let mut inner = self.shared.inner.lock().expect("assignment lock");
        if let Control::Tell { text } = &control {
            // Shown at once, as the person's, wherever the timeline is.
            let line = format!("You · {}", text.trim());
            if let Some(step) = inner.assignment.steps.last_mut() {
                step.events.push(line);
            }
            self.shared.save_and_send(&mut inner);
        }
        // A level is a fact about the record, and lowering it takes effect at
        // once: written now, not at the next checkpoint, so a stop that comes
        // first still finds it changed.
        if let Control::Level { level } = &control {
            let level = (*level).min(self.shared.highest);
            let a = &mut inner.assignment;
            if a.level != level && !a.state.is_over() {
                a.levels.push(LevelChange {
                    from: a.level,
                    to: level,
                    by: Who::Person,
                    at: now(),
                });
                a.level = level;
                self.shared.save_and_send(&mut inner);
            }
            return;
        }
        if matches!(control, Control::TakeOver) && !inner.assignment.state.is_over() {
            inner.assignment.took_over = Some(took_over(&inner.assignment));
        }
        if matches!(control, Control::Pause) && !inner.assignment.state.is_over() {
            inner.assignment.pausing = true;
            self.shared.save_and_send(&mut inner);
        }
        inner.controls.push_back(control);
        self.shared.wake.notify_all();
    }

    pub fn snapshot(&self) -> Assignment {
        self.shared
            .inner
            .lock()
            .expect("assignment lock")
            .assignment
            .clone()
    }

    pub fn is_over(&self) -> bool {
        self.snapshot().state.is_over()
    }
}

/// Everything an assignment needs, gathered at the start.
pub struct Setup {
    pub assignment: Assignment,
    pub work: Work,
    pub rules: RepoRules,
    pub limits: Limits,
    pub world: Box<dyn World>,
    pub driver: Box<dyn Driver>,
    pub store: Store,
    pub sink: Arc<dyn Sink>,
}

/// Start an assignment on a thread of its own.
pub fn start(setup: Setup) -> Handle {
    let (handle, engine) = prepare(setup);
    let quiet = handle.clone();
    std::thread::Builder::new()
        .name("spagitty-assignment".into())
        .spawn(move || engine.run())
        .expect("spawning an assignment");
    std::thread::Builder::new()
        .name("spagitty-assignment-quiet".into())
        .spawn(move || watch_quiet(quiet))
        .expect("spawning an assignment watcher");
    handle
}

/// The engine and its handle, without a thread: for tests, which run it on
/// their own and drive the handle from another.
pub fn prepare(setup: Setup) -> (Handle, Engine) {
    let shared = Arc::new(Shared {
        inner: Mutex::new(Inner {
            assignment: setup.assignment,
            controls: VecDeque::new(),
            last_heard: Instant::now(),
            last_sent: Instant::now(),
            dirty: false,
        }),
        wake: Condvar::new(),
        cancel: Arc::new(AtomicBool::new(false)),
        store: setup.store,
        sink: setup.sink,
        highest: setup.rules.highest,
    });
    let engine = Engine {
        shared: shared.clone(),
        work: setup.work,
        rules: setup.rules,
        limits: setup.limits,
        world: setup.world,
        driver: setup.driver,
        told: Vec::new(),
        started: Instant::now(),
        stale: false,
        choices: BTreeMap::new(),
        why: VecDeque::new(),
        moved: false,
        units: 0,
    };
    (Handle { shared }, engine)
}

fn watch_quiet(handle: Handle) {
    let mut said = 0u64;
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let shared = &handle.shared;
        let mut inner = shared.inner.lock().expect("assignment lock");
        if inner.assignment.state.is_over() {
            return;
        }
        if inner.dirty && inner.last_sent.elapsed() >= Duration::from_millis(250) {
            shared.save_and_send(&mut inner);
        }
        if inner.assignment.state != State::Working {
            said = 0;
            continue;
        }
        let quiet = inner.last_heard.elapsed();
        if quiet >= QUIET {
            let minutes = quiet.as_secs() / 60;
            if minutes != said {
                said = minutes;
                inner.assignment.sentence = format!("No output for {minutes} minutes");
                inner.assignment.quiet_since = Some(now().saturating_sub(quiet.as_secs()));
                shared.save_and_send(&mut inner);
            }
        }
    }
}

/// Why the run stopped short.
#[derive(Debug)]
enum Halt {
    Stopped,
    Failed(String),
}

type Flow<T> = Result<T, Halt>;

/// What the person did at a gate.
enum Released {
    Continue,
    Redo(String),
}

pub struct Engine {
    shared: Arc<Shared>,
    work: Work,
    rules: RepoRules,
    limits: Limits,
    world: Box<dyn World>,
    driver: Box<dyn Driver>,
    /// What the person said, for the next step.
    told: Vec<String>,
    started: Instant,
    /// Carrying on against an old head: every finding says so.
    stale: bool,
    /// What the person chose for a region instead, by `path#index`.
    choices: BTreeMap<String, Choice>,
    why: VecDeque<String>,
    /// The webview saw the head or a branch move.
    moved: bool,
    /// Files planned, or conflicts: what a failure counts against.
    units: usize,
}

impl Engine {
    /// Run to the end. Never panics on the agent's account: every way a step
    /// can go wrong ends as *Stopped* or *Failed*, with its reason.
    pub fn run(mut self) {
        self.update(|a| {
            a.state = State::Working;
            a.sentence = "Starting".into();
        });
        let flow = match self.work.clone() {
            Work::Review(work) => self.review(&work),
            Work::Merge(work) => self.merge(&work),
        };
        let mut inner = self.shared.inner.lock().expect("assignment lock");
        let a = &mut inner.assignment;
        match flow {
            Ok(()) => {}
            Err(Halt::Stopped) => {
                a.state = State::Stopped;
                if a.took_over.is_none() && a.reason.is_none() {
                    a.reason = Some("Stopped. Everything it proposed is kept.".into());
                }
                a.sentence = a
                    .took_over
                    .clone()
                    .or_else(|| a.reason.clone())
                    .unwrap_or_else(|| "Stopped".into());
            }
            Err(Halt::Failed(reason)) => {
                a.state = State::Failed;
                a.sentence = reason.clone();
                a.reason = Some(reason);
            }
        }
        for step in &mut a.steps {
            if matches!(step.state, StepState::Running | StepState::Waiting) {
                step.state = match (a.state, step.state) {
                    (State::Failed, _) => StepState::Failed,
                    // Cut off before it answered: not finished work, so a
                    // resume does it again rather than skipping it.
                    (_, StepState::Running) => StepState::Superseded,
                    _ => StepState::Done,
                };
                step.gate = None;
                step.ended_at.get_or_insert(now());
            }
        }
        a.pausing = false;
        a.ended_at = Some(now());
        self.shared.save_and_send(&mut inner);
        self.shared.wake.notify_all();
    }

    // ── State ────────────────────────────────────────────────────────────

    fn update(&self, change: impl FnOnce(&mut Assignment)) {
        let mut inner = self.shared.inner.lock().expect("assignment lock");
        change(&mut inner.assignment);
        self.shared.save_and_send(&mut inner);
    }

    fn read<T>(&self, look: impl FnOnce(&Assignment) -> T) -> T {
        look(
            &self
                .shared
                .inner
                .lock()
                .expect("assignment lock")
                .assignment,
        )
    }

    fn level(&self) -> Level {
        self.read(|a| a.level)
    }

    fn stop_asked(&self) -> bool {
        self.shared.cancel.load(Ordering::Acquire)
    }

    /// Take every queued control that does not need a gate.
    fn take_controls(&mut self) -> Vec<Control> {
        let queued: Vec<Control> = {
            let mut inner = self.shared.inner.lock().expect("assignment lock");
            inner.controls.drain(..).collect()
        };
        let mut left = Vec::new();
        for control in queued {
            match control {
                Control::Level { .. } => {}
                Control::Tell { text } => self.told.push(text),
                Control::AskWhy { proposal } => self.why.push_back(proposal),
                Control::Decide {
                    proposal,
                    state,
                    choice,
                } => self.decide(&proposal, state, choice),
                Control::Plan { files } => self.replan(&files),
                Control::Moved => self.moved = true,
                other => left.push(other),
            }
        }
        left
    }

    fn decide(&mut self, id: &str, state: ProposalState, choice: Option<Choice>) {
        let mut region = None;
        self.update(|a| {
            if let Some(proposal) = a.proposal_mut(id) {
                proposal.state = state;
                proposal.decided_by = Some(Who::Person);
                if let ProposalBody::Resolution {
                    path, region: at, ..
                } = &proposal.body
                {
                    region = Some(format!("{path}#{at}"));
                }
            }
        });
        if let (Some(region), Some(choice)) = (region, choice) {
            self.choices.insert(region, choice);
        }
    }

    fn replan(&self, files: &[String]) {
        self.update(|a| {
            if let Some(proposal) = a
                .proposals
                .iter_mut()
                .rev()
                .find(|p| matches!(p.body, ProposalBody::Plan(_)))
            {
                if let ProposalBody::Plan(plan) = &mut proposal.body {
                    let mut next: Vec<PlanItem> = Vec::new();
                    for path in files {
                        let why = plan
                            .files
                            .iter()
                            .find(|item| &item.path == path)
                            .map(|item| item.why.clone())
                            .unwrap_or_default();
                        next.push(PlanItem {
                            path: path.clone(),
                            why,
                        });
                    }
                    plan.files = next;
                    proposal.decided_by = Some(Who::Person);
                    proposal.state = ProposalState::Edited;
                }
            }
        });
    }

    /// Between steps: what the person asked for, and what no level lifts.
    fn checkpoint(&mut self) -> Flow<()> {
        loop {
            if self.stop_asked() {
                return Err(Halt::Stopped);
            }
            let left = self.take_controls();
            if left
                .iter()
                .any(|control| matches!(control, Control::Stop | Control::TakeOver))
            {
                return Err(Halt::Stopped);
            }
            while let Some(id) = self.why.pop_front() {
                self.ask_why(&id)?;
            }
            if let Some(limit) = self.over_limit() {
                self.update(|a| a.reason = Some(limit));
                return Err(Halt::Stopped);
            }
            if self.moved || self.world.moved() {
                if self.stale {
                    self.moved = false;
                } else {
                    self.wait_moved()?;
                    continue;
                }
            }
            let pausing =
                left.iter().any(|c| matches!(c, Control::Pause)) || self.read(|a| a.pausing);
            if pausing {
                self.pause()?;
                continue;
            }
            return Ok(());
        }
    }

    fn over_limit(&self) -> Option<String> {
        let used = self.read(|a| a.tokens.total());
        if let Some(tokens) = self.limits.tokens {
            if used >= tokens {
                return Some(format!(
                    "Reached its limit of {} tokens. Everything it made is kept.",
                    group(tokens)
                ));
            }
        }
        if let Some(minutes) = self.limits.minutes {
            if self.started.elapsed() >= Duration::from_secs(minutes * 60) {
                return Some(format!(
                    "Reached its limit of {minutes} minutes. Everything it made is kept."
                ));
            }
        }
        None
    }

    fn pause(&mut self) -> Flow<()> {
        let last = self.read(|a| a.steps.last().map(|s| s.label.clone()));
        self.update(|a| {
            a.state = State::Paused;
            a.pausing = false;
            a.sentence = match &last {
                Some(label) => format!("Paused after {label}"),
                None => "Paused".into(),
            };
        });
        loop {
            match self.next_control()? {
                Control::Resume | Control::Continue => {
                    self.update(|a| {
                        a.state = State::Working;
                        a.sentence = "Going on".into();
                    });
                    return Ok(());
                }
                _ => continue,
            }
        }
    }

    fn wait_moved(&mut self) -> Flow<()> {
        self.update(|a| {
            a.state = State::Waiting;
            let reason = match a.job {
                level::Job::Review => "The pull request changed since the agent started.",
                level::Job::Merge => "A branch moved since the agent started.",
            };
            a.reason = Some(reason.into());
            a.sentence = format!("Waiting for you: {reason}");
        });
        loop {
            if let Control::CarryOn | Control::Continue = self.next_control()? {
                self.stale = true;
                self.moved = false;
                self.update(|a| {
                    a.state = State::Working;
                    a.reason = None;
                    a.sentence = "Carrying on against the old head".into();
                });
                return Ok(());
            }
        }
    }

    /// Wait for the next control that needs the engine's attention. Stop is
    /// always an answer.
    fn next_control(&mut self) -> Flow<Control> {
        loop {
            if self.stop_asked() {
                return Err(Halt::Stopped);
            }
            let left = self.take_controls();
            while let Some(id) = self.why.pop_front() {
                self.ask_why(&id)?;
            }
            for control in left {
                match control {
                    Control::Stop | Control::TakeOver => return Err(Halt::Stopped),
                    Control::Pause => continue,
                    other => return Ok(other),
                }
            }
            let inner = self.shared.inner.lock().expect("assignment lock");
            if inner.controls.is_empty() && !self.shared.cancel.load(Ordering::Acquire) {
                let _ = self
                    .shared
                    .wake
                    .wait_timeout(inner, Duration::from_millis(500))
                    .expect("assignment lock");
            }
        }
    }

    // ── Steps ────────────────────────────────────────────────────────────

    fn begin(&self, kind: StepKind, label: String, sentence: String) -> usize {
        let mut index = 0;
        self.update(|a| {
            index = a.steps.len();
            a.steps.push(Step {
                index,
                kind,
                label,
                state: StepState::Running,
                started_at: now(),
                ended_at: None,
                gate: None,
                events: Vec::new(),
                sent: Vec::new(),
                refused: Vec::new(),
                note: None,
                command: None,
                checks: Vec::new(),
                tokens: Tokens::default(),
            });
            a.state = State::Working;
            a.sentence = sentence;
            a.quiet_since = None;
        });
        index
    }

    fn finish(&self, step: usize, label: Option<String>) {
        self.update(|a| {
            if let Some(entry) = a.steps.get_mut(step) {
                if entry.state != StepState::Superseded {
                    entry.state = StepState::Done;
                }
                entry.gate = None;
                entry.ended_at = Some(now());
                if let Some(label) = label {
                    entry.label = label;
                }
            }
        });
    }

    /// Run the agent for one step.
    fn ask(&mut self, step: usize, kind: StepKind, prompt: String) -> Flow<Reply> {
        let mut full = prompt;
        full.push_str(&prompt::notes("", &std::mem::take(&mut self.told)));
        let ask = Ask {
            kind,
            prompt: full,
            workdir: self.world.workdir(),
        };
        let snapshot = self.world.snapshot();
        let shared = self.shared.clone();
        let mut hear = |line: &str| {
            let mut inner = shared.inner.lock().expect("assignment lock");
            inner.last_heard = Instant::now();
            let id = inner.assignment.id.clone();
            shared.store.transcript(&inner.assignment, line);
            if let Some(entry) = inner.assignment.steps.get_mut(step) {
                if entry.events.len() < STEP_EVENTS {
                    entry.events.push(line.to_string());
                }
            }
            inner.dirty = true;
            shared.sink.line(&id, line);
            if inner.last_sent.elapsed() >= Duration::from_millis(250) {
                shared.save_and_send(&mut inner);
            }
        };
        let cancel = self.shared.cancel.clone();
        let result = self.driver.run(&ask, &*self.world, &mut hear, &cancel);
        let restored = self.world.restore(&snapshot);
        let refused = restored.clone().unwrap_or_default();

        let reply = match &result {
            Ok(reply) => Some(reply.clone()),
            Err(_) => None,
        };
        self.update(|a| {
            if let Some(reply) = &reply {
                a.tokens.add(reply.tokens);
            }
            if let Some(entry) = a.steps.get_mut(step) {
                if let Some(reply) = &reply {
                    entry.tokens.add(reply.tokens);
                    entry.command = reply.command.clone();
                    entry.sent = reply.sent.clone();
                }
                for path in &refused {
                    entry.events.push(format!(
                        "changed {path}, which it was not asked to; not kept"
                    ));
                }
                entry.refused = refused.clone();
            }
        });

        // What it wrote is still there: say so and stop, rather than carry
        // on as though it had been put back.
        if let Err(why) = restored {
            return Err(Halt::Failed(format!(
                "What it wrote where it was not asked to could not be put back: {why}"
            )));
        }
        if self.stop_asked() {
            return Err(Halt::Stopped);
        }
        match result {
            Ok(reply) => Ok(reply),
            Err(error) => Err(Halt::Failed(self.failure(&error))),
        }
    }

    /// *Codex exited with 1 after 4 of 12 files. Its 3 findings are kept.*
    fn failure(&self, what: &str) -> String {
        let (name, job, done, kept) = self.read(|a| {
            (
                a.agent.name.clone(),
                a.job,
                units_done(a),
                a.proposals
                    .iter()
                    .filter(|p| !matches!(p.body, ProposalBody::Plan(_)))
                    .count(),
            )
        });
        let unit = match job {
            level::Job::Review => "files",
            level::Job::Merge => "conflicts",
        };
        let what = what.trim().trim_end_matches('.');
        let mut out = format!("{name} {what}");
        if self.units > 0 {
            out.push_str(&format!(" after {done} of {} {unit}", self.units));
        }
        out.push('.');
        if kept > 0 {
            let noun = match job {
                level::Job::Review if kept == 1 => "finding is",
                level::Job::Review => "findings are",
                level::Job::Merge if kept == 1 => "proposal is",
                level::Job::Merge => "proposals are",
            };
            out.push_str(&format!(" Its {kept} {noun} kept."));
        }
        out
    }

    fn no_answer(&self, reply: &Reply) -> Halt {
        let words = reply.last_words.trim();
        let tail: String = words
            .chars()
            .rev()
            .take(200)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Halt::Failed(self.failure(&if tail.is_empty() {
            "gave no answer Spagitty could read".to_string()
        } else {
            format!("gave no answer Spagitty could read: “{tail}”")
        }))
    }

    fn propose(&self, step: usize, body: ProposalBody, sure: bool) -> String {
        let mut id = String::new();
        let stale = self.stale;
        self.update(|a| {
            id = format!("p{}-{}", step, a.proposals.len());
            a.proposals.push(Proposal {
                id: id.clone(),
                step,
                body,
                sure,
                state: ProposalState::Proposed,
                decided_by: None,
                why: None,
                stale,
            });
        });
        id
    }

    /// The gate a step has reached. Returns what the person did, if they had
    /// to do anything.
    fn gate(
        &mut self,
        step: usize,
        gate: Gate,
        now_: Circumstances,
        waiting: String,
    ) -> Flow<Released> {
        match level::at(self.level(), gate, now_) {
            AtGate::Stops => self.hold(step, gate, waiting),
            AtGate::Proposes => {
                self.finish(step, None);
                Ok(Released::Continue)
            }
            AtGate::GoesOn | AtGate::AgentDoesIt | AtGate::YouDoIt => {
                self.apply_sure(step);
                self.finish(step, None);
                Ok(Released::Continue)
            }
        }
    }

    /// The agent's sure proposals from this step become the person's material.
    /// Never an unsure one, and never one from before a raise.
    fn apply_sure(&self, step: usize) {
        self.update(|a| {
            for proposal in a.proposals.iter_mut().filter(|p| p.step == step) {
                if proposal.state == ProposalState::Proposed && proposal.sure {
                    proposal.state = ProposalState::Applied;
                    proposal.decided_by = Some(Who::Agent);
                }
            }
        });
    }

    fn hold(&mut self, step: usize, gate: Gate, waiting: String) -> Flow<Released> {
        self.update(|a| {
            a.state = State::Waiting;
            a.sentence = format!("Waiting for you: {waiting}");
            if let Some(entry) = a.steps.get_mut(step) {
                entry.state = StepState::Waiting;
                entry.gate = Some(gate);
            }
        });
        loop {
            match self.next_control()? {
                Control::Continue | Control::Resume => {
                    self.finish(step, None);
                    return Ok(Released::Continue);
                }
                Control::Redo { note } => {
                    self.update(|a| {
                        if let Some(entry) = a.steps.get_mut(step) {
                            entry.state = StepState::Superseded;
                            entry.gate = None;
                            entry.ended_at = Some(now());
                            entry.note = Some(note.clone());
                        }
                        for proposal in a.proposals.iter_mut().filter(|p| p.step == step) {
                            if proposal.state == ProposalState::Proposed {
                                proposal.state = ProposalState::Superseded;
                            }
                        }
                    });
                    return Ok(Released::Redo(note));
                }
                _ => continue,
            }
        }
    }

    fn ask_why(&mut self, id: &str) -> Flow<()> {
        if self.level() == Level::Unattended {
            return Ok(());
        }
        let what = self.read(|a| {
            a.proposals
                .iter()
                .find(|p| p.id == id)
                .map(|p| describe(&p.body))
        });
        let Some(what) = what else { return Ok(()) };
        let state = self.read(|a| a.state);
        let sentence = self.read(|a| a.sentence.clone());
        let step = self.begin(
            StepKind::Why {
                proposal: id.to_string(),
            },
            "Why".into(),
            "Saying why".into(),
        );
        let job = self.read(|a| a.job);
        let text = format!(
            "{}{}",
            prompt::preamble(job, &self.world.policy()),
            prompt::why_step(&what)
        );
        let reply = self.ask(
            step,
            StepKind::Why {
                proposal: id.into(),
            },
            text,
        )?;
        let why = reply
            .answer
            .and_then(|answer| answer.why)
            .unwrap_or_else(|| reply.last_words.trim().to_string());
        self.update(|a| {
            if let Some(proposal) = a.proposal_mut(id) {
                proposal.why = Some(why.clone());
            }
            if let Some(entry) = a.steps.get_mut(step) {
                entry.events.push(why);
            }
            // Asking why costs a step; it does not change where the run was.
            a.state = state;
            a.sentence = sentence;
        });
        self.finish(step, None);
        Ok(())
    }

    // ── Review ───────────────────────────────────────────────────────────

    fn review(&mut self, work: &ReviewWork) -> Flow<()> {
        let files = self.world.changed_files().map_err(Halt::Failed)?;
        let policy = self.world.policy();
        let note = self.read(|a| a.note.clone());
        let opening = format!(
            "{}{}",
            prompt::preamble(level::Job::Review, &policy),
            prompt::notes(&note, &[])
        );

        let read = self.begin(
            StepKind::Read,
            "Read the pull request".into(),
            "Reading the pull request".into(),
        );
        let context = self.pr_context(work, &files);
        self.finish(
            read,
            Some(format!(
                "Read the pull request · {}",
                plural(files.len(), "file")
            )),
        );
        self.checkpoint()?;

        // The plan: kept from before a restart, or asked for.
        let plan = match self.kept_plan() {
            Some(plan) => plan,
            None => loop {
                let step = self.begin(StepKind::Plan, "Plan".into(), "Planning".into());
                let reply = self.ask(
                    step,
                    StepKind::Plan,
                    format!("{opening}{}", prompt::plan_step(&context)),
                )?;
                let Some(mut plan) = reply.answer.as_ref().and_then(Answer::plan) else {
                    return Err(self.no_answer(&reply));
                };
                // Only files the pull request changes, each once.
                let mut seen = Vec::new();
                plan.files.retain(|item| {
                    let ok =
                        files.iter().any(|f| f.path == item.path) && !seen.contains(&item.path);
                    seen.push(item.path.clone());
                    ok
                });
                let count = plan.files.len();
                self.update(|a| {
                    a.planned = count + 3;
                    if let Some(entry) = a.steps.get_mut(step) {
                        entry.label = format!("Plan · {}", plural(count, "file"));
                    }
                });
                self.propose(step, ProposalBody::Plan(plan.clone()), true);
                match self.gate(
                    step,
                    Gate::Plan,
                    Circumstances::default(),
                    format!("the plan · {}", plural(count, "file")),
                )? {
                    Released::Continue => break self.kept_plan().unwrap_or(plan),
                    Released::Redo(note) => self.told.push(note),
                }
            },
        };
        let total = plan.files.len();
        self.units = total;
        self.update(|a| a.planned = total + 3);

        for (number, item) in plan.files.iter().enumerate() {
            if self.file_done(&item.path) {
                continue;
            }
            self.checkpoint()?;
            self.review_file(work, &plan, &opening, &item.path, number + 1, total)?;
        }

        self.checkpoint()?;
        let summary = self.verdict(&opening)?;
        self.send(summary)
    }

    fn pr_context(&self, work: &ReviewWork, files: &[Changed]) -> String {
        let (title, target) = self.read(|a| match &a.target {
            Target::Review {
                title,
                target,
                number,
                ..
            } => (format!("#{number} {title}"), target.clone()),
            Target::Merge { .. } => (String::new(), String::new()),
        });
        let mut out = format!("{title}\n");
        if !target.is_empty() {
            out.push_str(&format!("Into {target}.\n"));
        }
        if !work.description.trim().is_empty() {
            out.push_str(&format!("\nDescription:\n{}\n", work.description.trim()));
        }
        if !work.checks.trim().is_empty() {
            out.push_str(&format!("\nChecks: {}\n", work.checks.trim()));
        }
        out.push_str("\nChanged files:\n");
        for file in files {
            let fix = if work.conflict_fixes.contains(&file.path) {
                " (has conflict fixes)"
            } else {
                ""
            };
            let size = if file.binary {
                "binary".to_string()
            } else {
                format!("+{} -{}", file.added, file.removed)
            };
            out.push_str(&format!("- {} {size}{fix}\n", file.path));
        }
        let (open, resolved): (Vec<&ThreadBrief>, Vec<&ThreadBrief>) =
            work.threads.iter().partition(|thread| !thread.resolved);
        if !open.is_empty() {
            out.push_str("\nOpen threads:\n");
            for thread in open {
                out.push_str(&format!("- {}\n", brief(thread)));
            }
        }
        if !resolved.is_empty() {
            out.push_str(&format!(
                "\n{} resolved.\n",
                plural(resolved.len(), "thread")
            ));
        }
        out
    }

    fn kept_plan(&self) -> Option<Plan> {
        self.read(|a| {
            a.proposals
                .iter()
                .rev()
                .find_map(|p| match (&p.body, p.state) {
                    (ProposalBody::Plan(plan), state)
                        if !matches!(
                            state,
                            ProposalState::Superseded | ProposalState::Dismissed
                        ) =>
                    {
                        Some(plan.clone())
                    }
                    _ => None,
                })
        })
    }

    fn file_done(&self, path: &str) -> bool {
        self.read(|a| {
            a.steps.iter().any(|s| {
                s.state == StepState::Done
                    && matches!(&s.kind, StepKind::File { path: p } if p == path)
            })
        })
    }

    fn review_file(
        &mut self,
        work: &ReviewWork,
        plan: &Plan,
        opening: &str,
        path: &str,
        number: usize,
        total: usize,
    ) -> Flow<()> {
        let lines = self.world.diff(path).map_err(Halt::Failed)?;
        let rendered = prompt::render_diff(&lines, 3);
        let threads: String = work
            .threads
            .iter()
            .filter(|t| !t.resolved && t.path.as_deref() == Some(path))
            .map(|t| format!("- {}\n", brief(t)))
            .collect();
        let name = file_name(path);
        loop {
            let kind = StepKind::File {
                path: path.to_string(),
            };
            let step = self.begin(
                kind.clone(),
                name.to_string(),
                format!("Reading {path} · {number} of {total} files"),
            );
            let text = format!(
                "{opening}{}",
                prompt::file_step(
                    path,
                    &plan.look_for,
                    &rendered,
                    work.conflict_fixes.iter().any(|p| p == path),
                    &threads
                )
            );
            let reply = self.ask(step, kind, text)?;
            let Some(findings) = reply.answer.as_ref().and_then(|a| a.findings.clone()) else {
                return Err(self.no_answer(&reply));
            };
            let mut kept = 0;
            let mut unsure = false;
            for mut finding in findings {
                finding.path = path.to_string();
                if !prompt::on_diff(&lines, finding.side, finding.line) {
                    self.update(|a| {
                        if let Some(entry) = a.steps.get_mut(step) {
                            entry.events.push(format!(
                                "a finding on line {} is not on a line of the diff; not kept",
                                finding.line
                            ));
                        }
                    });
                    continue;
                }
                if let Some(start) = finding.start_line {
                    if start >= finding.line || !prompt::on_diff(&lines, finding.side, start) {
                        finding.start_line = None;
                    }
                }
                unsure |= !finding.sure;
                kept += 1;
                let sure = finding.sure;
                self.propose(step, ProposalBody::Comment(finding), sure);
            }
            self.update(|a| {
                if let Some(entry) = a.steps.get_mut(step) {
                    entry.label = format!("{name} · {}", plural(kept, "finding"));
                }
            });
            let waiting = format!("{} on {name}", plural(kept, "finding"));
            match self.gate(
                step,
                Gate::File,
                Circumstances {
                    unsure,
                    ..Default::default()
                },
                waiting,
            )? {
                Released::Continue => return Ok(()),
                Released::Redo(note) => self.told.push(note),
            }
        }
    }

    fn verdict(&mut self, opening: &str) -> Flow<Summary> {
        loop {
            let found: String = self.read(|a| {
                a.proposals
                    .iter()
                    .filter(|p| {
                        !matches!(
                            p.state,
                            ProposalState::Superseded | ProposalState::Dismissed
                        )
                    })
                    .filter_map(|p| match &p.body {
                        ProposalBody::Comment(f) => Some(format!(
                            "- {}:{} ({:?}) {}\n",
                            f.path, f.line, f.severity, f.body
                        )),
                        _ => None,
                    })
                    .collect()
            });
            let found = if found.is_empty() {
                "Nothing.".to_string()
            } else {
                found
            };
            let step = self.begin(StepKind::Verdict, "Verdict".into(), "Summing up".into());
            let reply = self.ask(
                step,
                StepKind::Verdict,
                format!("{opening}{}", prompt::verdict_step(&found)),
            )?;
            let Some(summary) = reply.answer.as_ref().and_then(Answer::summary) else {
                return Err(self.no_answer(&reply));
            };
            self.update(|a| {
                if let Some(entry) = a.steps.get_mut(step) {
                    entry.label = format!("Verdict · {}", verdict_word(summary.verdict));
                }
            });
            let sure = summary.sure;
            self.propose(step, ProposalBody::Verdict(summary.clone()), sure);
            match self.gate(
                step,
                Gate::Verdict,
                Circumstances {
                    unsure: !sure,
                    ..Default::default()
                },
                "the verdict".into(),
            )? {
                Released::Continue => return Ok(summary),
                Released::Redo(note) => self.told.push(note),
            }
        }
    }

    fn send(&mut self, summary: Summary) -> Flow<()> {
        let unsure = self.read(|a| {
            a.proposals
                .iter()
                .any(|p| p.state == ProposalState::Proposed && !p.sure)
        });
        let verdict = if self.rules.verdicts {
            summary.verdict
        } else {
            Verdict::Comment
        };
        let findings = self.read(|a| {
            a.proposals
                .iter()
                .filter(|p| matches!(p.body, ProposalBody::Comment(_)))
                .filter(|p| !matches!(p.state, ProposalState::Superseded))
                .count()
        });
        self.last_act(
            Gate::Send,
            Circumstances {
                unsure,
                ..Default::default()
            },
            LastAct::Send {
                verdict,
                body: summary.summary,
            },
            (
                "Sending the review".into(),
                format!("Sent as {}", verdict_word(verdict)),
            ),
            (
                "Finish review".into(),
                format!("Done · {} to decide", plural(findings, "finding")),
            ),
        )
    }

    /// The irreversible act. The agent's at *Unattended* when nothing refuses
    /// it; the person's otherwise.
    fn last_act(
        &mut self,
        gate: Gate,
        now_: Circumstances,
        act: LastAct,
        agent_words: (String, String),
        person_words: (String, String),
    ) -> Flow<()> {
        let level = self.level();
        match level::at(level, gate, now_) {
            AtGate::AgentDoesIt => {
                let step = self.begin(StepKind::Last, agent_words.0.clone(), agent_words.0);
                self.update(|a| a.last_act = Some(act));
                loop {
                    if let Control::Acted { ok, message } = self.next_control()? {
                        self.update(|a| a.last_act = None);
                        if !ok {
                            return Err(Halt::Failed(message));
                        }
                        self.finish(step, Some(agent_words.1.clone()));
                        self.update(|a| {
                            a.state = State::Done;
                            a.sentence = agent_words.1.clone();
                        });
                        return Ok(());
                    }
                }
            }
            AtGate::Stops | AtGate::YouDoIt if level >= Level::SignOff => {
                let step = self.begin(StepKind::Last, person_words.0.clone(), String::new());
                self.update(|a| {
                    a.state = State::Waiting;
                    a.sentence = format!("Waiting for you: {}", person_words.0);
                    if let Some(entry) = a.steps.get_mut(step) {
                        entry.state = StepState::Waiting;
                        entry.gate = Some(gate);
                    }
                });
                loop {
                    match self.next_control()? {
                        Control::Acted { ok: true, .. } | Control::Continue => {
                            self.finish(step, None);
                            self.update(|a| {
                                a.state = State::Done;
                                a.sentence = "Done".into();
                            });
                            return Ok(());
                        }
                        _ => continue,
                    }
                }
            }
            _ => {
                self.update(|a| {
                    a.state = State::Done;
                    a.sentence = person_words.1;
                });
                Ok(())
            }
        }
    }

    // ── Merge ────────────────────────────────────────────────────────────

    fn merge(&mut self, work: &MergeWork) -> Flow<()> {
        let policy = self.world.policy();
        let (note, names, lands, into) = self.read(|a| {
            let (names, into) = match &a.target {
                Target::Merge {
                    a: x, b: y, into, ..
                } => (
                    (x.clone(), y.clone()),
                    if into.is_empty() {
                        x.clone()
                    } else {
                        into.clone()
                    },
                ),
                Target::Review { .. } => ((String::new(), String::new()), String::new()),
            };
            (a.note.clone(), names, a.lands, into)
        });
        let opening = format!(
            "{}{}",
            prompt::preamble(level::Job::Merge, &policy),
            prompt::notes(&note, &[])
        );
        let regions: Vec<(usize, usize)> = work
            .files
            .iter()
            .enumerate()
            .flat_map(|(f, file)| (0..file.regions.len()).map(move |r| (f, r)))
            .collect();
        let total = regions.len();
        self.units = total;
        let read = self.begin(
            StepKind::Read,
            "Read the merge".into(),
            "Reading the merge".into(),
        );
        self.update(|a| a.planned = total + 2 + usize::from(lands));
        self.finish(
            read,
            Some(format!(
                "Read the merge · {} in {}",
                plural(total, "conflict"),
                plural(work.files.len(), "file")
            )),
        );

        for (number, (f, r)) in regions.iter().enumerate() {
            let file = &work.files[*f];
            let region = &file.regions[*r];
            if self.region_done(&file.path, region.index) {
                continue;
            }
            self.checkpoint()?;
            self.resolve(
                &opening,
                file,
                region,
                number + 1,
                total,
                (&names.0, &names.1),
            )?;
        }

        self.checkpoint()?;
        let (configured, failed) = self.checks(work)?;
        if lands {
            self.checkpoint()?;
            let unresolved = self.unresolved(work);
            let unsure = self.read(|a| {
                a.proposals.iter().any(|p| {
                    matches!(p.body, ProposalBody::Resolution { .. })
                        && p.state == ProposalState::Proposed
                        && !p.sure
                })
            });
            let refused = !configured || unresolved > 0 || !self.rules.may_land_unattended(&into);
            self.last_act(
                Gate::Land,
                Circumstances {
                    unsure,
                    checks_failed: failed,
                    moved: self.world.moved(),
                    last_act_refused: refused,
                    ..Default::default()
                },
                LastAct::Land,
                ("Landing".into(), format!("Landed in {into}")),
                (
                    "Complete merge".into(),
                    "Ready for you to complete the merge".into(),
                ),
            )
        } else {
            self.update(|a| {
                a.state = State::Done;
                a.sentence = format!(
                    "Proposed {} · the merge is yours",
                    plural(total, "resolution")
                );
            });
            Ok(())
        }
    }

    fn region_done(&self, path: &str, index: usize) -> bool {
        self.read(|a| {
            a.steps.iter().any(|s| {
                s.state == StepState::Done
                    && matches!(&s.kind, StepKind::Conflict { path: p, region } if p == path && *region == index)
            })
        })
    }

    fn resolve(
        &mut self,
        opening: &str,
        file: &MergeFile,
        region: &MergeRegion,
        number: usize,
        total: usize,
        names: (&str, &str),
    ) -> Flow<()> {
        let around = around(file, region, 4);
        let name = file_name(&file.path);
        loop {
            let kind = StepKind::Conflict {
                path: file.path.clone(),
                region: region.index,
            };
            let step = self.begin(
                kind.clone(),
                name.to_string(),
                format!("Proposing a resolution · conflict {number} of {total}"),
            );
            let text = format!(
                "{opening}{}",
                prompt::conflict_step(
                    &file.path,
                    number,
                    total,
                    names,
                    &region.a,
                    &region.b,
                    region.base.as_deref(),
                    &around
                )
            );
            let reply = self.ask(step, kind, text)?;
            let Some(resolution) = reply.answer.as_ref().and_then(|a| a.resolution.clone()) else {
                return Err(self.no_answer(&reply));
            };
            let choice = classify::classify(&region.a, &region.b, &resolution.text);
            let label = format!("{name} · {}", short_choice(&choice));
            self.update(|a| {
                if let Some(entry) = a.steps.get_mut(step) {
                    entry.label = label.clone();
                }
            });
            let sure = resolution.sure;
            self.propose(
                step,
                ProposalBody::Resolution {
                    path: file.path.clone(),
                    region: region.index,
                    choice,
                    why: resolution.why,
                },
                sure,
            );
            let waiting = if sure {
                format!("conflict {number} in {name}")
            } else {
                format!("{name} · unsure")
            };
            match self.gate(
                step,
                Gate::Conflict,
                Circumstances {
                    unsure: !sure,
                    ..Default::default()
                },
                waiting,
            )? {
                Released::Continue => return Ok(()),
                Released::Redo(note) => self.told.push(note),
            }
        }
    }

    /// The choice that stands for a region: the person's, or the agent's
    /// latest one still standing.
    fn standing(&self, path: &str, index: usize) -> Option<Choice> {
        if let Some(choice) = self.choices.get(&format!("{path}#{index}")) {
            return Some(choice.clone());
        }
        self.read(|a| {
            a.proposals.iter().rev().find_map(|p| match &p.body {
                ProposalBody::Resolution {
                    path: at,
                    region,
                    choice,
                    ..
                } if at == path
                    && *region == index
                    && !matches!(
                        p.state,
                        ProposalState::Superseded | ProposalState::Dismissed
                    ) =>
                {
                    Some(choice.clone())
                }
                _ => None,
            })
        })
    }

    fn unresolved(&self, work: &MergeWork) -> usize {
        work.files
            .iter()
            .flat_map(|file| {
                file.regions
                    .iter()
                    .map(move |r| (file.path.as_str(), r.index))
            })
            .filter(|(path, index)| self.standing(path, *index).is_none())
            .count()
    }

    /// Each conflicted file as the standing choices make it. A region with no
    /// choice keeps its markers: checks then say so rather than passing.
    pub(crate) fn resolved(&self, work: &MergeWork) -> Vec<(String, String)> {
        work.files
            .iter()
            .map(|file| {
                let choices: Vec<Option<Choice>> = file
                    .regions
                    .iter()
                    .map(|r| self.standing(&file.path, r.index))
                    .collect();
                (file.path.clone(), apply(file, &choices))
            })
            .collect()
    }

    fn checks(&mut self, work: &MergeWork) -> Flow<(bool, bool)> {
        let commands = self.world.checks();
        if commands.is_empty() {
            let step = self.begin(StepKind::Checks, "Checks".into(), "Checks".into());
            self.update(|a| {
                if let Some(entry) = a.steps.get_mut(step) {
                    entry
                        .events
                        .push("No checks are configured for this repository.".into());
                }
            });
            self.finish(step, Some("Checks · none configured".into()));
            return Ok((false, false));
        }
        loop {
            let step = self.begin(
                StepKind::Checks,
                "Checks".into(),
                format!("Running checks · {}", commands[0]),
            );
            let resolved = self.resolved(work);
            let shared = self.shared.clone();
            let mut hear = |line: &str| {
                let mut inner = shared.inner.lock().expect("assignment lock");
                inner.last_heard = Instant::now();
                let id = inner.assignment.id.clone();
                if let Some(command) = line.strip_prefix("$ ") {
                    inner.assignment.sentence = format!("Running checks · {command}");
                }
                if let Some(entry) = inner.assignment.steps.get_mut(step) {
                    if entry.events.len() < STEP_EVENTS {
                        entry.events.push(line.to_string());
                    }
                }
                inner.dirty = true;
                shared.sink.line(&id, line);
            };
            let runs = self
                .world
                .run_checks(&resolved, &self.shared.cancel, &mut hear)
                .map_err(Halt::Failed)?;
            if self.stop_asked() {
                return Err(Halt::Stopped);
            }
            let failed = runs.iter().filter(|run| !run.passed).count();
            let label = if failed == 0 {
                "Checks · passed".to_string()
            } else {
                format!("Checks · {} failed", failed)
            };
            self.update(|a| {
                if let Some(entry) = a.steps.get_mut(step) {
                    entry.checks = runs.clone();
                    entry.label = label.clone();
                }
            });
            let first = runs
                .iter()
                .find(|run| !run.passed)
                .map(|run| run.command.clone());
            let waiting = match first {
                Some(command) => format!("{command} failed"),
                None => "the checks".into(),
            };
            match self.gate(
                step,
                Gate::Checks,
                Circumstances {
                    checks_failed: failed > 0,
                    ..Default::default()
                },
                waiting,
            )? {
                Released::Continue => return Ok((true, failed > 0)),
                Released::Redo(_) => continue,
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Files or conflicts finished.
fn units_done(a: &Assignment) -> usize {
    a.steps
        .iter()
        .filter(|s| {
            s.state == StepState::Done
                && matches!(s.kind, StepKind::File { .. } | StepKind::Conflict { .. })
        })
        .count()
}

/// *You took over at file 5 of 12*.
fn took_over(a: &Assignment) -> String {
    let done = units_done(a);
    let planned = a.planned.saturating_sub(match a.job {
        level::Job::Review => 3,
        level::Job::Merge => 2 + usize::from(a.lands),
    });
    let unit = match a.job {
        level::Job::Review => "file",
        level::Job::Merge => "conflict",
    };
    if planned == 0 {
        return "You took over".into();
    }
    format!(
        "You took over at {unit} {} of {planned}",
        (done + 1).min(planned)
    )
}

/// The lines a choice puts in a region.
pub fn chosen(region: &MergeRegion, choice: &Choice) -> Vec<String> {
    match choice {
        Choice::A => region.a.clone(),
        Choice::B => region.b.clone(),
        Choice::Ab => region.a.iter().chain(&region.b).cloned().collect(),
        Choice::Ba => region.b.iter().chain(&region.a).cloned().collect(),
        Choice::Pick { a, b } => region
            .a
            .iter()
            .zip(a)
            .filter(|(_, keep)| **keep)
            .map(|(line, _)| line.clone())
            .chain(
                region
                    .b
                    .iter()
                    .zip(b)
                    .filter(|(_, keep)| **keep)
                    .map(|(line, _)| line.clone()),
            )
            .collect(),
        Choice::Edit { text } => classify::lines_of(text),
    }
}

/// A file's text with each region replaced by its choice.
pub fn apply(file: &MergeFile, choices: &[Option<Choice>]) -> String {
    if file.whole {
        let lines = match (file.regions.first(), choices.first()) {
            (Some(region), Some(Some(choice))) => chosen(region, choice),
            _ => file.merged.clone(),
        };
        return join(&lines, file.eol || !lines.is_empty());
    }
    let mut out: Vec<String> = Vec::new();
    let mut at = 0;
    for (region, choice) in file.regions.iter().zip(choices) {
        let end = region.end.min(file.merged.len().saturating_sub(1));
        if region.start < at || region.start >= file.merged.len() {
            continue;
        }
        out.extend(file.merged[at..region.start].iter().cloned());
        match choice {
            Some(choice) => out.extend(chosen(region, choice)),
            None => out.extend(file.merged[region.start..=end].iter().cloned()),
        }
        at = end + 1;
    }
    if at < file.merged.len() {
        out.extend(file.merged[at..].iter().cloned());
    }
    join(&out, file.eol)
}

fn join(lines: &[String], eol: bool) -> String {
    let mut text = lines.join("\n");
    if eol && !lines.is_empty() {
        text.push('\n');
    }
    text
}

fn around(file: &MergeFile, region: &MergeRegion, context: usize) -> String {
    if file.whole || file.merged.is_empty() {
        return String::new();
    }
    let from = region.start.saturating_sub(context);
    let to = (region.end + context + 1).min(file.merged.len());
    file.merged[from..to].join("\n")
}

fn describe(body: &ProposalBody) -> String {
    match body {
        ProposalBody::Plan(plan) => plan
            .files
            .iter()
            .map(|f| format!("- {}", f.path))
            .collect::<Vec<_>>()
            .join("\n"),
        ProposalBody::Comment(Finding {
            path, line, body, ..
        }) => format!("On {path}:{line}: {body}"),
        ProposalBody::Verdict(summary) => {
            format!("{} — {}", verdict_word(summary.verdict), summary.summary)
        }
        ProposalBody::Resolution {
            path,
            region,
            choice,
            why,
        } => {
            format!(
                "Conflict {} in {path}: {} — {why}",
                region + 1,
                choice.label()
            )
        }
    }
}

fn brief(thread: &ThreadBrief) -> String {
    let place = match (&thread.path, thread.line) {
        (Some(path), Some(line)) => format!("{path}:{line} "),
        (Some(path), None) => format!("{path} "),
        _ => String::new(),
    };
    let body: String = thread.body.chars().take(400).collect();
    format!("{place}{}: {}", thread.author, body.replace('\n', " "))
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn group(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Comment => "Comment",
        Verdict::Approve => "Approve",
        Verdict::RequestChanges => "Request changes",
    }
}

fn short_choice(choice: &Choice) -> &'static str {
    match choice {
        Choice::A => "A",
        Choice::B => "B",
        Choice::Ab | Choice::Ba => "Both",
        Choice::Pick { .. } => "Pick",
        Choice::Edit { .. } => "Edit",
    }
}

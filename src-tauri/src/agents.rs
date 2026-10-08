// SPDX-License-Identifier: GPL-3.0-or-later

//! Agents on this machine, and the assignments they work on (2.0).
//!
//! Settings › Agents is where agents are attached, once, for the machine: the
//! command-line agents detection finds or the person adds by hand, and models
//! reached over an API with the person's own key. Which of them a repository
//! may use, and how far they may go there, is that repository's rules, kept
//! here per repository path.
//!
//! An assignment is started from Review or Merger. The engine runs it on a
//! thread of its own ([`spagitty_farm::assign::engine`]); this module gathers
//! what it needs, refuses what no level allows before anything runs, keeps the
//! handle the person's controls go through, and forwards every change to the
//! webview as an event.
//!
//! # What is kept where
//!
//! - `agents.json` in the configuration directory: the machine list, the
//!   defaults and each repository's rules. Never a key.
//! - The OS keychain: each remote agent's key, under `agent:<id>`, beside the
//!   forge tokens. The webview never holds one; it is read here at request
//!   time.
//! - `assignments/` in the data directory: each assignment's record and raw
//!   transcript. Never sent anywhere.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use spagitty_core::forge::keychain;
use spagitty_core::models::{self, Endpoint, Provider};
use spagitty_farm::agent::{adapter_for, AgentRunRequest};
use spagitty_farm::assign::engine::{self, Control, Limits, Setup, Sink, Work};
use spagitty_farm::assign::level::{Job, Level};
use spagitty_farm::assign::local::{read_only, LocalDriver};
use spagitty_farm::assign::record::{AgentRef, Assignment, Reach, State as Life, Store, Target};
use spagitty_farm::assign::remote::RemoteDriver;
use spagitty_farm::assign::rules::RepoRules;
use spagitty_farm::assign::world::{self, Repository};
use spagitty_farm::execution::log::TranscriptWriter;
use spagitty_farm::execution::process::{self, Collected, Ended};
use spagitty_farm::model::{AgentAvailability, AgentDefinition, AgentProvider};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

pub const EVENT: &str = "assignment-event";
pub const LINE: &str = "assignment-line";
const FILE: &str = "agents.json";
/// The keychain "host" agent keys are filed under, beside the forge tokens.
const KEY_HOST: &str = "agent";

// ── Errors ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    kind: String,
    message: String,
}

impl Failure {
    fn new(kind: &str, message: impl Into<String>) -> Failure {
        Failure {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

impl From<spagitty_core::Error> for Failure {
    fn from(error: spagitty_core::Error) -> Self {
        let kind = match &error {
            spagitty_core::Error::Keychain(_) => "keychain",
            spagitty_core::Error::Model { .. } => "model",
            _ => "git",
        };
        Failure::new(kind, error.to_string())
    }
}

type Result<T> = std::result::Result<T, Failure>;

// ── The machine list ─────────────────────────────────────────────────────

/// What an agent may be offered for. Off means it is never offered for that
/// job: a switch about trust, not a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Jobs {
    pub review: bool,
    pub merge: bool,
    pub farm: bool,
}

impl Default for Jobs {
    fn default() -> Self {
        Jobs {
            review: true,
            merge: true,
            farm: true,
        }
    }
}

impl Jobs {
    fn allows(self, job: Job) -> bool {
        match job {
            Job::Review => self.review,
            Job::Merge => self.merge,
        }
    }
}

/// A model reached over an API. The key is not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAgent {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    /// Empty for the provider's own.
    #[serde(default)]
    pub base: String,
    pub model: String,
    #[serde(default)]
    pub jobs: Jobs,
    #[serde(default)]
    pub tokens_per_run: Option<u64>,
    #[serde(default)]
    pub tokens_per_day: Option<u64>,
    #[serde(default)]
    pub minutes: Option<u64>,
    /// The key's last four characters, to show that one is stored.
    #[serde(default)]
    pub key_end: Option<String>,
    /// Tokens spent today, and which day that was (days since the epoch).
    #[serde(default)]
    pub spent: u64,
    #[serde(default)]
    pub spent_day: u64,
}

impl RemoteAgent {
    fn slug(&self) -> &'static str {
        match self.provider {
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
            Provider::Google => "google",
            Provider::Compatible => "compatible",
        }
    }

    fn endpoint(&self, key: String) -> Endpoint {
        Endpoint {
            provider: self.provider,
            base: self.base.clone(),
            model: self.model.clone(),
            key,
        }
    }

    fn is_local(&self) -> bool {
        self.endpoint(String::new()).is_local()
    }
}

/// Where an assignment starts from, per job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Defaults {
    pub review: Option<String>,
    pub review_level: Level,
    pub merge: Option<String>,
    pub merge_level: Level,
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults {
            review: None,
            review_level: Level::StepByStep,
            merge: None,
            merge_level: Level::StepByStep,
        }
    }
}

/// Which moments become a notification while the window is not focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Notify {
    pub waiting: bool,
    pub finished: bool,
    pub stopped: bool,
}

impl Default for Notify {
    fn default() -> Self {
        Notify {
            waiting: true,
            finished: true,
            stopped: true,
        }
    }
}

/// `agents.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Machine {
    /// Opt-in: Codex commands run without its filesystem or network sandbox.
    pub codex_full_access: bool,
    /// Opt-in: headless agy runs approve tool permission requests.
    pub agy_auto_approve: bool,
    pub omp: OmpOptions,
    /// Per built-in or custom agent id.
    pub jobs: BTreeMap<String, Jobs>,
    /// Command-line agents added by hand.
    pub custom: Vec<AgentDefinition>,
    pub remote: Vec<RemoteAgent>,
    pub defaults: Defaults,
    pub notify: Notify,
    /// Each repository's rules, by its path on this machine.
    pub repos: BTreeMap<String, RepoRules>,
    /// The first-start offer of agents found in repositories was answered.
    pub offered: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OmpOptions {
    pub model: String,
    pub profile: String,
}

impl OmpOptions {
    fn apply(&self, definition: &mut AgentDefinition) {
        if definition.provider != AgentProvider::OhMyPi {
            return;
        }
        for (flag, value) in [("--model", &self.model), ("--profile", &self.profile)] {
            let value = value.trim();
            if !value.is_empty() {
                definition.extra_args.extend([flag.into(), value.into()]);
            }
        }
    }
}

impl Machine {
    fn jobs(&self, id: &str) -> Jobs {
        self.jobs.get(id).copied().unwrap_or_default()
    }

    fn rules(&self, repo: &str) -> RepoRules {
        self.repos.get(repo).cloned().unwrap_or_default()
    }
}

fn file<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

/// The machine list, for reading: a file that cannot be read reads as none.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Machine {
    file(app)
        .map(|path| read_at(&path).unwrap_or_default())
        .unwrap_or_default()
}

/// The file at `path`, telling *not there yet* (an empty list) apart from
/// *there and unreadable*. A change must not write an empty list over a file
/// it could not read: that would drop every agent, rule and consent in it.
fn read_at(path: &Path) -> Result<Machine> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            Failure::new(
                "io",
                format!(
                    "{} could not be read, so it was left as it is: {e}",
                    path.display()
                ),
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Machine::default()),
        Err(e) => Err(Failure::new("io", e.to_string())),
    }
}

fn save<R: Runtime>(app: &AppHandle<R>, machine: &Machine) -> Result<()> {
    let path = file(app).ok_or_else(|| Failure::new("io", "no configuration directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Failure::new("io", e.to_string()))?;
    }
    let json = serde_json::to_vec_pretty(machine).map_err(|e| Failure::new("io", e.to_string()))?;
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, json).map_err(|e| Failure::new("io", e.to_string()))?;
    std::fs::rename(partial, path).map_err(|e| Failure::new("io", e.to_string()))
}

/// Change the machine list under the lock, so two quick clicks do not each
/// write over the other.
fn change<R: Runtime>(
    app: &AppHandle<R>,
    state: &AgentsState,
    edit: impl FnOnce(&mut Machine) -> Result<()>,
) -> Result<Machine> {
    let _held = state.config.lock().expect("agents config lock");
    let mut machine = match file(app) {
        Some(path) => read_at(&path)?,
        None => Machine::default(),
    };
    edit(&mut machine)?;
    save(app, &machine)?;
    Ok(machine)
}

// ── What the section shows ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalView {
    pub id: String,
    pub name: String,
    pub provider: AgentProvider,
    pub availability: AgentAvailability,
    pub jobs: Jobs,
    pub custom: bool,
    pub definition: AgentDefinition,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteView {
    #[serde(flatten)]
    pub agent: RemoteAgent,
    /// The endpoint is on this machine: nothing leaves it.
    pub local: bool,
    pub provider_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsSnapshot {
    pub codex_full_access: bool,
    pub agy_auto_approve: bool,
    pub omp: OmpOptions,
    pub local: Vec<LocalView>,
    pub remote: Vec<RemoteView>,
    pub defaults: Defaults,
    pub notify: Notify,
    /// The open repository's rules, when one was named.
    pub rules: Option<RepoRules>,
    /// Agents found in repositories this machine knows, offered once.
    pub offer: Vec<AgentDefinition>,
}

fn locals(machine: &Machine) -> Vec<LocalView> {
    let mut out: Vec<LocalView> = AgentProvider::BUILT_IN
        .iter()
        .map(|provider| {
            let adapter = adapter_for(*provider);
            let availability = adapter.detect();
            let executable = availability
                .path()
                .cloned()
                .unwrap_or_else(|| PathBuf::from(adapter.executables()[0]));
            let mut definition = adapter.default_definition(executable);
            codex_access(&mut definition, machine.codex_full_access);
            agy_access(&mut definition, machine.agy_auto_approve);
            machine.omp.apply(&mut definition);
            let id = definition.id.as_str().to_string();
            LocalView {
                name: definition.display_name.clone(),
                jobs: machine.jobs(&id),
                provider: *provider,
                availability,
                custom: false,
                definition,
                id,
            }
        })
        .collect();
    for definition in &machine.custom {
        let id = definition.id.as_str().to_string();
        out.push(LocalView {
            name: definition.display_name.clone(),
            jobs: machine.jobs(&id),
            provider: AgentProvider::Custom,
            availability: adapter_for(AgentProvider::Custom).probe(&definition.executable),
            custom: true,
            definition: definition.clone(),
            id,
        });
    }
    out
}

fn remotes(machine: &Machine) -> Vec<RemoteView> {
    machine
        .remote
        .iter()
        .map(|agent| RemoteView {
            local: agent.is_local(),
            provider_label: agent.provider.label().to_string(),
            agent: agent.clone(),
        })
        .collect()
}

fn codex_access(definition: &mut AgentDefinition, full_access: bool) {
    if definition.provider == AgentProvider::Codex && full_access {
        definition
            .extra_args
            .extend(["--sandbox".into(), "danger-full-access".into()]);
    }
}

fn agy_access(definition: &mut AgentDefinition, auto_approve: bool) {
    if definition.provider == AgentProvider::Agy && auto_approve {
        definition
            .extra_args
            .push("--dangerously-skip-permissions".into());
    }
}

/// Custom agents in the farm registries of repositories this machine knows,
/// not yet on the machine list. Built-in ones are detected anyway.
fn offer<R: Runtime>(app: &AppHandle<R>, machine: &Machine) -> Vec<AgentDefinition> {
    if machine.offered {
        return Vec::new();
    }
    let mut found: Vec<AgentDefinition> = Vec::new();
    for repo in crate::recents::load(app) {
        let Some(registry) = spagitty_farm::persistence::store::load_registry::<
            spagitty_farm::agent::AgentRegistry,
        >(&repo) else {
            continue;
        };
        for status in registry.statuses() {
            let definition = status.definition;
            if definition.provider == AgentProvider::Custom
                && !machine.custom.iter().any(|d| d.id == definition.id)
                && !found.iter().any(|d| d.id == definition.id)
            {
                found.push(definition);
            }
        }
    }
    found
}

#[tauri::command(async)]
pub fn agents_snapshot<R: Runtime>(
    app: AppHandle<R>,
    repo: Option<String>,
) -> Result<AgentsSnapshot> {
    let machine = load(&app);
    Ok(AgentsSnapshot {
        codex_full_access: machine.codex_full_access,
        agy_auto_approve: machine.agy_auto_approve,
        omp: machine.omp.clone(),
        local: locals(&machine),
        remote: remotes(&machine),
        defaults: machine.defaults.clone(),
        notify: machine.notify,
        rules: repo.map(|repo| machine.rules(&repo)),
        offer: offer(&app, &machine),
    })
}

#[tauri::command(async)]
pub fn agents_set_codex_full_access<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    enabled: bool,
) -> Result<()> {
    change(&app, &state, |machine| {
        machine.codex_full_access = enabled;
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_set_agy_auto_approve<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    enabled: bool,
) -> Result<()> {
    change(&app, &state, |machine| {
        machine.agy_auto_approve = enabled;
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_set_omp_options<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    options: OmpOptions,
) -> Result<()> {
    change(&app, &state, |machine| {
        machine.omp = OmpOptions {
            model: options.model.trim().into(),
            profile: options.profile.trim().into(),
        };
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_set_jobs<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    id: String,
    jobs: Jobs,
) -> Result<()> {
    change(&app, &state, |machine| {
        if let Some(remote) = machine.remote.iter_mut().find(|r| r.id == id) {
            remote.jobs = Jobs {
                farm: false,
                ..jobs
            };
        } else {
            machine.jobs.insert(id, jobs);
        }
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_save_custom<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    definition: AgentDefinition,
) -> Result<()> {
    if definition.provider != AgentProvider::Custom {
        return Err(Failure::new(
            "refused",
            "only a custom agent is added by hand",
        ));
    }
    change(&app, &state, |machine| {
        machine.custom.retain(|d| d.id != definition.id);
        machine.custom.push(definition);
        Ok(())
    })?;
    Ok(())
}

/// Answer the first-start offer: add these, or none, and do not ask again.
#[tauri::command(async)]
pub fn agents_take_offer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    take: Vec<AgentDefinition>,
) -> Result<()> {
    change(&app, &state, |machine| {
        for definition in take {
            if definition.provider == AgentProvider::Custom
                && !machine.custom.iter().any(|d| d.id == definition.id)
            {
                machine.custom.push(definition);
            }
        }
        machine.offered = true;
        Ok(())
    })?;
    Ok(())
}

/// What a remote agent is saved with. `key` is only ever on the way in.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub provider: Provider,
    #[serde(default)]
    pub base: String,
    pub model: String,
    #[serde(default)]
    pub jobs: Option<Jobs>,
    #[serde(default)]
    pub tokens_per_run: Option<u64>,
    #[serde(default)]
    pub tokens_per_day: Option<u64>,
    #[serde(default)]
    pub minutes: Option<u64>,
    /// A new key. `None` keeps the one stored.
    #[serde(default)]
    pub key: Option<String>,
}

fn last_four(key: &str) -> Option<String> {
    let chars: Vec<char> = key.trim().chars().collect();
    (chars.len() >= 8).then(|| chars[chars.len() - 4..].iter().collect())
}

#[tauri::command(async)]
pub fn agents_save_remote<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    agent: RemoteInput,
) -> Result<RemoteView> {
    if agent.model.trim().is_empty() {
        return Err(Failure::new("refused", "an API agent needs a model"));
    }
    let machine = change(&app, &state, |machine| {
        let id = agent.id.clone().unwrap_or_else(|| {
            let mut n = machine.remote.len() + 1;
            while machine.remote.iter().any(|r| r.id == format!("api-{n}")) {
                n += 1;
            }
            format!("api-{n}")
        });
        let previous = machine.remote.iter().find(|r| r.id == id).cloned();
        let mut key_end = previous.as_ref().and_then(|p| p.key_end.clone());
        if let Some(key) = agent
            .key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
        {
            keychain::store(KEY_HOST, &id, key)?;
            key_end = last_four(key);
        }
        let saved = RemoteAgent {
            id: id.clone(),
            name: if agent.name.trim().is_empty() {
                agent.provider.label().to_string()
            } else {
                agent.name.trim().to_string()
            },
            provider: agent.provider,
            base: agent.base.trim().to_string(),
            model: agent.model.trim().to_string(),
            jobs: agent
                .jobs
                .map(|j| Jobs { farm: false, ..j })
                .unwrap_or(Jobs {
                    review: true,
                    merge: true,
                    farm: false,
                }),
            tokens_per_run: agent.tokens_per_run,
            tokens_per_day: agent.tokens_per_day,
            minutes: agent.minutes,
            key_end,
            spent: previous.as_ref().map(|p| p.spent).unwrap_or(0),
            spent_day: previous.as_ref().map(|p| p.spent_day).unwrap_or(0),
        };
        machine.remote.retain(|r| r.id != id);
        machine.remote.push(saved);
        Ok(())
    })?;
    let saved = machine
        .remote
        .last()
        .cloned()
        .ok_or_else(|| Failure::new("io", "not saved"))?;
    Ok(RemoteView {
        local: saved.is_local(),
        provider_label: saved.provider.label().to_string(),
        agent: saved,
    })
}

/// Remove an agent: a custom or remote one from the list, with its key.
#[tauri::command(async)]
pub fn agents_remove<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    id: String,
) -> Result<()> {
    change(&app, &state, |machine| {
        if machine.remote.iter().any(|r| r.id == id) {
            let _ = keychain::forget(KEY_HOST, &id);
            machine.remote.retain(|r| r.id != id);
        }
        machine.custom.retain(|d| d.id.as_str() != id);
        machine.jobs.remove(&id);
        for rules in machine.repos.values_mut() {
            if let Some(list) = &mut rules.agents {
                list.retain(|a| a != &id);
            }
        }
        if machine.defaults.review.as_deref() == Some(id.as_str()) {
            machine.defaults.review = None;
        }
        if machine.defaults.merge.as_deref() == Some(id.as_str()) {
            machine.defaults.merge = None;
        }
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_set_defaults<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    defaults: Defaults,
    notify: Option<Notify>,
) -> Result<()> {
    change(&app, &state, |machine| {
        machine.defaults = defaults;
        if let Some(notify) = notify {
            machine.notify = notify;
        }
        Ok(())
    })?;
    Ok(())
}

#[tauri::command(async)]
pub fn agents_set_rules<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    repo: String,
    rules: RepoRules,
) -> Result<()> {
    change(&app, &state, |machine| {
        machine.repos.insert(repo, rules);
        Ok(())
    })?;
    Ok(())
}

/// Agree, once, that this repository's code may go to `provider`.
#[tauri::command(async)]
pub fn agents_consent<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    repo: String,
    provider: String,
) -> Result<()> {
    change(&app, &state, |machine| {
        let rules = machine.repos.entry(repo).or_default();
        if !rules.consent.contains(&provider) {
            rules.consent.push(provider);
        }
        Ok(())
    })?;
    Ok(())
}

// ── Test ─────────────────────────────────────────────────────────────────

/// What *Test* found: what answered, and how long it took, or the provider's
/// own sentence.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tested {
    pub ok: bool,
    pub said: String,
    pub ms: u64,
}

const TEST_PROMPT: &str = "Reply with the single word: ok";
const TEST_TIMEOUT: Duration = Duration::from_secs(90);

/// Run a local agent once on a tiny prompt in a temporary directory.
#[tauri::command(async)]
pub fn agents_test_local<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Tested> {
    let machine = load(&app);
    let view = locals(&machine)
        .into_iter()
        .find(|view| view.id == id)
        .ok_or_else(|| Failure::new("notFound", "no such agent"))?;
    if matches!(view.availability, AgentAvailability::Missing) {
        return Err(Failure::new(
            "refused",
            format!("{} is not installed", view.name),
        ));
    }
    let dir = tempfile_dir()?;
    let adapter = adapter_for(view.provider);
    let request = AgentRunRequest {
        workdir: dir.clone(),
        prompt: TEST_PROMPT.into(),
        unattended: false,
    };
    let command = read_only(view.provider, adapter.command(&view.definition, &request));
    let started = Instant::now();
    let collected = Arc::new(Collected::default());
    let transcript = TranscriptWriter::create(&dir.join("test.log"))
        .map_err(|e| Failure::new("io", e.to_string()))?;
    let session = process::start(
        &command,
        &dir,
        transcript,
        collected.clone(),
        adapter.narrator(),
    )
    .map_err(|e| Failure::new("refused", e.to_string()))?;
    let cancel = session.cancellation();
    let timer = std::thread::spawn(move || {
        let deadline = Instant::now() + TEST_TIMEOUT;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
            if cancel.was_cancelled() {
                return;
            }
        }
        cancel.cancel();
    });
    let ended = session.wait();
    drop(timer);
    let _ = std::fs::remove_dir_all(&dir);
    let lines = collected.lines();
    Ok(local_test_result(
        ended,
        &lines,
        started.elapsed().as_millis() as u64,
    ))
}

fn local_test_result(ended: Ended, lines: &[String], ms: u64) -> Tested {
    let said = lines
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .cloned()
        .unwrap_or_default();
    let answered = lines
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case("ok"));
    match ended {
        Ended::Ok if answered => Tested { ok: true, said, ms },
        Ended::Ok => Tested {
            ok: false,
            said: if said.is_empty() {
                "The agent exited without answering the test.".into()
            } else {
                said
            },
            ms,
        },
        Ended::Cancelled => Tested {
            ok: false,
            said: format!("No answer in {} seconds", TEST_TIMEOUT.as_secs()),
            ms,
        },
        Ended::Failed { code, message } => Tested {
            ok: false,
            said: if said.is_empty() {
                code.map(|c| format!("Exited with {c}")).unwrap_or(message)
            } else {
                said
            },
            ms,
        },
    }
}

fn tempfile_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "spagitty-agent-test-{}-{}",
        std::process::id(),
        nanos()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| Failure::new("io", e.to_string()))?;
    Ok(dir)
}

/// The key to use: the one just typed, or the stored one for `id`.
fn key_for(id: Option<&str>, typed: Option<String>) -> Result<String> {
    if let Some(key) = typed
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
    {
        return Ok(key);
    }
    match id {
        Some(id) => Ok(keychain::read(KEY_HOST, id)?.unwrap_or_default()),
        None => Ok(String::new()),
    }
}

/// What an endpoint is tested and listed with, before or after it is saved.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    #[serde(default)]
    pub id: Option<String>,
    pub provider: Provider,
    #[serde(default)]
    pub base: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub key: Option<String>,
}

impl Probe {
    fn endpoint(&self) -> Result<Endpoint> {
        Ok(Endpoint {
            provider: self.provider,
            base: self.base.clone(),
            model: self.model.clone(),
            key: key_for(self.id.as_deref(), self.key.clone())?,
        })
    }
}

#[tauri::command(async)]
pub fn agents_models(probe: Probe) -> Result<Vec<String>> {
    Ok(models::models(&probe.endpoint()?)?)
}

#[tauri::command(async)]
pub fn agents_test_remote(probe: Probe) -> Result<Tested> {
    let endpoint = probe.endpoint()?;
    Ok(match models::test(&endpoint) {
        Ok(took) => Tested {
            ok: true,
            said: format!("{} answered", endpoint.model),
            ms: took.as_millis() as u64,
        },
        Err(error) => Tested {
            ok: false,
            said: error.to_string(),
            ms: 0,
        },
    })
}

// ── Assignments ──────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AgentsState {
    config: Mutex<()>,
    live: Mutex<HashMap<String, engine::Handle>>,
}

pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(AgentsState::default());
}

fn store<R: Runtime>(app: &AppHandle<R>) -> Result<Store> {
    app.path()
        .app_data_dir()
        .map(|dir| Store::new(dir.join("assignments")))
        .map_err(|e| Failure::new("io", e.to_string()))
}

struct Emit<R: Runtime> {
    app: AppHandle<R>,
}

#[derive(Clone, Serialize)]
struct Line<'a> {
    id: &'a str,
    line: &'a str,
}

impl<R: Runtime> Sink for Emit<R> {
    fn changed(&self, assignment: &Assignment) {
        let _ = self.app.emit(EVENT, assignment);
    }
    fn line(&self, id: &str, line: &str) {
        let _ = self.app.emit(LINE, Line { id, line });
    }
}

/// What the webview hands over to start an assignment.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub repo: String,
    pub agent: String,
    pub level: Level,
    #[serde(default)]
    pub note: String,
    pub target: Target,
    pub work: Work,
    /// For a merge: resolve, check and land, rather than resolve only.
    #[serde(default)]
    pub lands: bool,
    /// Resume this assignment: a new run from its last finished step, told
    /// what was already done. Its record — steps, proposals, decisions — is
    /// kept and carried on.
    #[serde(default)]
    pub resume: Option<String>,
}

/// An assignment carried on from its record: what it made is kept, the step
/// that was cut short is marked superseded, and the agent is told how far
/// the earlier run got.
pub fn resumed(mut kept: Assignment, level: Level, note: &str) -> Assignment {
    use spagitty_farm::assign::record::{StepKind, StepState};
    let unit = match kept.job {
        Job::Review => "file",
        Job::Merge => "conflict",
    };
    let done = kept
        .steps
        .iter()
        .filter(|s| {
            s.state == StepState::Done
                && matches!(s.kind, StepKind::File { .. } | StepKind::Conflict { .. })
        })
        .count();
    for step in &mut kept.steps {
        if matches!(step.state, StepState::Running | StepState::Waiting) {
            step.state = StepState::Superseded;
            step.gate = None;
        }
    }
    let mut told = note.trim().to_string();
    if done > 0 {
        if !told.is_empty() {
            told.push(' ');
        }
        told.push_str(&format!(
            "This carries on an earlier run, which finished {}: do not do those again.",
            engine::plural(done, unit)
        ));
    }
    kept.note = told;
    kept.level = level;
    kept.state = Life::Starting;
    kept.sentence = "Resuming".into();
    kept.ended_at = None;
    kept.reason = None;
    kept.took_over = None;
    kept.last_act = None;
    kept.pausing = false;
    kept.quiet_since = None;
    kept
}

/// Two assignments on one job are left for later: one per pull request, one
/// per merge.
fn same_job(a: &Target, b: &Target) -> bool {
    match (a, b) {
        (
            Target::Review {
                host,
                owner,
                name,
                number,
                ..
            },
            Target::Review {
                host: h2,
                owner: o2,
                name: n2,
                number: m2,
                ..
            },
        ) => host == h2 && owner == o2 && name == n2 && number == m2,
        (Target::Merge { a, b, .. }, Target::Merge { a: a2, b: b2, .. }) => a == a2 && b == b2,
        _ => false,
    }
}

fn days_now() -> u64 {
    spagitty_farm::assign::record::now() / 86_400
}

#[tauri::command(async)]
pub fn assignment_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    request: StartRequest,
) -> Result<Assignment> {
    let machine = load(&app);
    let job = request.target.job();
    let rules = machine.rules(&request.repo);
    if !rules.allows(&request.agent) {
        return Err(Failure::new(
            "refused",
            "This repository does not allow that agent.",
        ));
    }
    {
        let live = state.live.lock().expect("assignments lock");
        if live.values().any(|handle| {
            let a = handle.snapshot();
            !a.state.is_over() && a.repo == request.repo && same_job(&a.target, &request.target)
        }) {
            return Err(Failure::new(
                "refused",
                "An agent is already assigned to this.",
            ));
        }
    }

    let store = store(&app)?;
    let kept = request
        .resume
        .as_deref()
        .and_then(|id| store.load(&request.repo, id))
        .filter(|kept| kept.state.is_over() && kept.agent.id == request.agent);
    let id = match &kept {
        Some(kept) => kept.id.clone(),
        None => format!(
            "{}-{:08x}",
            job_word(job),
            spagitty_farm::assign::record::fnv(format!("{}{}", request.repo, nanos()).as_bytes())
                as u32
        ),
    };
    let level = rules.cap(request.level);

    let (agent, driver, limits): (AgentRef, Box<dyn engine::Driver>, Limits) = if let Some(remote) =
        machine.remote.iter().find(|r| r.id == request.agent)
    {
        if !remote.jobs.allows(job) {
            return Err(Failure::new(
                "refused",
                format!("{} is not offered for this job.", remote.name),
            ));
        }
        if !remote.is_local() && !rules.consented(remote.slug()) {
            return Err(Failure::new(
                    "consent",
                    format!(
                        "This sends parts of this repository — the changed files, and any others the agent asks to read — to {}.",
                        remote.provider.label()
                    ),
                ));
        }
        let mut per_run = remote.tokens_per_run;
        if let Some(per_day) = remote.tokens_per_day {
            let spent = if remote.spent_day == days_now() {
                remote.spent
            } else {
                0
            };
            let left = per_day.saturating_sub(spent);
            if left == 0 {
                return Err(Failure::new(
                    "refused",
                    format!("{} has used its tokens for today.", remote.name),
                ));
            }
            per_run = Some(per_run.map_or(left, |run| run.min(left)));
        }
        let key = keychain::read(KEY_HOST, &remote.id)?.unwrap_or_default();
        (
            AgentRef {
                id: remote.id.clone(),
                name: remote.name.clone(),
                reach: Reach::Remote,
                version: None,
                provider: remote.slug().into(),
                model: Some(remote.model.clone()),
            },
            Box::new(RemoteDriver::new(remote.endpoint(key), remote.name.clone())),
            Limits {
                tokens: per_run,
                minutes: remote.minutes,
            },
        )
    } else {
        let view = locals(&machine)
            .into_iter()
            .find(|view| view.id == request.agent)
            .ok_or_else(|| Failure::new("notFound", "no such agent"))?;
        if !view.jobs.allows(job) {
            return Err(Failure::new(
                "refused",
                format!("{} is not offered for this job.", view.name),
            ));
        }
        let version = match &view.availability {
            AgentAvailability::Available { version, .. } => Some(version.clone()),
            AgentAvailability::Broken { reason, .. } => {
                return Err(Failure::new(
                    "refused",
                    format!("{} does not run: {reason}", view.name),
                ))
            }
            AgentAvailability::Missing => {
                return Err(Failure::new(
                    "refused",
                    format!("{} is not installed.", view.name),
                ))
            }
        };
        let logs = store.dir(&request.repo, &id);
        (
            AgentRef {
                id: view.id.clone(),
                name: view.name.clone(),
                reach: Reach::Local,
                version,
                provider: serde_json::to_value(view.provider)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
                model: None,
            },
            Box::new(LocalDriver::new(view.definition.clone(), logs)),
            Limits::default(),
        )
    };

    // An agent never reviews its own work.
    if let Target::Review { base, head, .. } = &request.target {
        if world::wrote_commits(Path::new(&request.repo), base, head, &[agent.name.as_str()]) {
            return Err(Failure::new(
                "refused",
                format!(
                    "{} wrote commits in this pull request, so it cannot review it.",
                    agent.name
                ),
            ));
        }
    }

    let world = Repository::open(Path::new(&request.repo), &request.target, &id)
        .map_err(|e| Failure::new("git", e))?;
    let fresh = Assignment {
        id: id.clone(),
        repo: request.repo.clone(),
        job,
        agent,
        level,
        levels: Vec::new(),
        target: request.target.clone(),
        lands: request.lands,
        state: Life::Starting,
        sentence: "Starting".into(),
        note: request.note.clone(),
        steps: Vec::new(),
        proposals: Vec::new(),
        planned: 0,
        tokens: Default::default(),
        started_at: spagitty_farm::assign::record::now(),
        ended_at: None,
        reason: None,
        took_over: None,
        last_act: None,
        pausing: false,
        quiet_since: None,
    };
    let assignment = match kept {
        Some(kept) => resumed(kept, level, &request.note),
        None => fresh,
    };
    let _ = store.save(&assignment);
    let handle = engine::start(Setup {
        assignment: assignment.clone(),
        work: request.work,
        rules,
        limits,
        world: Box::new(world),
        driver,
        store,
        sink: Arc::new(Emit { app: app.clone() }),
    });
    state
        .live
        .lock()
        .expect("assignments lock")
        .insert(id.clone(), handle.clone());
    if limits.tokens.is_some() {
        spawn_spend_keeper(app.clone(), handle);
    }
    Ok(assignment)
}

/// Count a remote assignment's tokens against its agent's day, once it ends.
fn spawn_spend_keeper<R: Runtime>(app: AppHandle<R>, handle: engine::Handle) {
    std::thread::spawn(move || {
        while !handle.is_over() {
            std::thread::sleep(Duration::from_secs(2));
        }
        let done = handle.snapshot();
        if let Some(state) = app.try_state::<AgentsState>() {
            let _ = change(&app, &state, |machine| {
                if let Some(remote) = machine.remote.iter_mut().find(|r| r.id == done.agent.id) {
                    let today = days_now();
                    if remote.spent_day != today {
                        remote.spent_day = today;
                        remote.spent = 0;
                    }
                    remote.spent += done.tokens.total();
                }
                Ok(())
            });
        }
    });
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

fn job_word(job: Job) -> &'static str {
    match job {
        Job::Review => "review",
        Job::Merge => "merge",
    }
}

#[tauri::command(async)]
pub fn assignment_control(
    state: State<'_, AgentsState>,
    id: String,
    control: Control,
) -> Result<()> {
    let handle = state
        .live
        .lock()
        .expect("assignments lock")
        .get(&id)
        .cloned()
        .ok_or_else(|| Failure::new("notFound", "That assignment is not running."))?;
    handle.send(control);
    Ok(())
}

/// Every assignment kept for a repository, live ones as they are now. One
/// whose process did not outlive Spagitty reads *Stopped when Spagitty
/// closed*.
#[tauri::command(async)]
pub fn assignment_list<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    repo: String,
) -> Result<Vec<Assignment>> {
    let store = store(&app)?;
    let live = state.live.lock().expect("assignments lock");
    Ok(store
        .list(&repo)
        .into_iter()
        .map(|kept| match live.get(&kept.id) {
            Some(handle) => handle.snapshot(),
            None if !kept.state.is_over() => {
                let mut closed = kept;
                closed.state = Life::Stopped;
                closed.reason = Some("Stopped when Spagitty closed.".into());
                closed.sentence = "Stopped when Spagitty closed".into();
                closed.last_act = None;
                let _ = store.save(&closed);
                closed
            }
            None => kept,
        })
        .collect())
}

#[tauri::command(async)]
pub fn assignment_transcript<R: Runtime>(
    app: AppHandle<R>,
    repo: String,
    id: String,
) -> Result<String> {
    Ok(store(&app)?.read_transcript(&repo, &id))
}

/// Forget a finished assignment's record. Its proposals already made into
/// the person's material stay where they are.
#[tauri::command(async)]
pub fn assignment_forget<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AgentsState>,
    repo: String,
    id: String,
) -> Result<()> {
    let mut live = state.live.lock().expect("assignments lock");
    if live.get(&id).is_some_and(|handle| !handle.is_over()) {
        return Err(Failure::new("refused", "Stop the agent first."));
    }
    live.remove(&id);
    let dir = store(&app)?.dir(&repo, &id);
    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

/// Stop everything when the window goes: a local agent's process does not
/// outlive Spagitty.
pub fn shutdown<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AgentsState>() {
        for handle in state.live.lock().expect("assignments lock").values() {
            handle.send(Control::Stop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_file_from_nothing_is_the_safe_one() {
        let machine: Machine = serde_json::from_str("{}").unwrap();
        assert!(machine.jobs("claude").review);
        assert_eq!(machine.defaults.review_level, Level::StepByStep);
        assert_eq!(machine.rules("/work/app").highest, Level::SignOff);
        assert!(machine.notify.waiting);
        assert!(!machine.codex_full_access);
        assert!(!machine.agy_auto_approve);
        assert_eq!(machine.omp, OmpOptions::default());
    }

    #[test]
    fn agy_auto_approval_is_saved_and_keeps_review_plan_mode() {
        let machine: Machine = serde_json::from_str(r#"{"agyAutoApprove":true}"#).unwrap();
        assert_eq!(
            serde_json::to_value(&machine).unwrap()["agyAutoApprove"],
            true
        );
        let mut definition = adapter_for(AgentProvider::Agy).default_definition("agy".into());
        agy_access(&mut definition, machine.agy_auto_approve);
        let command = read_only(
            AgentProvider::Agy,
            adapter_for(AgentProvider::Agy).command(
                &definition,
                &AgentRunRequest {
                    workdir: "/tmp/t".into(),
                    prompt: "ok".into(),
                    unattended: false,
                },
            ),
        );
        assert_eq!(
            command.args,
            [
                "--mode",
                "plan",
                "--dangerously-skip-permissions",
                "--print",
                "ok"
            ]
        );
        let mut other = adapter_for(AgentProvider::Codex).default_definition("codex".into());
        agy_access(&mut other, true);
        assert!(other.extra_args.is_empty());
    }

    #[test]
    fn omp_model_and_profile_survive_save_and_reach_the_headless_command() {
        let machine: Machine =
            serde_json::from_str(r#"{"omp":{"model":"provider/test-model","profile":"work"}}"#)
                .unwrap();
        let saved: Machine =
            serde_json::from_slice(&serde_json::to_vec(&machine).unwrap()).unwrap();
        let mut definition = adapter_for(AgentProvider::OhMyPi).default_definition("omp".into());
        saved.omp.apply(&mut definition);
        let command = read_only(
            AgentProvider::OhMyPi,
            adapter_for(AgentProvider::OhMyPi).command(
                &definition,
                &AgentRunRequest {
                    workdir: "/tmp/test".into(),
                    prompt: "ok".into(),
                    unattended: false,
                },
            ),
        );
        assert_eq!(command.stdin.as_deref(), Some("ok"));
        assert_eq!(
            command.args,
            [
                "--print",
                "--model",
                "provider/test-model",
                "--profile",
                "work",
                "--tools",
                "read,grep,glob"
            ]
        );
        let mut other = adapter_for(AgentProvider::Agy).default_definition("agy".into());
        saved.omp.apply(&mut other);
        assert!(other.extra_args.is_empty());
        let mut unset = adapter_for(AgentProvider::OhMyPi).default_definition("omp".into());
        OmpOptions::default().apply(&mut unset);
        assert!(unset.extra_args.is_empty());
    }

    #[test]
    fn full_access_is_saved_and_used_by_codex_only() {
        let machine: Machine = serde_json::from_str(r#"{"codexFullAccess":true}"#).unwrap();
        assert!(machine.codex_full_access);
        assert_eq!(
            serde_json::to_value(&machine).unwrap()["codexFullAccess"],
            true
        );
        let mut codex = adapter_for(AgentProvider::Codex).default_definition("codex".into());
        codex_access(&mut codex, true);
        let request = AgentRunRequest {
            workdir: "/tmp/test".into(),
            prompt: "ok".into(),
            unattended: false,
        };
        let command = read_only(
            AgentProvider::Codex,
            adapter_for(AgentProvider::Codex).command(&codex, &request),
        );
        assert_eq!(
            command
                .args
                .iter()
                .filter(|arg| arg.as_str() == "--sandbox")
                .count(),
            1
        );
        assert!(command.args.iter().any(|arg| arg == "danger-full-access"));
        let mut agy = adapter_for(AgentProvider::Agy).default_definition("agy".into());
        codex_access(&mut agy, true);
        assert!(agy.extra_args.is_empty());
    }

    /// A file that is there but does not parse is an error to a change, not
    /// an empty list to write back over it; one that is not there yet is.
    #[test]
    fn an_unreadable_machine_file_is_not_taken_for_an_empty_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        assert!(read_at(&path).unwrap().remote.is_empty());

        std::fs::write(
            &path,
            br#"{"defaults": {"reviewLevel": "fromANewerBuild"}}"#,
        )
        .unwrap();
        let refused = read_at(&path).unwrap_err();
        assert!(
            refused.message.contains("left as it is"),
            "{}",
            refused.message
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"defaults": {"reviewLevel": "fromANewerBuild"}}"#
        );
    }

    #[test]
    fn the_machine_file_never_holds_a_key() {
        let machine = Machine {
            remote: vec![RemoteAgent {
                id: "api-1".into(),
                name: "Anthropic".into(),
                provider: Provider::Anthropic,
                base: String::new(),
                model: "claude-sonnet-5-5".into(),
                jobs: Jobs::default(),
                tokens_per_run: Some(40_000),
                tokens_per_day: None,
                minutes: None,
                key_end: last_four("sk-ant-0123456789abcd"),
                spent: 0,
                spent_day: 0,
            }],
            ..Machine::default()
        };
        let text = serde_json::to_string(&machine).unwrap();
        assert!(!text.contains("sk-ant"));
        assert!(text.contains("\"keyEnd\":\"abcd\""));
    }

    #[test]
    fn a_short_key_shows_no_ending() {
        assert_eq!(last_four("abc"), None);
        assert_eq!(last_four("  0123456789  "), Some("6789".into()));
    }

    #[test]
    fn one_job_is_one_pull_request_or_one_merge() {
        let review = |number| Target::Review {
            host: "github.com".into(),
            owner: "o".into(),
            name: "n".into(),
            number,
            title: String::new(),
            base: "a".into(),
            head: "b".into(),
            target: "main".into(),
        };
        assert!(same_job(&review(1), &review(1)));
        assert!(!same_job(&review(1), &review(2)));
        let merge = |b: &str| Target::Merge {
            a: "main".into(),
            b: b.into(),
            a_tip: "1".into(),
            b_tip: "2".into(),
            base: "0".into(),
            strategy: String::new(),
            into: String::new(),
        };
        assert!(same_job(&merge("feat"), &merge("feat")));
        assert!(!same_job(&merge("feat"), &review(1)));
    }

    #[test]
    fn a_resumed_assignment_keeps_its_work_and_is_told_how_far_it_got() {
        use spagitty_farm::assign::record::{Step, StepKind, StepState};
        let step = |index, kind, state| Step {
            index,
            kind,
            label: String::new(),
            state,
            started_at: 0,
            ended_at: None,
            gate: None,
            events: Vec::new(),
            sent: Vec::new(),
            refused: Vec::new(),
            note: None,
            command: None,
            checks: Vec::new(),
            tokens: Default::default(),
        };
        let mut kept: Assignment = serde_json::from_value(serde_json::json!({
            "id": "review-1", "repo": "/w", "job": "review",
            "agent": {"id": "claude", "name": "Claude Code", "reach": "local", "provider": "claudeCode"},
            "level": "signOff", "state": "stopped", "sentence": "Stopped when Spagitty closed",
            "reason": "Stopped when Spagitty closed.", "startedAt": 1,
            "target": {"kind": "review", "host": "h", "owner": "o", "name": "n", "number": 1, "title": "t", "base": "a", "head": "b"}
        }))
        .unwrap();
        kept.steps = vec![
            step(
                0,
                StepKind::File {
                    path: "a.rs".into(),
                },
                StepState::Done,
            ),
            step(
                1,
                StepKind::File {
                    path: "b.rs".into(),
                },
                StepState::Running,
            ),
        ];
        let next = resumed(kept, Level::StepByStep, "mind the cache");
        assert_eq!(next.state, Life::Starting);
        assert_eq!(next.level, Level::StepByStep);
        assert_eq!(next.reason, None);
        assert_eq!(next.steps[1].state, StepState::Superseded);
        assert_eq!(
            next.note,
            "mind the cache This carries on an earlier run, which finished 1 file: do not do those again."
        );
    }

    #[test]
    fn a_local_endpoint_needs_no_consent() {
        let mut agent = RemoteAgent {
            id: "api-1".into(),
            name: "Ollama".into(),
            provider: Provider::Compatible,
            base: "http://localhost:11434/v1".into(),
            model: "qwen3-coder:30b".into(),
            jobs: Jobs::default(),
            tokens_per_run: None,
            tokens_per_day: None,
            minutes: None,
            key_end: None,
            spent: 0,
            spent_day: 0,
        };
        assert!(agent.is_local());
        agent.base = "https://openrouter.ai/api/v1".into();
        assert!(!agent.is_local());
    }

    #[test]
    fn a_successful_exit_without_the_test_answer_is_not_a_pass() {
        let failed = local_test_result(Ended::Ok, &["No default model selected".into()], 12);
        assert!(!failed.ok);
        assert_eq!(failed.said, "No default model selected");
        assert!(!local_test_result(Ended::Ok, &[], 12).ok);
        assert!(
            local_test_result(
                Ended::Ok,
                &[
                    "codex".into(),
                    "ok".into(),
                    "tokens used".into(),
                    "100".into()
                ],
                12,
            )
            .ok
        );
    }
}

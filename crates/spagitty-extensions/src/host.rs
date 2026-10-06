// SPDX-License-Identifier: GPL-3.0-or-later

//! The extension host: the API the desktop calls, and the answers it gives
//! workers.
//!
//! # Locking
//!
//! A handful of small mutexes, each held only while state is read or written,
//! never across a process spawn or a wait on a worker. Starting a worker takes
//! a per-extension start lock so two first uses cannot start two processes.
//!
//! # What a callback is checked against
//!
//! Every request a worker makes is answered by [`Link::request`], which checks,
//! in order: that the extension's current session is the one asking and is
//! starting or active; that the handle it names was minted for this extension
//! and session and has not expired; that the capability the method needs is
//! granted to this extension, at this version, for the repository the handle
//! belongs to; and — where the method names one — that the operation is running
//! and is this extension's. Only then does it do anything.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::capabilities::Capability;
use crate::history::{self, Requester, ReviewRecord};
use crate::manifest::{Context, Manifest, ReviewScope, ReviewTarget, SettingScope};
use crate::operations::{self, Expiry, Kind, Operation, Operations};
use crate::protocol::{code, RpcError};
use crate::redact::redact;
use crate::registry::{self, Entry, Paths, Provenance, Unreadable};
use crate::review::{
    self, ActionRequired, Completeness, GatePolicy, ReviewFinding, ReviewResult, RunStatus,
};
use crate::snapshot::{self, Preview};
use crate::storage::{self, Consent, Store};
use crate::tools::{self, Detected};
use crate::version::{self, Compatibility};
use crate::worker::{Inbound, Worker};
use crate::{package, Error, Result};

pub const HANDSHAKE: Duration = Duration::from_secs(10);
pub const PANEL_TIMEOUT: Duration = Duration::from_secs(30);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Three crashes inside this window keep an extension stopped until a person
/// restarts it.
pub const CRASH_WINDOW: Duration = Duration::from_secs(600);
pub const CRASH_LIMIT: usize = 3;
pub const MAX_LOGS: usize = 200;
pub const STORAGE_VALUE: usize = 64 * 1024;
pub const STORAGE_KEYS: usize = 256;

/// What the trust model is, in the words the interface uses.
pub const TRUST: &str = "Extensions are programs that run on this computer with your permissions. \
Spagitty only does for an extension what you grant it, but an extension can still read your files \
and use the network on its own. Install only extensions you trust.";

// ── What the desktop supplies ──────────────────────────────────────────────

/// The services only the desktop can provide: anything needing the forge
/// token, which never leaves the backend, or the person, who is in the window.
pub trait Services: Send + Sync {
    /// A pull request snapshot (FEAT-099) for the repository at `workdir`.
    fn pull_request_snapshot(
        &self,
        workdir: &Path,
        number: u64,
    ) -> std::result::Result<Value, RpcError>;
    /// Show the person the exact `body` for pull request `number`, and post it
    /// only if they agree.
    fn post_pull_request_comment(
        &self,
        workdir: &Path,
        number: u64,
        body: &str,
        extension_name: &str,
        cancel: &AtomicBool,
    ) -> std::result::Result<Value, RpcError>;
}

/// Where the host's events go. The desktop forwards them to the window.
pub trait Events: Send + Sync {
    fn emit(&self, event: HostEvent);
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum HostEvent {
    /// Something about this extension changed; read the list again.
    #[serde(rename_all = "camelCase")]
    Changed { extension: String },
    #[serde(rename_all = "camelCase")]
    OperationStarted {
        operation: String,
        extension: String,
        review_id: Option<String>,
        title: String,
    },
    #[serde(rename_all = "camelCase")]
    OperationProgress {
        operation: String,
        extension: String,
        message: Option<String>,
        elapsed_ms: u64,
        findings: usize,
    },
    #[serde(rename_all = "camelCase")]
    OperationFinished {
        operation: String,
        extension: String,
        review_id: Option<String>,
        status: String,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    Notice {
        extension: String,
        level: String,
        message: String,
    },
}

// ── Configuration and views ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    pub app_version: String,
    pub paths: Paths,
    pub target: String,
    pub locale: String,
    pub inactivity: Duration,
    pub deadline: Duration,
    pub cancel_grace: Duration,
    pub handshake: Duration,
}

impl Config {
    pub fn new(app_version: &str, paths: Paths) -> Config {
        Config {
            app_version: app_version.into(),
            paths,
            target: version::current_target().into(),
            locale: "en".into(),
            inactivity: operations::INACTIVITY,
            deadline: operations::DEADLINE,
            cancel_grace: operations::CANCEL_GRACE,
            handshake: HANDSHAKE,
        }
    }
}

/// Where a contribution was invoked from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    #[serde(default)]
    pub kind: Context,
    /// The working directory of the repository involved.
    #[serde(default)]
    pub workdir: Option<PathBuf>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub pull_request: Option<PullRequestRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestRef {
    pub number: u64,
    #[serde(default)]
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Installed,
    Disabled,
    Starting,
    Active,
    Stopping,
    Failed,
    Incompatible,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityView {
    pub capability: Capability,
    pub required: bool,
    pub granted: bool,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingView {
    #[serde(flatten)]
    pub setting: crate::manifest::Setting,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub id: String,
    pub name: String,
    pub install_url: Option<String>,
    pub minimum_version: Option<String>,
    pub chosen: Option<PathBuf>,
    pub detected: Option<Detected>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRun {
    pub tool: String,
    pub profile: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub at: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub stderr: String,
    pub logs: Vec<String>,
    pub tool_runs: Vec<ToolRun>,
    pub error: Option<String>,
    pub program: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unavailable {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub provenance: Provenance,
    /// Shipped with Spagitty. Decided by where it came from, never by what
    /// its manifest says.
    pub official: bool,
    pub state: State,
    pub state_reason: Option<String>,
    pub compatibility: Compatibility,
    pub enabled: bool,
    pub consented: bool,
    /// What enabling sends where, one sentence per review provider that says.
    pub sends_code_to: Vec<String>,
    pub capabilities: Vec<CapabilityView>,
    pub manifest: Manifest,
    pub settings: Vec<SettingView>,
    pub tools: Vec<ToolView>,
    pub unavailable: Vec<Unavailable>,
    pub previous_version: Option<String>,
    pub diagnostics: Diagnostics,
    pub running: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub extensions: Vec<ExtensionView>,
    pub problems: Vec<Unreadable>,
    pub trust: &'static str,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub token: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub targets: Vec<String>,
    pub capabilities: Vec<CapabilityView>,
    pub files: Vec<package::PackageFile>,
    pub digest: String,
    pub warnings: Vec<String>,
    pub compatibility: Compatibility,
    /// The installed version this would replace.
    pub replaces: Option<String>,
    /// Capabilities this version asks for that the installed one did not.
    pub added_capabilities: Vec<Capability>,
    pub trust: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub operation: String,
    pub review_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Completed,
    Failed,
    Cancelled,
}

// ── Internal state ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    Idle,
    Starting,
    Active,
    Stopping,
    Failed,
}

#[derive(Default)]
struct Slot {
    run: Option<Run>,
    worker: Option<Arc<Worker>>,
    session: String,
    version: String,
    crashes: VecDeque<Instant>,
    error: Option<String>,
    unavailable: Vec<Unavailable>,
    tools: HashMap<String, Detected>,
    logs: VecDeque<String>,
    tool_runs: VecDeque<ToolRun>,
    last_stderr: String,
    program: Option<PathBuf>,
    /// Activated for this repository id in this session.
    activated: bool,
}

impl Slot {
    fn run(&self) -> Run {
        self.run.unwrap_or(Run::Idle)
    }

    fn log(&mut self, line: String) {
        self.logs.push_back(line);
        while self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }
}

#[derive(Debug, Clone)]
struct RepoHandle {
    extension: String,
    session: String,
    workdir: PathBuf,
    main_workdir: PathBuf,
    repository_id: String,
}

#[derive(Debug, Clone)]
struct WorkdirHandle {
    extension: String,
    session: String,
    operation: String,
    path: PathBuf,
    repository_id: String,
}

#[derive(Default)]
struct Handles {
    repos: HashMap<String, RepoHandle>,
    workdirs: HashMap<String, WorkdirHandle>,
    next: u64,
}

/// A repository, identified.
#[derive(Debug, Clone)]
struct Repository {
    workdir: PathBuf,
    main_workdir: PathBuf,
    id: String,
}

fn repository(workdir: &Path) -> Result<Repository> {
    let repo = spagitty_core::repo::open(workdir)?;
    let id = snapshot::repository_id(&repo);
    let workdir = spagitty_core::repo::workdir(&repo)?.to_path_buf();
    let main_workdir =
        spagitty_core::compare::main_workdir(&repo).unwrap_or_else(|| workdir.clone());
    Ok(Repository {
        workdir,
        main_workdir,
        id,
    })
}

struct Inner {
    config: Config,
    store: Store,
    events: Arc<dyn Events>,
    services: Arc<dyn Services>,
    entries: Mutex<(Vec<Entry>, Vec<Unreadable>)>,
    slots: Mutex<HashMap<String, Slot>>,
    starting: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    operations: Operations,
    handles: Mutex<Handles>,
    staged: Mutex<HashMap<String, package::Validated>>,
    finished: Mutex<HashMap<String, ReviewRecord>>,
    finished_signal: Condvar,
    sequence: AtomicU64,
    shutdown: AtomicBool,
}

/// The extension host. Cheap to clone; every clone is the same host.
#[derive(Clone)]
pub struct ExtensionHost {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for ExtensionHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtensionHost").finish()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn rpc(code: i64, message: impl Into<String>) -> RpcError {
    RpcError::new(code, message)
}

impl Inner {
    fn next(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}-{}",
            now_ms(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn emit(&self, event: HostEvent) {
        self.events.emit(event);
    }

    fn changed(&self, extension: &str) {
        self.emit(HostEvent::Changed {
            extension: extension.into(),
        });
    }

    fn refresh(&self) {
        self.import_development_requests();
        let found = registry::discover(&self.config.paths, &self.store);
        *self.entries.lock().expect("entries") = found;
    }

    /// Attach the development directories `ext dev` asked for.
    ///
    /// The tool cannot edit the user state while Spagitty holds it, so it
    /// leaves a request — `{"path": "…"}` — in `development-requests/`, and
    /// the host attaches it the next time it reads its list. Each request is
    /// still one directory a person named on the command line; nothing finds
    /// development directories by looking.
    fn import_development_requests(&self) {
        let dir = self.config.paths.data.join(registry::DEVELOPMENT_REQUESTS);
        let Ok(requests) = std::fs::read_dir(&dir) else {
            return;
        };
        let mut attached = Vec::new();
        for request in requests.flatten() {
            let path = request.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let wanted = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .and_then(|v| v.get("path").and_then(Value::as_str).map(PathBuf::from));
            let _ = std::fs::remove_file(&path);
            let Some(wanted) = wanted else { continue };
            let reserved: Vec<String> = registry::discover(&self.config.paths, &self.store)
                .0
                .iter()
                .filter(|e| e.provenance == Provenance::Bundled)
                .map(|e| e.manifest.id.clone())
                .collect();
            match registry::attach(&self.store, &wanted, &reserved) {
                Ok(manifest) => attached.push(manifest.id),
                Err(error) => self.emit(HostEvent::Notice {
                    extension: String::new(),
                    level: "error".into(),
                    message: format!("{} could not be attached: {error}", wanted.display()),
                }),
            }
        }
        for id in attached {
            self.changed(&id);
        }
    }

    fn entry(&self, id: &str) -> Result<Entry> {
        self.entries
            .lock()
            .expect("entries")
            .0
            .iter()
            .find(|e| e.manifest.id == id)
            .cloned()
            .ok_or_else(|| Error::NotInstalled(id.into()))
    }

    fn reserved(&self) -> Vec<String> {
        self.entries
            .lock()
            .expect("entries")
            .0
            .iter()
            .filter(|e| e.provenance == Provenance::Bundled)
            .map(|e| e.manifest.id.clone())
            .collect()
    }

    fn compatibility(&self, manifest: &Manifest) -> Compatibility {
        version::check(manifest, &self.config.app_version, &self.config.target)
    }

    fn granted(&self, extension: &str, version: &str, repository: &str) -> BTreeSet<Capability> {
        self.store
            .read(|s| s.granted(extension, version, repository))
    }

    /// The settings object a worker sees: user values, then repository values,
    /// then declared defaults.
    fn settings_for(
        &self,
        manifest: &Manifest,
        main_workdir: Option<&Path>,
    ) -> BTreeMap<String, Value> {
        let user = self
            .store
            .read(|s| s.settings.get(&manifest.id).cloned().unwrap_or_default());
        let repo = main_workdir
            .map(|dir| {
                storage::read_repository_settings(dir)
                    .remove(&manifest.id)
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        manifest
            .contributes
            .settings
            .iter()
            .filter(|s| s.kind != crate::manifest::SettingType::Executable)
            .map(|setting| {
                let held = match setting.scope {
                    SettingScope::User => user.get(&setting.key),
                    SettingScope::Repository => repo.get(&setting.key),
                };
                let value = held
                    .filter(|v| setting.accept(v).is_ok())
                    .cloned()
                    .unwrap_or_else(|| setting.default_value());
                (setting.key.clone(), value)
            })
            .collect()
    }

    fn mint_repo(&self, extension: &str, session: &str, repo: &Repository) -> String {
        let mut handles = self.handles.lock().expect("handles");
        // One handle per repository per session.
        if let Some((key, _)) = handles.repos.iter().find(|(_, h)| {
            h.extension == extension
                && h.session == session
                && h.repository_id == repo.id
                && h.workdir == repo.workdir
        }) {
            return key.clone();
        }
        handles.next += 1;
        let key = format!("repo:{}", handles.next);
        handles.repos.insert(
            key.clone(),
            RepoHandle {
                extension: extension.into(),
                session: session.into(),
                workdir: repo.workdir.clone(),
                main_workdir: repo.main_workdir.clone(),
                repository_id: repo.id.clone(),
            },
        );
        key
    }

    fn mint_workdir(
        &self,
        extension: &str,
        session: &str,
        operation: &str,
        path: &Path,
        repository_id: &str,
    ) -> String {
        let mut handles = self.handles.lock().expect("handles");
        handles.next += 1;
        let key = format!("wd:{}", handles.next);
        handles.workdirs.insert(
            key.clone(),
            WorkdirHandle {
                extension: extension.into(),
                session: session.into(),
                operation: operation.into(),
                path: path.to_path_buf(),
                repository_id: repository_id.into(),
            },
        );
        key
    }

    fn forget_session(&self, extension: &str, session: &str) {
        let mut handles = self.handles.lock().expect("handles");
        handles
            .repos
            .retain(|_, h| !(h.extension == extension && h.session == session));
        handles
            .workdirs
            .retain(|_, h| !(h.extension == extension && h.session == session));
    }

    fn forget_operation_handles(&self, operation: &str) {
        self.handles
            .lock()
            .expect("handles")
            .workdirs
            .retain(|_, h| h.operation != operation);
    }

    // ── Starting and stopping ──────────────────────────────────────────────

    fn start_lock(&self, id: &str) -> Arc<Mutex<()>> {
        self.starting
            .lock()
            .expect("start locks")
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// The running worker for `id`, started if it is not, with its session.
    fn ensure_active(
        self: &Arc<Self>,
        id: &str,
        repo: Option<&Repository>,
        reason: &str,
    ) -> Result<(Arc<Worker>, String)> {
        let entry = self.entry(id)?;
        let compatibility = self.compatibility(&entry.manifest);
        if !compatibility.compatible {
            return Err(Error::Incompatible(compatibility.reasons.join(" ")));
        }
        if let Some(repo) = repo {
            if !self.store.read(|s| s.is_enabled(id, &repo.id)) {
                return Err(Error::Refused(format!(
                    "{} is not enabled for this repository.",
                    entry.manifest.name
                )));
            }
        }

        let lock = self.start_lock(id);
        let _guard = lock.lock().expect("start lock");
        {
            let mut slots = self.slots.lock().expect("slots");
            let slot = slots.entry(id.to_string()).or_default();
            if slot.run() == Run::Active {
                if let Some(worker) = slot.worker.clone().filter(|w| w.is_alive()) {
                    if slot.version == entry.manifest.version {
                        return Ok((worker, slot.session.clone()));
                    }
                }
            }
            if slot.run() == Run::Failed {
                let recent = slot
                    .crashes
                    .iter()
                    .filter(|t| t.elapsed() < CRASH_WINDOW)
                    .count();
                if recent >= CRASH_LIMIT {
                    return Err(Error::Refused(format!(
                        "{} stopped unexpectedly {recent} times. Restart it from Settings › Extensions.",
                        entry.manifest.name
                    )));
                }
            }
        }

        let program = registry::program(&entry, &self.config.target, &self.config.paths)
            .ok_or_else(|| {
                Error::Incompatible(format!(
                    "{} has no program for this computer.",
                    entry.manifest.name
                ))
            })?;
        let session = self.next("s");
        {
            let mut slots = self.slots.lock().expect("slots");
            let slot = slots.entry(id.to_string()).or_default();
            slot.run = Some(Run::Starting);
            slot.session = session.clone();
            slot.version = entry.manifest.version.clone();
            slot.error = None;
            slot.unavailable.clear();
            slot.tools.clear();
            slot.program = Some(program.clone());
            slot.activated = false;
        }
        self.changed(id);

        let link = Arc::new(Link {
            inner: Arc::downgrade(self),
            extension: id.to_string(),
            session: session.clone(),
        });
        let env = vec![
            ("SPAGITTY_EXTENSION_ID".to_string(), id.to_string()),
            ("SPAGITTY_EXTENSION_API".to_string(), "1".to_string()),
        ];
        let spawned = Worker::spawn(&program, &entry.dir, &env, link);
        let worker = match spawned {
            Ok(worker) => worker,
            Err(error) => {
                let message = format!("It could not be started: {error}");
                self.fail_start(id, &session, &message);
                return Err(Error::Worker(message));
            }
        };
        self.slots
            .lock()
            .expect("slots")
            .entry(id.to_string())
            .or_default()
            .worker = Some(worker.clone());

        let granted: Vec<&'static str> = repo
            .map(|r| self.granted(id, &entry.manifest.version, &r.id))
            .unwrap_or_default()
            .into_iter()
            .map(Capability::as_str)
            .collect();
        let settings = self.settings_for(&entry.manifest, repo.map(|r| r.main_workdir.as_path()));
        let initialize = json!({
            "apiVersions": [crate::API_VERSION],
            "host": {"name": "Spagitty", "version": self.config.app_version},
            "extension": {"id": id, "version": entry.manifest.version},
            "session": session,
            "capabilities": granted,
            "locale": self.config.locale,
            "platform": self.config.target,
            "limits": {
                "maxMessageBytes": crate::protocol::MAX_MESSAGE_BYTES,
                "maxConcurrentOperations": operations::MAX_PER_EXTENSION,
                "inactivityMs": self.config.inactivity.as_millis() as u64,
                "cancelGraceMs": self.config.cancel_grace.as_millis() as u64,
            },
            "settings": settings,
        });
        let answer = worker.request("extension.initialize", initialize, self.config.handshake);
        let api = match answer {
            Ok(result) => result
                .get("apiVersion")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            Err(error) => {
                worker.terminate();
                let message = format!("It did not complete its handshake: {error}.");
                self.fail_start(id, &session, &message);
                return Err(Error::Worker(message));
            }
        };
        if !version::negotiated(&api) {
            worker.terminate();
            let message = format!(
                "It speaks extension API {api:?}; this Spagitty speaks {}.",
                crate::API_VERSION
            );
            self.fail_start(id, &session, &message);
            return Err(Error::Incompatible(message));
        }

        match worker.request(
            "extension.activate",
            json!({"reason": reason}),
            REQUEST_TIMEOUT,
        ) {
            Ok(result) => {
                let unavailable = result
                    .get("unavailable")
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(|u| {
                                Some(Unavailable {
                                    id: u.get("id")?.as_str()?.chars().take(64).collect(),
                                    reason: redact(
                                        &u.get("reason")?
                                            .as_str()?
                                            .chars()
                                            .take(300)
                                            .collect::<String>(),
                                    ),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut slots = self.slots.lock().expect("slots");
                let slot = slots.entry(id.to_string()).or_default();
                if slot.session == session {
                    slot.unavailable = unavailable;
                    slot.run = Some(Run::Active);
                    slot.activated = true;
                }
            }
            Err(error) => {
                worker.terminate();
                let message = format!("It could not be activated: {error}.");
                self.fail_start(id, &session, &message);
                return Err(Error::Worker(message));
            }
        }
        self.changed(id);
        Ok((worker, session))
    }

    fn fail_start(&self, id: &str, session: &str, message: &str) {
        {
            let mut slots = self.slots.lock().expect("slots");
            let slot = slots.entry(id.to_string()).or_default();
            if slot.session == session {
                slot.run = Some(Run::Failed);
                slot.error = Some(message.to_string());
                slot.crashes.push_back(Instant::now());
                if let Some(worker) = slot.worker.take() {
                    slot.last_stderr = worker.stderr_tail();
                }
            }
        }
        self.forget_session(id, session);
        self.changed(id);
    }

    /// Stop a worker cleanly: cancel its work, ask it to deactivate, then end
    /// its tree if it has not gone by itself.
    fn stop(&self, id: &str) {
        let (worker, session) = {
            let mut slots = self.slots.lock().expect("slots");
            let Some(slot) = slots.get_mut(id) else {
                return;
            };
            let Some(worker) = slot.worker.clone() else {
                slot.run = Some(Run::Idle);
                return;
            };
            slot.run = Some(Run::Stopping);
            (worker, slot.session.clone())
        };
        self.changed(id);
        for op in self.operations.finish_all(id) {
            op.cancel.store(true, Ordering::Release);
            self.conclude(
                op,
                Conclusion::Cancelled("The extension was stopped.".into()),
            );
        }
        let _ = worker.request("extension.deactivate", json!({}), self.config.cancel_grace);
        worker.close_input();
        if !worker.wait_exit(self.config.cancel_grace) {
            worker.terminate();
        }
        {
            let mut slots = self.slots.lock().expect("slots");
            if let Some(slot) = slots.get_mut(id) {
                if slot.session == session {
                    slot.last_stderr = worker.stderr_tail();
                    slot.worker = None;
                    slot.run = Some(Run::Idle);
                    slot.activated = false;
                }
            }
        }
        self.forget_session(id, &session);
        self.changed(id);
    }

    // ── Ending operations ──────────────────────────────────────────────────

    fn conclude(&self, op: Operation, conclusion: Conclusion) {
        // A tool timeout or completion can race the supervisor's next tick.
        // A late worker answer cannot turn an expired deadline into success.
        let conclusion = if matches!(&conclusion, Conclusion::Worker { .. })
            && !op.is_cancelling()
            && Instant::now() >= op.deadline
        {
            Conclusion::Failed("It ran past its time limit and was stopped.".into())
        } else {
            conclusion
        };
        op.cancel.store(true, Ordering::Release);
        self.forget_operation_handles(&op.id);
        match &op.kind {
            Kind::Command { .. } => {
                let (status, message) = match conclusion {
                    Conclusion::Worker {
                        status, message, ..
                    } => {
                        let status = if op.is_cancelling() {
                            OperationStatus::Cancelled
                        } else {
                            status
                        };
                        (status, message.unwrap_or_default())
                    }
                    Conclusion::Cancelled(m) => (OperationStatus::Cancelled, m),
                    Conclusion::Failed(m) => (OperationStatus::Failed, m),
                };
                self.emit(HostEvent::OperationFinished {
                    operation: op.id.clone(),
                    extension: op.extension.clone(),
                    review_id: None,
                    status: serde_json::to_value(status)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    message: redact(&message),
                });
            }
            Kind::Review { .. } => self.conclude_review(op, conclusion),
        }
    }

    fn conclude_review(&self, op: Operation, conclusion: Conclusion) {
        let Kind::Review {
            provider,
            review_id,
            snapshot: reviewed,
            workdir,
            main_workdir,
            requester,
        } = op.kind.clone()
        else {
            return;
        };
        let (mut said, completeness, provider_version, summary, action, message) = match conclusion
        {
            Conclusion::Worker {
                status,
                message,
                review,
            } => {
                let review = review.unwrap_or(Value::Null);
                let said = match status {
                    OperationStatus::Cancelled => RunStatus::Cancelled,
                    OperationStatus::Failed => review
                        .get("status")
                        .and_then(Value::as_str)
                        .and_then(RunStatus::parse)
                        .filter(|s| {
                            matches!(
                                s,
                                RunStatus::ActionRequired
                                    | RunStatus::Incomplete
                                    | RunStatus::Failed
                            )
                        })
                        .unwrap_or(RunStatus::Failed),
                    OperationStatus::Completed => review
                        .get("status")
                        .and_then(Value::as_str)
                        .and_then(RunStatus::parse)
                        .filter(|s| s.is_terminal() && *s != RunStatus::Stale)
                        .unwrap_or(RunStatus::Failed),
                };
                let completeness = review
                    .get("completeness")
                    .cloned()
                    .and_then(|c| serde_json::from_value(c).ok())
                    .unwrap_or(Completeness::Unknown);
                let version = review
                    .get("providerVersion")
                    .and_then(Value::as_str)
                    .map(|v| v.chars().take(64).collect())
                    .unwrap_or_else(|| "unknown".to_string());
                let summary = review
                    .get("summary")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let action = review
                    .get("actionRequired")
                    .cloned()
                    .and_then(|a| serde_json::from_value::<ActionRequired>(a).ok());
                (
                    said,
                    completeness,
                    version,
                    summary,
                    action,
                    message.unwrap_or_default(),
                )
            }
            Conclusion::Cancelled(m) => (
                RunStatus::Cancelled,
                Completeness::Unknown,
                "unknown".into(),
                None,
                None,
                m,
            ),
            Conclusion::Failed(m) => (
                RunStatus::Failed,
                Completeness::Unknown,
                "unknown".into(),
                None,
                None,
                m,
            ),
        };
        if op.is_cancelling() {
            said = RunStatus::Cancelled;
        }
        if said == RunStatus::ActionRequired && action.is_none() {
            said = RunStatus::Failed;
        }

        // Was the code still the code when the review ended?
        let configuration_files = self
            .entry(&op.extension)
            .ok()
            .and_then(|e| {
                e.manifest
                    .review_provider(&provider)
                    .map(|p| p.configuration_files.clone())
            })
            .unwrap_or_default();
        let request = snapshot::Request {
            target: reviewed.target,
            scope: reviewed.scope,
            base: Some(reviewed.base_commit.clone()),
            task_id: reviewed.task_id.clone(),
            pull_request_number: reviewed.pull_request_number,
        };
        let current = snapshot::take(&workdir, &request, &configuration_files)
            .ok()
            .map(|(s, _)| s);
        let stale = current.as_ref().map_or(true, |c| !c.same_code(&reviewed));
        let status = review::settle(said, completeness, stale && said != RunStatus::Failed);

        let mut findings = op.findings.clone();
        for finding in &mut findings {
            finding.review_id = review_id.clone();
            finding.provider_id = provider.clone();
        }
        let summary = summary.unwrap_or_else(|| message.clone());
        let result = ReviewResult {
            review_id: review_id.clone(),
            provider_id: provider.clone(),
            provider_version,
            snapshot_id: reviewed.id.clone(),
            status,
            summary: redact(&summary),
            findings,
            completeness,
            started_at: op.started_at.clone(),
            finished_at: Some(snapshot::now_iso()),
            action_required: action,
        };
        let gate = review::decide(&result, &reviewed, current.as_ref(), GatePolicy::default());
        let record = ReviewRecord {
            schema_version: history::SCHEMA_VERSION,
            extension: op.extension.clone(),
            extension_version: op.extension_version.clone(),
            provider,
            requester,
            snapshot: reviewed,
            result,
            gate: Some(gate),
            created_ms: now_ms(),
        };
        if let Err(error) = history::put(&main_workdir, record.clone()) {
            self.emit(HostEvent::Notice {
                extension: op.extension.clone(),
                level: "error".into(),
                message: format!("The review finished but could not be saved: {error}"),
            });
        }
        let status_word = serde_json::to_value(status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        {
            let mut finished = self.finished.lock().expect("finished reviews");
            finished.insert(review_id.clone(), record);
            if finished.len() > 200 {
                let oldest: Vec<String> = finished
                    .keys()
                    .take(finished.len() - 200)
                    .cloned()
                    .collect();
                for key in oldest {
                    finished.remove(&key);
                }
            }
        }
        self.finished_signal.notify_all();
        self.emit(HostEvent::OperationFinished {
            operation: op.id,
            extension: op.extension,
            review_id: Some(review_id),
            status: status_word,
            message: redact(&message),
        });
    }

    fn supervise(&self) {
        for (id, expiry) in self.operations.expired(
            Instant::now(),
            self.config.inactivity,
            self.config.cancel_grace,
        ) {
            let Some(op) = self.operations.finish(&id) else {
                continue;
            };
            let worker = self
                .slots
                .lock()
                .expect("slots")
                .get(&op.extension)
                .and_then(|s| s.worker.clone());
            if let Some(worker) = &worker {
                worker.notify("operation.cancel", json!({"operationId": id}));
            }
            let extension = op.extension.clone();
            match expiry {
                Expiry::Inactive => self.conclude(
                    op,
                    Conclusion::Failed(format!(
                        "It reported nothing for {} seconds and was stopped.",
                        self.config.inactivity.as_secs()
                    )),
                ),
                Expiry::Deadline => self.conclude(
                    op,
                    Conclusion::Failed("It ran past its time limit and was stopped.".into()),
                ),
                Expiry::CancelGrace => {
                    self.conclude(op, Conclusion::Cancelled("Cancelled.".into()));
                    // It did not stop when asked; its process tree is ended so
                    // nothing it started keeps running.
                    if let Some(worker) = worker {
                        if self.operations.count_for(&extension) == 0 {
                            let mut slots = self.slots.lock().expect("slots");
                            if let Some(slot) = slots.get_mut(&extension) {
                                slot.run = Some(Run::Stopping);
                            }
                        }
                        worker.terminate();
                    }
                }
            }
        }
    }
}

enum Conclusion {
    Worker {
        status: OperationStatus,
        message: Option<String>,
        review: Option<Value>,
    },
    Cancelled(String),
    Failed(String),
}

// ── The public API ─────────────────────────────────────────────────────────

impl ExtensionHost {
    pub fn new(
        config: Config,
        events: Arc<dyn Events>,
        services: Arc<dyn Services>,
    ) -> ExtensionHost {
        let _ = std::fs::create_dir_all(&config.paths.data);
        let store = Store::open(&config.paths.data);
        let inner = Arc::new(Inner {
            config,
            store,
            events,
            services,
            entries: Mutex::new((Vec::new(), Vec::new())),
            slots: Mutex::new(HashMap::new()),
            starting: Mutex::new(HashMap::new()),
            operations: Operations::default(),
            handles: Mutex::new(Handles::default()),
            staged: Mutex::new(HashMap::new()),
            finished: Mutex::new(HashMap::new()),
            finished_signal: Condvar::new(),
            sequence: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
        });
        inner.refresh();
        let weak = Arc::downgrade(&inner);
        std::thread::Builder::new()
            .name("extension-supervisor".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(200));
                let Some(inner) = weak.upgrade() else { return };
                if inner.shutdown.load(Ordering::Acquire) {
                    return;
                }
                inner.supervise();
            })
            .expect("starting the extension supervisor");
        ExtensionHost { inner }
    }

    pub fn paths(&self) -> &Paths {
        &self.inner.config.paths
    }

    /// Every extension, as the Settings section shows it for the repository at
    /// `workdir` (or for no repository).
    pub fn list(&self, workdir: Option<&Path>) -> Listing {
        self.inner.refresh();
        let repo = workdir.and_then(|w| repository(w).ok());
        let (entries, problems) = self.inner.entries.lock().expect("entries").clone();
        let extensions = entries
            .iter()
            .map(|entry| self.view(entry, repo.as_ref()))
            .collect();
        Listing {
            extensions,
            problems,
            trust: TRUST,
            target: self.inner.config.target.clone(),
        }
    }

    fn view(&self, entry: &Entry, repo: Option<&Repository>) -> ExtensionView {
        let inner = &self.inner;
        let manifest = &entry.manifest;
        let id = &manifest.id;
        let compatibility = inner.compatibility(manifest);
        let enabled = repo.is_some_and(|r| inner.store.read(|s| s.is_enabled(id, &r.id)));
        let consented = repo.is_some_and(|r| inner.store.read(|s| s.consented(id, &r.id)));
        let granted = repo
            .map(|r| inner.granted(id, &manifest.version, &r.id))
            .unwrap_or_default();
        let capabilities = manifest
            .capabilities
            .required
            .iter()
            .map(|c| (*c, true))
            .chain(manifest.capabilities.optional.iter().map(|c| (*c, false)))
            .map(|(capability, required)| CapabilityView {
                capability,
                required,
                granted: granted.contains(&capability),
                description: capability.describe(),
            })
            .collect();

        let settings_values = inner.settings_for(manifest, repo.map(|r| r.main_workdir.as_path()));
        let chosen = inner
            .store
            .read(|s| s.executables.get(id).cloned().unwrap_or_default());
        let settings = manifest
            .contributes
            .settings
            .iter()
            .map(|setting| SettingView {
                setting: setting.clone(),
                value: match setting.kind {
                    crate::manifest::SettingType::Executable => setting
                        .tool
                        .as_ref()
                        .and_then(|t| chosen.get(t))
                        .map(|p| Value::String(p.display().to_string()))
                        .unwrap_or(Value::Null),
                    _ => settings_values
                        .get(&setting.key)
                        .cloned()
                        .unwrap_or(Value::Null),
                },
            })
            .collect();

        let slots = inner.slots.lock().expect("slots");
        let slot = slots.get(id);
        let tools = manifest
            .external_tools
            .iter()
            .map(|tool| ToolView {
                id: tool.id.clone(),
                name: tool.label().to_string(),
                install_url: tool.install_url.clone(),
                minimum_version: tool.minimum_version.clone(),
                chosen: chosen.get(&tool.id).cloned(),
                detected: slot.and_then(|s| s.tools.get(&tool.id).cloned()),
            })
            .collect();

        let run = slot.map(Slot::run).unwrap_or(Run::Idle);
        let (state, state_reason) = if !compatibility.compatible {
            (State::Incompatible, Some(compatibility.reasons.join(" ")))
        } else if repo.is_none() {
            (State::Disabled, Some("Open a repository to use it.".into()))
        } else if !enabled {
            (State::Disabled, None)
        } else {
            match run {
                Run::Idle => (State::Installed, None),
                Run::Starting => (State::Starting, None),
                Run::Active => (State::Active, None),
                Run::Stopping => (State::Stopping, None),
                Run::Failed => (State::Failed, slot.and_then(|s| s.error.clone())),
            }
        };
        let diagnostics = Diagnostics {
            stderr: slot
                .map(|s| {
                    s.worker
                        .as_ref()
                        .map(|w| w.stderr_tail())
                        .unwrap_or_else(|| s.last_stderr.clone())
                })
                .unwrap_or_default(),
            logs: slot
                .map(|s| s.logs.iter().cloned().collect())
                .unwrap_or_default(),
            tool_runs: slot
                .map(|s| s.tool_runs.iter().cloned().collect())
                .unwrap_or_default(),
            error: slot.and_then(|s| s.error.clone()),
            program: slot.and_then(|s| s.program.clone()),
        };
        let unavailable = slot.map(|s| s.unavailable.clone()).unwrap_or_default();
        drop(slots);

        ExtensionView {
            id: id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            publisher: manifest.publisher.clone(),
            description: manifest.description.clone(),
            license: manifest.license.clone(),
            homepage: manifest.homepage.clone(),
            provenance: entry.provenance,
            official: entry.provenance == Provenance::Bundled,
            state,
            state_reason,
            compatibility,
            enabled,
            consented,
            sends_code_to: manifest
                .contributes
                .review_providers
                .iter()
                .filter_map(|p| p.sends_code_to.clone())
                .collect(),
            capabilities,
            manifest: manifest.clone(),
            settings,
            tools,
            unavailable,
            previous_version: if entry.provenance == Provenance::Local {
                inner
                    .store
                    .read(|s| s.installed.get(id).and_then(|i| i.previous.clone()))
            } else {
                None
            },
            diagnostics,
            running: inner.operations.count_for(id),
        }
    }

    // ── Packages ─────────────────────────────────────────────────────────

    /// Validate a package file and describe it. Nothing is installed or run.
    pub fn inspect_package(&self, path: &Path, workdir: Option<&Path>) -> Result<InstallPreview> {
        let meta = std::fs::metadata(path)
            .map_err(|e| Error::Package(format!("{}: {e}", path.display())))?;
        if meta.len() > crate::zip::MAX_ARCHIVE {
            return Err(Error::Package("the file is larger than 300 MiB".into()));
        }
        let bytes = std::fs::read(path)?;
        let validated = package::validate(bytes)?;
        let manifest = &validated.manifest;
        let inner = &self.inner;
        inner.refresh();
        if inner.reserved().contains(&manifest.id) {
            return Err(Error::Conflict(format!(
                "{} is part of Spagitty and is updated with Spagitty; a file cannot replace it.",
                manifest.id
            )));
        }
        let installed = inner.entry(&manifest.id).ok();
        if let Some(existing) = &installed {
            if existing.provenance == Provenance::Development {
                return Err(Error::Conflict(format!(
                    "{} is attached for development; detach it first.",
                    manifest.id
                )));
            }
        }
        let repo = workdir.and_then(|w| repository(w).ok());
        let granted = match (&installed, &repo) {
            (Some(existing), Some(r)) => {
                inner.granted(&manifest.id, &existing.manifest.version, &r.id)
            }
            _ => BTreeSet::new(),
        };
        let before: BTreeSet<Capability> = installed
            .as_ref()
            .map(|e| e.manifest.capabilities.requested().collect())
            .unwrap_or_default();
        let added = manifest
            .capabilities
            .requested()
            .filter(|c| !before.contains(c))
            .collect::<Vec<_>>();
        let capabilities = manifest
            .capabilities
            .required
            .iter()
            .map(|c| (*c, true))
            .chain(manifest.capabilities.optional.iter().map(|c| (*c, false)))
            .map(|(capability, required)| CapabilityView {
                capability,
                required,
                granted: granted.contains(&capability) && !added.contains(&capability),
                description: capability.describe(),
            })
            .collect();
        let token = inner.next("pkg");
        let preview = InstallPreview {
            token: token.clone(),
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            publisher: manifest.publisher.clone(),
            description: manifest.description.clone(),
            license: manifest.license.clone(),
            targets: manifest.runtime.entrypoints.keys().cloned().collect(),
            capabilities,
            files: validated.files.clone(),
            digest: validated.digest.clone(),
            warnings: validated.warnings.clone(),
            compatibility: inner.compatibility(manifest),
            replaces: installed.map(|e| e.manifest.version),
            added_capabilities: if before.is_empty() { Vec::new() } else { added },
            trust: TRUST,
        };
        let mut staged = inner.staged.lock().expect("staged");
        staged.clear();
        staged.insert(token, validated);
        Ok(preview)
    }

    /// Install (or update to) the package an earlier inspection described.
    pub fn install(&self, token: &str) -> Result<registry::Installed> {
        let inner = &self.inner;
        let validated = inner
            .staged
            .lock()
            .expect("staged")
            .remove(token)
            .ok_or_else(|| {
                Error::Refused("Choose the package again; that preview has expired.".into())
            })?;
        let compatibility = inner.compatibility(&validated.manifest);
        if !compatibility.compatible {
            return Err(Error::Incompatible(compatibility.reasons.join(" ")));
        }
        let id = validated.manifest.id.clone();
        // An update waits for the old version's work to end.
        if inner.operations.count_for(&id) > 0 {
            return Err(Error::Refused(format!(
                "{} is working. Wait for it to finish, or cancel it, before updating.",
                validated.manifest.name
            )));
        }
        inner.stop(&id);
        let installed = registry::install(
            &inner.config.paths,
            &inner.store,
            &validated,
            &inner.reserved(),
        )?;
        inner.refresh();
        inner.changed(&id);
        Ok(installed)
    }

    pub fn rollback(&self, id: &str) -> Result<registry::Installed> {
        let inner = &self.inner;
        if inner.operations.count_for(id) > 0 {
            return Err(Error::Refused(
                "Wait for its work to finish, or cancel it, first.".into(),
            ));
        }
        inner.stop(id);
        let done = registry::rollback(&inner.config.paths, &inner.store, id)?;
        inner.refresh();
        inner.changed(id);
        Ok(done)
    }

    /// Remove an installed extension. Its review history in `workdir`'s
    /// repository is deleted too unless `keep_history`. External tools it
    /// used are left alone: they were installed separately.
    pub fn uninstall(&self, id: &str, keep_history: bool, workdir: Option<&Path>) -> Result<()> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        if entry.provenance == Provenance::Bundled {
            return Err(Error::Refused(format!(
                "{} ships with Spagitty. Disable it instead.",
                entry.manifest.name
            )));
        }
        inner.stop(id);
        match entry.provenance {
            Provenance::Local => registry::uninstall(&inner.config.paths, &inner.store, id)?,
            Provenance::Development => registry::detach(&inner.store, id)?,
            Provenance::Bundled => unreachable!(),
        }
        if !keep_history {
            if let Some(repo) = workdir.and_then(|w| repository(w).ok()) {
                history::delete(&repo.main_workdir, id, None)?;
            }
        }
        inner.slots.lock().expect("slots").remove(id);
        inner.refresh();
        inner.changed(id);
        Ok(())
    }

    pub fn attach_development(&self, dir: &Path) -> Result<Manifest> {
        let inner = &self.inner;
        inner.refresh();
        let manifest = registry::attach(&inner.store, dir, &inner.reserved())?;
        inner.refresh();
        inner.changed(&manifest.id);
        Ok(manifest)
    }

    pub fn restart(&self, id: &str) -> Result<()> {
        let inner = &self.inner;
        inner.entry(id)?;
        inner.stop(id);
        {
            let mut slots = inner.slots.lock().expect("slots");
            let slot = slots.entry(id.to_string()).or_default();
            slot.crashes.clear();
            slot.error = None;
            slot.run = Some(Run::Idle);
        }
        inner.changed(id);
        Ok(())
    }

    // ── Enablement, grants, consent, settings ─────────────────────────────

    /// Enable `id` for the repository at `workdir`, granting the required
    /// capabilities and the optional ones in `optional`. `consent` is the
    /// statement the person agreed to when the extension sends code to a
    /// service. Nothing is started and nothing is reviewed.
    pub fn enable(
        &self,
        id: &str,
        workdir: &Path,
        optional: &[Capability],
        consent: Option<&str>,
    ) -> Result<()> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        let manifest = &entry.manifest;
        let compatibility = inner.compatibility(manifest);
        if !compatibility.compatible {
            return Err(Error::Incompatible(compatibility.reasons.join(" ")));
        }
        for capability in optional {
            if !manifest.capabilities.optional.contains(capability) {
                return Err(Error::Refused(format!(
                    "{} does not ask for {capability}.",
                    manifest.name
                )));
            }
        }
        let sends = manifest
            .contributes
            .review_providers
            .iter()
            .any(|p| p.sends_code_to.is_some());
        let repo = repository(workdir)?;
        let already = inner.store.read(|s| s.consented(id, &repo.id));
        if sends && !already && consent.map_or(true, |c| c.trim().is_empty()) {
            return Err(Error::Refused(format!(
                "{} sends code to a service. Agree to that before enabling it here.",
                manifest.name
            )));
        }
        inner.store.update(|s| {
            s.enabled
                .entry(id.to_string())
                .or_default()
                .insert(repo.id.clone());
            for capability in manifest.capabilities.required.iter().chain(optional) {
                s.grant(id, &manifest.version, &repo.id, *capability);
            }
            if let Some(statement) = consent.filter(|c| !c.trim().is_empty()) {
                s.consent.entry(id.to_string()).or_default().insert(
                    repo.id.clone(),
                    Consent {
                        statement: statement.chars().take(1000).collect(),
                        at_ms: now_ms(),
                    },
                );
            }
        })?;
        inner.changed(id);
        Ok(())
    }

    /// Disable `id` for the repository at `workdir`. Its work there is
    /// cancelled and its worker stopped; grants and consent are kept for when
    /// it is enabled again, unless `forget` is set.
    pub fn disable(&self, id: &str, workdir: &Path, forget: bool) -> Result<()> {
        let inner = &self.inner;
        inner.entry(id)?;
        let repo = repository(workdir)?;
        inner.stop(id);
        inner.store.update(|s| {
            if let Some(repos) = s.enabled.get_mut(id) {
                repos.remove(&repo.id);
            }
            if forget {
                s.grants
                    .retain(|g| !(g.extension == id && g.repository == repo.id));
                if let Some(c) = s.consent.get_mut(id) {
                    c.remove(&repo.id);
                }
            }
        })?;
        inner.changed(id);
        Ok(())
    }

    /// Grant or revoke an optional capability. A required one cannot be
    /// revoked on its own — disabling is how.
    pub fn set_grant(
        &self,
        id: &str,
        workdir: &Path,
        capability: Capability,
        granted: bool,
    ) -> Result<()> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        if !entry.manifest.capabilities.optional.contains(&capability) {
            return Err(Error::Refused(format!(
                "{capability} is required by {}; disable it instead.",
                entry.manifest.name
            )));
        }
        let repo = repository(workdir)?;
        inner.store.update(|s| {
            if granted {
                s.grant(id, &entry.manifest.version, &repo.id, capability);
            } else {
                s.revoke(id, &repo.id, capability);
            }
        })?;
        inner.changed(id);
        Ok(())
    }

    pub fn set_setting(
        &self,
        id: &str,
        key: &str,
        value: Value,
        workdir: Option<&Path>,
    ) -> Result<()> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        let setting = entry.manifest.setting(key).ok_or_else(|| {
            Error::Refused(format!(
                "{} has no setting called {key}.",
                entry.manifest.name
            ))
        })?;
        let value = setting.accept(&value).map_err(|why| {
            Error::Refused(format!(
                "{}: {why}.",
                setting.title.as_deref().unwrap_or(key)
            ))
        })?;
        let repo = workdir.and_then(|w| repository(w).ok());
        match setting.scope {
            SettingScope::User => {
                inner.store.update(|s| {
                    s.settings
                        .entry(id.to_string())
                        .or_default()
                        .insert(key.to_string(), value.clone());
                })?;
            }
            SettingScope::Repository => {
                let repo = repo.as_ref().ok_or_else(|| {
                    Error::Refused("Open the repository this setting is for.".into())
                })?;
                storage::write_repository_setting(&repo.main_workdir, id, key, value.clone())?;
            }
        }
        let worker = inner
            .slots
            .lock()
            .expect("slots")
            .get(id)
            .and_then(|s| s.worker.clone());
        if let Some(worker) = worker {
            let settings = inner.settings_for(
                &entry.manifest,
                repo.as_ref().map(|r| r.main_workdir.as_path()),
            );
            worker.notify("settings.changed", json!({"settings": settings}));
        }
        inner.changed(id);
        Ok(())
    }

    /// Use `path` for one of the extension's tools. `None` goes back to
    /// looking on `PATH`.
    pub fn choose_executable(&self, id: &str, tool: &str, path: Option<&Path>) -> Result<Detected> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        let declared = entry
            .manifest
            .tool(tool)
            .ok_or_else(|| {
                Error::Refused(format!(
                    "{} declares no tool called {tool}.",
                    entry.manifest.name
                ))
            })?
            .clone();
        if let Some(path) = path {
            if !tools::runnable(path) {
                return Err(Error::Refused(format!(
                    "{} is not a program this computer can run.",
                    path.display()
                )));
            }
        }
        inner.store.update(|s| match path {
            Some(path) => {
                s.executables
                    .entry(id.to_string())
                    .or_default()
                    .insert(tool.to_string(), path.to_path_buf());
            }
            None => {
                if let Some(tools) = s.executables.get_mut(id) {
                    tools.remove(tool);
                }
            }
        })?;
        let detected = tools::detect(&declared, path);
        inner
            .slots
            .lock()
            .expect("slots")
            .entry(id.to_string())
            .or_default()
            .tools
            .insert(tool.into(), detected.clone());
        inner.changed(id);
        Ok(detected)
    }

    /// Look for one of the extension's tools and ask its version. Runs the
    /// tool's version check, which never contacts a service.
    pub fn detect_tool(&self, id: &str, tool: &str) -> Result<Detected> {
        let inner = &self.inner;
        let entry = inner.entry(id)?;
        let declared = entry.manifest.tool(tool).ok_or_else(|| {
            Error::Refused(format!(
                "{} declares no tool called {tool}.",
                entry.manifest.name
            ))
        })?;
        let chosen = inner
            .store
            .read(|s| s.executables.get(id).and_then(|t| t.get(tool)).cloned());
        let detected = tools::detect(declared, chosen.as_deref());
        inner
            .slots
            .lock()
            .expect("slots")
            .entry(id.to_string())
            .or_default()
            .tools
            .insert(tool.into(), detected.clone());
        inner.changed(id);
        Ok(detected)
    }

    // ── Running things ───────────────────────────────────────────────────

    fn check_usable(&self, id: &str, repo: Option<&Repository>) -> Result<Entry> {
        let entry = self.inner.entry(id)?;
        let compatibility = self.inner.compatibility(&entry.manifest);
        if !compatibility.compatible {
            return Err(Error::Incompatible(compatibility.reasons.join(" ")));
        }
        match repo {
            Some(repo) if self.inner.store.read(|s| s.is_enabled(id, &repo.id)) => Ok(entry),
            Some(_) => Err(Error::Refused(format!(
                "{} is not enabled for this repository.",
                entry.manifest.name
            ))),
            None => Err(Error::Refused("Open a repository first.".into())),
        }
    }

    fn context_json(
        &self,
        id: &str,
        session: &str,
        invocation: &Invocation,
        repo: Option<&Repository>,
    ) -> Value {
        let mut context = json!({"kind": invocation.kind});
        if let Some(repo) = repo {
            context["repository"] = json!(self.inner.mint_repo(id, session, repo));
        }
        if let Some(task) = &invocation.task_id {
            context["taskId"] = json!(task);
        }
        if let Some(pr) = &invocation.pull_request {
            context["pullRequest"] = json!({"number": pr.number, "headSha": pr.head_sha});
        }
        context
    }

    /// Run one of an extension's commands.
    pub fn run_command(&self, id: &str, command: &str, invocation: &Invocation) -> Result<Started> {
        let repo = invocation.workdir.as_deref().map(repository).transpose()?;
        let entry = self.check_usable(id, repo.as_ref())?;
        let declared = entry.manifest.command(command).ok_or_else(|| {
            Error::Refused(format!(
                "{} has no command called {command}.",
                entry.manifest.name
            ))
        })?;
        if declared.review_provider.is_some() {
            return Err(Error::Refused(
                "That command starts a review; start it as a review.".into(),
            ));
        }
        if self.inner.operations.count_for(id) >= operations::MAX_PER_EXTENSION {
            return Err(Error::Refused(format!(
                "{} is already doing as much as it may at once.",
                entry.manifest.name
            )));
        }
        let (worker, session) = self.inner.ensure_active(id, repo.as_ref(), "command")?;
        let operation = self.inner.next("op");
        let now = Instant::now();
        self.inner.operations.insert(Operation {
            id: operation.clone(),
            extension: id.into(),
            extension_version: entry.manifest.version.clone(),
            session: session.clone(),
            kind: Kind::Command {
                command: command.into(),
            },
            repository: None,
            started: now,
            started_at: snapshot::now_iso(),
            last_activity: now,
            deadline: now + self.inner.config.deadline,
            cancel_requested: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: None,
            findings: Vec::new(),
            dropped_findings: 0,
            host_work: 0,
        });
        self.inner.emit(HostEvent::OperationStarted {
            operation: operation.clone(),
            extension: id.into(),
            review_id: None,
            title: declared.title.clone(),
        });
        let context = self.context_json(id, &session, invocation, repo.as_ref());
        let sent = worker.request(
            "command.execute",
            json!({"operationId": operation, "command": command, "context": context}),
            REQUEST_TIMEOUT,
        );
        if let Err(error) = sent {
            if let Some(op) = self.inner.operations.finish(&operation) {
                self.inner.conclude(
                    op,
                    Conclusion::Failed(format!("It did not accept the command: {error}.")),
                );
            }
            return Err(Error::Worker(format!(
                "It did not accept the command: {error}."
            )));
        }
        Ok(Started {
            operation,
            review_id: None,
        })
    }

    /// What a review would cover, for the person to look at before anything
    /// is sent anywhere.
    pub fn preview_review(
        &self,
        id: &str,
        provider: &str,
        request: &snapshot::Request,
        workdir: &Path,
    ) -> Result<Preview> {
        let repo = repository(workdir)?;
        let entry = self.check_usable(id, Some(&repo))?;
        let declared = self.provider(&entry, provider, request)?;
        let (_, preview) = snapshot::take(workdir, request, &declared.configuration_files)?;
        Ok(preview)
    }

    fn provider(
        &self,
        entry: &Entry,
        provider: &str,
        request: &snapshot::Request,
    ) -> Result<crate::manifest::ReviewProvider> {
        let declared = entry.manifest.review_provider(provider).ok_or_else(|| {
            Error::Refused(format!(
                "{} has no review provider called {provider}.",
                entry.manifest.name
            ))
        })?;
        if !declared.targets.contains(&request.target) {
            return Err(Error::Refused(format!(
                "{} cannot review this kind of change.",
                entry.manifest.name
            )));
        }
        if !declared.scopes.is_empty() && !declared.scopes.contains(&request.scope) {
            return Err(Error::Refused(format!(
                "{} cannot review that scope.",
                entry.manifest.name
            )));
        }
        Ok(declared.clone())
    }

    /// Start a review of `workdir` as `request` describes. Returns at once;
    /// the result arrives as an `OperationFinished` event and in the history.
    pub fn start_review(
        &self,
        id: &str,
        provider: &str,
        request: &snapshot::Request,
        workdir: &Path,
        requester: Requester,
    ) -> Result<Started> {
        let inner = &self.inner;
        let repo = repository(workdir)?;
        let entry = self.check_usable(id, Some(&repo))?;
        let declared = self.provider(&entry, provider, request)?;
        if !inner
            .granted(id, &entry.manifest.version, &repo.id)
            .contains(&Capability::ReviewProvide)
        {
            return Err(Error::Refused(format!(
                "{} has not been allowed to provide reviews here.",
                entry.manifest.name
            )));
        }
        if declared.sends_code_to.is_some() && !inner.store.read(|s| s.consented(id, &repo.id)) {
            return Err(Error::Refused(format!(
                "{} sends code to a service, and nobody has agreed to that for this repository.",
                entry.manifest.name
            )));
        }
        if inner.operations.count_for(id) >= operations::MAX_PER_EXTENSION {
            return Err(Error::Refused(format!(
                "{} is already doing as much as it may at once.",
                entry.manifest.name
            )));
        }
        let (snapshot, _) = snapshot::take(workdir, request, &declared.configuration_files)?;
        let (worker, session) = inner.ensure_active(id, Some(&repo), "reviewProvider")?;

        let operation = inner.next("op");
        let review_id = inner.next("rv");
        let workdir_handle = inner.mint_workdir(id, &session, &operation, &repo.workdir, &repo.id);
        let repository_handle = inner.mint_repo(id, &session, &repo);
        let now = Instant::now();
        inner.operations.insert(Operation {
            id: operation.clone(),
            extension: id.into(),
            extension_version: entry.manifest.version.clone(),
            session: session.clone(),
            kind: Kind::Review {
                provider: provider.into(),
                review_id: review_id.clone(),
                snapshot: snapshot.clone(),
                workdir: repo.workdir.clone(),
                main_workdir: repo.main_workdir.clone(),
                requester,
            },
            repository: Some(repository_handle.clone()),
            started: now,
            started_at: snapshot::now_iso(),
            last_activity: now,
            deadline: now + inner.config.deadline,
            cancel_requested: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: None,
            findings: Vec::new(),
            dropped_findings: 0,
            host_work: 0,
        });
        inner.emit(HostEvent::OperationStarted {
            operation: operation.clone(),
            extension: id.into(),
            review_id: Some(review_id.clone()),
            title: declared
                .title
                .clone()
                .unwrap_or_else(|| format!("{} review", entry.manifest.name)),
        });
        let sent = worker.request(
            "review.start",
            json!({
                "operationId": operation,
                "provider": provider,
                "reviewId": review_id,
                "repository": repository_handle,
                "snapshot": snapshot,
                "workdir": workdir_handle,
            }),
            REQUEST_TIMEOUT,
        );
        if let Err(error) = sent {
            if let Some(op) = inner.operations.finish(&operation) {
                inner.conclude(
                    op,
                    Conclusion::Failed(format!("It did not accept the review: {error}.")),
                );
            }
            return Err(Error::Worker(format!(
                "It did not accept the review: {error}."
            )));
        }
        Ok(Started {
            operation,
            review_id: Some(review_id),
        })
    }

    /// Ask an operation to stop. Its tool runs end now; the worker has the
    /// cancellation grace to say it has stopped before its tree is ended.
    pub fn cancel(&self, operation: &str) -> Result<()> {
        let inner = &self.inner;
        let extension = inner
            .operations
            .with_any(operation, |op| {
                if op.cancel_requested.is_none() {
                    op.cancel_requested = Some(Instant::now());
                }
                op.cancel.store(true, Ordering::Release);
                op.extension.clone()
            })
            .ok_or_else(|| Error::Refused("That has already finished.".into()))?;
        let worker = inner
            .slots
            .lock()
            .expect("slots")
            .get(&extension)
            .and_then(|s| s.worker.clone());
        if let Some(worker) = worker {
            worker.notify("operation.cancel", json!({"operationId": operation}));
        }
        inner.emit(HostEvent::OperationProgress {
            operation: operation.into(),
            extension,
            message: Some("Cancelling…".into()),
            elapsed_ms: 0,
            findings: 0,
        });
        Ok(())
    }

    /// Wait for a review to finish, up to `timeout`, or until `stop` is set.
    pub fn await_review(
        &self,
        review_id: &str,
        timeout: Duration,
        stop: &AtomicBool,
    ) -> Option<ReviewRecord> {
        let inner = &self.inner;
        let deadline = Instant::now() + timeout;
        let mut finished = inner.finished.lock().expect("finished reviews");
        loop {
            if let Some(record) = finished.get(review_id) {
                return Some(record.clone());
            }
            if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
                return None;
            }
            finished = inner
                .finished_signal
                .wait_timeout(finished, Duration::from_millis(250))
                .expect("finished reviews")
                .0;
        }
    }

    /// Check local provider availability before accepting persisted gate evidence.
    pub fn check_provider(&self, id: &str, provider: &str, workdir: &Path) -> Result<Value> {
        let repo = repository(workdir)?;
        let entry = self.check_usable(id, Some(&repo))?;
        let declared = entry
            .manifest
            .contributes
            .review_providers
            .iter()
            .find(|p| p.id == provider)
            .ok_or_else(|| Error::Refused("That provider is not declared.".into()))?;
        let granted = self.inner.granted(id, &entry.manifest.version, &repo.id);
        if !granted.contains(&Capability::ReviewProvide)
            || (declared.sends_code_to.is_some()
                && !self.inner.store.read(|s| s.consented(id, &repo.id)))
        {
            return Err(Error::Refused(
                "The review grant or consent was revoked.".into(),
            ));
        }
        if self.inner.operations.count_for(id) >= operations::MAX_PER_EXTENSION {
            return Err(Error::Refused("The review provider is busy.".into()));
        }
        let (worker, session) = self
            .inner
            .ensure_active(id, Some(&repo), "reviewProvider")?;
        let operation = self.inner.next("op");
        let now = Instant::now();
        self.inner.operations.insert(Operation {
            id: operation.clone(),
            extension: id.into(),
            extension_version: entry.manifest.version.clone(),
            session: session.clone(),
            kind: Kind::Command {
                command: "review.check".into(),
            },
            repository: None,
            started: now,
            started_at: snapshot::now_iso(),
            last_activity: now,
            deadline: now + REQUEST_TIMEOUT,
            cancel_requested: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: None,
            findings: Vec::new(),
            dropped_findings: 0,
            host_work: 0,
        });
        let repository = self.inner.mint_repo(id, &session, &repo);
        let response = worker.request(
            "review.check",
            json!({"operationId":operation,"provider":provider,"repository":repository}),
            REQUEST_TIMEOUT,
        );
        let op = self
            .inner
            .operations
            .finish(&operation)
            .ok_or_else(|| Error::Worker("Provider check timed out.".into()))?;
        op.cancel.store(true, Ordering::Release);
        if Instant::now() >= op.deadline || op.is_cancelling() {
            return Err(Error::Worker(
                "Provider check timed out or was cancelled.".into(),
            ));
        }
        response.map_err(|e| Error::Worker(format!("Could not check the review provider: {e}")))
    }

    /// The review history of `id` in the repository at `workdir`, newest first.
    pub fn reviews(&self, id: &str, workdir: &Path) -> Result<Vec<ReviewRecord>> {
        let repo = repository(workdir)?;
        let mut records = history::load(&repo.main_workdir, id);
        records.reverse();
        Ok(records)
    }

    pub fn set_disposition(
        &self,
        id: &str,
        workdir: &Path,
        review: &str,
        finding: &str,
        disposition: review::Disposition,
    ) -> Result<ReviewRecord> {
        let repo = repository(workdir)?;
        let record =
            history::set_disposition(&repo.main_workdir, id, review, finding, disposition)?;
        self.inner.changed(id);
        Ok(record)
    }

    pub fn delete_reviews(&self, id: &str, workdir: &Path, review: Option<&str>) -> Result<()> {
        let repo = repository(workdir)?;
        history::delete(&repo.main_workdir, id, review)?;
        self.inner.changed(id);
        Ok(())
    }

    /// The data for one of an extension's panels.
    pub fn resolve_panel(&self, id: &str, panel: &str, invocation: &Invocation) -> Result<Value> {
        let repo = invocation.workdir.as_deref().map(repository).transpose()?;
        let entry = self.check_usable(id, repo.as_ref())?;
        let declared = entry.manifest.panel(panel).ok_or_else(|| {
            Error::Refused(format!(
                "{} has no panel called {panel}.",
                entry.manifest.name
            ))
        })?;
        if declared.renderer == crate::manifest::Renderer::ReviewFindings {
            return Err(Error::Refused(
                "A findings panel is drawn from the review history.".into(),
            ));
        }
        let (worker, session) = self.inner.ensure_active(id, repo.as_ref(), "panel")?;
        let context = self.context_json(id, &session, invocation, repo.as_ref());
        let data = worker
            .request(
                "panel.resolve",
                json!({"panel": panel, "context": context}),
                PANEL_TIMEOUT,
            )
            .map_err(|error| Error::Worker(format!("The panel could not be drawn: {error}.")))?;
        let size = serde_json::to_vec(&data)
            .map(|b| b.len())
            .unwrap_or(usize::MAX);
        if size > 512 * 1024 {
            return Err(Error::Worker(
                "The panel's data was larger than 512 KiB.".into(),
            ));
        }
        Ok(data)
    }

    /// Stop every worker. Called when the application closes.
    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
        let ids: Vec<String> = self
            .inner
            .slots
            .lock()
            .expect("slots")
            .keys()
            .cloned()
            .collect();
        for id in ids {
            self.inner.stop(&id);
        }
    }

    /// The running worker's process id, for tests that need to watch it.
    pub fn worker_pid(&self, id: &str) -> Option<u32> {
        self.inner
            .slots
            .lock()
            .expect("slots")
            .get(id)
            .and_then(|s| s.worker.as_ref().map(|w| w.pid()))
    }

    /// Whether the user state says `id` is enabled for `workdir`, and with
    /// which capabilities. For the farm's composition (FEAT-098).
    pub fn enabled_for(&self, id: &str, workdir: &Path) -> Option<BTreeSet<Capability>> {
        let repo = repository(workdir).ok()?;
        let entry = self.inner.entry(id).ok()?;
        self.inner
            .store
            .read(|s| s.is_enabled(id, &repo.id))
            .then(|| self.inner.granted(id, &entry.manifest.version, &repo.id))
    }

    /// The manifest of an extension, if it is known.
    pub fn manifest(&self, id: &str) -> Option<Manifest> {
        self.inner.entry(id).ok().map(|e| e.manifest)
    }

    /// The current value of one setting, for the repository at `workdir`.
    pub fn setting(&self, id: &str, key: &str, workdir: Option<&Path>) -> Option<Value> {
        let entry = self.inner.entry(id).ok()?;
        let main = workdir
            .and_then(|w| repository(w).ok())
            .map(|r| r.main_workdir);
        self.inner
            .settings_for(&entry.manifest, main.as_deref())
            .remove(key)
    }
}

// ── The link a worker talks to ─────────────────────────────────────────────

struct Link {
    inner: Weak<Inner>,
    extension: String,
    session: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteParams {
    operation_id: String,
    status: OperationStatus,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    review: Option<Value>,
}

fn param<'a>(params: &'a Value, key: &str) -> std::result::Result<&'a Value, RpcError> {
    params
        .get(key)
        .filter(|v| !v.is_null())
        .ok_or_else(|| rpc(code::INVALID_PARAMS, format!("{key} is required")))
}

fn param_str<'a>(params: &'a Value, key: &str) -> std::result::Result<&'a str, RpcError> {
    param(params, key)?
        .as_str()
        .ok_or_else(|| rpc(code::INVALID_PARAMS, format!("{key} must be a string")))
}

/// Check and normalise one finding a worker sent.
fn accept_finding(value: Value) -> std::result::Result<ReviewFinding, String> {
    let mut finding: ReviewFinding = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if finding.id.is_empty() || finding.id.len() > 128 {
        return Err("a finding id must be 1 to 128 characters".into());
    }
    finding.title = finding.title.chars().take(300).collect();
    finding.message = finding.message.chars().take(20_000).collect();
    finding.suggestion = finding.suggestion.map(|s| s.chars().take(20_000).collect());
    finding.provider_severity = finding
        .provider_severity
        .map(|s| s.chars().take(40).collect());
    finding.group = finding.group.map(|s| s.chars().take(100).collect());
    if let Some(path) = &finding.path {
        let clean = !path.is_empty()
            && path.len() <= 1024
            && !path.starts_with('/')
            && !path.contains('\\')
            && !path.contains(':')
            && path
                .split('/')
                .all(|s| !s.is_empty() && s != "." && s != "..");
        if !clean {
            return Err(format!("{path:?} is not a path inside the repository"));
        }
    }
    if matches!((finding.start_line, finding.end_line), (Some(start), Some(end)) if end < start)
        || finding.start_line == Some(0)
        || finding.end_line == Some(0)
    {
        return Err("line numbers must be 1 or more, the end not before the start".into());
    }
    if finding.end_line.is_some() && finding.start_line.is_none() {
        return Err("an end line needs a start line".into());
    }
    if finding
        .source_url
        .as_deref()
        .is_some_and(|u| !u.starts_with("https://"))
    {
        finding.source_url = None;
    }
    // The host decides these, whatever the worker sent.
    finding.disposition = review::Disposition::Open;
    finding.review_id.clear();
    finding.provider_id.clear();
    Ok(finding)
}

impl Link {
    fn inner(&self) -> Option<Arc<Inner>> {
        self.inner.upgrade()
    }

    fn current(&self, inner: &Inner) -> bool {
        inner
            .slots
            .lock()
            .expect("slots")
            .get(&self.extension)
            .is_some_and(|s| {
                s.session == self.session && matches!(s.run(), Run::Starting | Run::Active)
            })
    }

    fn log(&self, inner: &Inner, line: String) {
        if let Some(slot) = inner.slots.lock().expect("slots").get_mut(&self.extension) {
            if slot.session == self.session {
                slot.log(line);
            }
        }
    }

    fn version(&self, inner: &Inner) -> String {
        inner
            .slots
            .lock()
            .expect("slots")
            .get(&self.extension)
            .map(|s| s.version.clone())
            .unwrap_or_default()
    }

    fn repo_handle(&self, inner: &Inner, key: &str) -> std::result::Result<RepoHandle, RpcError> {
        inner
            .handles
            .lock()
            .expect("handles")
            .repos
            .get(key)
            .filter(|h| h.extension == self.extension && h.session == self.session)
            .cloned()
            .ok_or_else(|| {
                rpc(
                    code::BAD_HANDLE,
                    "that repository handle is unknown or has expired",
                )
            })
    }

    fn require(
        &self,
        inner: &Inner,
        repository_id: &str,
        capability: Capability,
    ) -> std::result::Result<(), RpcError> {
        if inner
            .granted(&self.extension, &self.version(inner), repository_id)
            .contains(&capability)
        {
            Ok(())
        } else {
            Err(rpc(
                code::NOT_GRANTED,
                format!("{capability} is not granted for this repository"),
            ))
        }
    }

    fn handle_request(
        &self,
        inner: &Arc<Inner>,
        method: &str,
        params: &Value,
    ) -> std::result::Result<Value, RpcError> {
        match method {
            "repository.describe" => {
                let handle = self.repo_handle(inner, param_str(params, "repository")?)?;
                self.require(inner, &handle.repository_id, Capability::RepositoryRead)?;
                let repo = spagitty_core::repo::open(&handle.workdir)
                    .map_err(|e| rpc(code::INTERNAL, e.to_string()))?;
                let head = spagitty_core::repo::head(&repo);
                let forge = spagitty_core::forge::identify_repo(&repo).ok().flatten();
                let remotes: Vec<Value> = spagitty_core::remotes::remotes(&repo)
                    .into_iter()
                    .map(|r| {
                        json!({
                            "name": r.name,
                            "forge": spagitty_core::forge::identify(&r.url).map(|f| f.kind.label()),
                        })
                    })
                    .collect();
                Ok(json!({
                    "name": handle.main_workdir.file_name().map(|n| n.to_string_lossy().into_owned()),
                    "branch": head.branch,
                    "head": head.id,
                    "detached": head.detached,
                    "remotes": remotes,
                    "forge": forge.map(|f| json!({"kind": f.kind, "host": f.host, "slug": f.slug()})),
                }))
            }
            "repository.changes" => {
                let handle = self.repo_handle(inner, param_str(params, "repository")?)?;
                self.require(inner, &handle.repository_id, Capability::RepositoryRead)?;
                let request = match params.get("operationId").and_then(Value::as_str) {
                    Some(operation) => inner
                        .operations
                        .with(operation, &self.extension, &self.session, |op| {
                            match &op.kind {
                                Kind::Review { snapshot, .. } => Some(snapshot::Request {
                                    target: snapshot.target,
                                    scope: snapshot.scope,
                                    base: Some(snapshot.base_commit.clone()),
                                    task_id: snapshot.task_id.clone(),
                                    pull_request_number: snapshot.pull_request_number,
                                }),
                                Kind::Command { .. } => None,
                            }
                        })
                        .ok_or_else(|| rpc(code::BAD_OPERATION, "that operation is not running"))?
                        .ok_or_else(|| {
                            rpc(code::BAD_OPERATION, "that operation is not a review")
                        })?,
                    None => snapshot::Request {
                        target: ReviewTarget::WorkingCopy,
                        scope: ReviewScope::Uncommitted,
                        base: None,
                        task_id: None,
                        pull_request_number: None,
                    },
                };
                let (_, preview) = snapshot::take(&handle.workdir, &request, &[])
                    .map_err(|e| rpc(code::INTERNAL, e.to_string()))?;
                Ok(
                    json!({"files": preview.files, "excluded": preview.excluded, "truncated": preview.truncated}),
                )
            }
            "review.snapshot" => {
                let operation = param_str(params, "operationId")?;
                let snapshot = inner
                    .operations
                    .with(operation, &self.extension, &self.session, |op| {
                        match &op.kind {
                            Kind::Review { snapshot, .. } => Some(snapshot.clone()),
                            Kind::Command { .. } => None,
                        }
                    })
                    .flatten()
                    .ok_or_else(|| rpc(code::BAD_OPERATION, "that is not a running review"))?;
                self.require(inner, &snapshot.repository_id, Capability::ReviewProvide)?;
                Ok(serde_json::to_value(snapshot).unwrap_or(Value::Null))
            }
            "tools.detect" => {
                let tool = param_str(params, "tool")?;
                let entry = inner
                    .entry(&self.extension)
                    .map_err(|e| rpc(code::INTERNAL, e.to_string()))?;
                let declared = entry
                    .manifest
                    .tool(tool)
                    .ok_or_else(|| rpc(code::TOOL_REFUSED, format!("{tool} is not declared")))?
                    .clone();
                if !entry
                    .manifest
                    .capabilities
                    .asks_for(Capability::ToolsExecute)
                {
                    return Err(rpc(code::NOT_GRANTED, "tools.execute is not declared"));
                }
                let detected = self.detect_cached(inner, &declared);
                Ok(serde_json::to_value(detected).unwrap_or(Value::Null))
            }
            "tools.run" => self.run_tool(inner, params),
            "forge.pullRequest.snapshot" => {
                let handle = self.repo_handle(inner, param_str(params, "repository")?)?;
                self.require(
                    inner,
                    &handle.repository_id,
                    Capability::ForgePullRequestRead,
                )?;
                let number = param(params, "number")?
                    .as_u64()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        rpc(code::INVALID_PARAMS, "number must be a positive integer")
                    })?;
                inner
                    .services
                    .pull_request_snapshot(&handle.workdir, number)
            }
            "forge.pullRequest.comment" => {
                let handle = self.repo_handle(inner, param_str(params, "repository")?)?;
                self.require(
                    inner,
                    &handle.repository_id,
                    Capability::ForgePullRequestComment,
                )?;
                let number = param(params, "number")?
                    .as_u64()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        rpc(code::INVALID_PARAMS, "number must be a positive integer")
                    })?;
                let body = param_str(params, "body")?;
                if body.trim().is_empty() || body.len() > 65_536 {
                    return Err(rpc(
                        code::INVALID_PARAMS,
                        "a comment must be 1 to 65536 bytes",
                    ));
                }
                let name = inner
                    .entry(&self.extension)
                    .map(|e| e.manifest.name)
                    .unwrap_or_default();
                {
                    let cancel = match params.get("operationId").and_then(Value::as_str) {
                        Some(op) => inner
                            .operations
                            .with(op, &self.extension, &self.session, |o| o.cancel.clone())
                            .ok_or_else(|| {
                                rpc(code::BAD_OPERATION, "That operation was stopped.")
                            })?,
                        None => Arc::new(AtomicBool::new(false)),
                    };
                    inner.services.post_pull_request_comment(
                        &handle.workdir,
                        number,
                        body,
                        &name,
                        &cancel,
                    )
                }
            }
            "storage.get" | "storage.set" => self.storage(inner, method, params),
            "ui.notify" => {
                let level = params
                    .get("level")
                    .and_then(Value::as_str)
                    .filter(|level| matches!(*level, "info" | "warn" | "error"))
                    .unwrap_or("info");
                let message: String = param_str(params, "message")?.chars().take(500).collect();
                inner.emit(HostEvent::Notice {
                    extension: self.extension.clone(),
                    level: level.into(),
                    message: redact(&message),
                });
                Ok(json!({}))
            }
            _ => Err(rpc(
                code::METHOD_NOT_FOUND,
                format!("the host has no method called {method}"),
            )),
        }
    }

    fn detect_cached(&self, inner: &Inner, tool: &crate::manifest::ExternalTool) -> Detected {
        if let Some(found) = inner
            .slots
            .lock()
            .expect("slots")
            .get(&self.extension)
            .and_then(|s| s.tools.get(&tool.id).cloned())
            .filter(|d| d.found)
        {
            return found;
        }
        let chosen = inner.store.read(|s| {
            s.executables
                .get(&self.extension)
                .and_then(|t| t.get(&tool.id))
                .cloned()
        });
        let detected = tools::detect(tool, chosen.as_deref());
        if let Some(slot) = inner.slots.lock().expect("slots").get_mut(&self.extension) {
            slot.tools.insert(tool.id.clone(), detected.clone());
        }
        inner.changed(&self.extension);
        detected
    }

    fn run_tool(&self, inner: &Arc<Inner>, params: &Value) -> std::result::Result<Value, RpcError> {
        let entry = inner
            .entry(&self.extension)
            .map_err(|e| rpc(code::INTERNAL, e.to_string()))?;
        let tool_id = param_str(params, "tool")?;
        let profile_id = param_str(params, "profile")?;
        let tool = entry
            .manifest
            .tool(tool_id)
            .ok_or_else(|| rpc(code::TOOL_REFUSED, format!("{tool_id} is not declared")))?
            .clone();
        let profile = tool
            .profile(profile_id)
            .ok_or_else(|| {
                rpc(
                    code::TOOL_REFUSED,
                    format!("{tool_id} has no profile called {profile_id}"),
                )
            })?
            .clone();
        let options: BTreeMap<String, Value> = match params.get("options") {
            None | Some(Value::Null) => BTreeMap::new(),
            Some(Value::Object(map)) => map.clone().into_iter().collect(),
            Some(_) => return Err(rpc(code::INVALID_PARAMS, "options must be an object")),
        };
        let args = tools::build_args(&tool, profile_id, &options)
            .map_err(|why| rpc(code::TOOL_REFUSED, why))?;

        // Which operation it belongs to, and so which cancellation ends it.
        let operation = params
            .get("operationId")
            .and_then(Value::as_str)
            .map(str::to_string);
        // Count the entire callback, including version detection and failures.
        // Multiple callbacks can wait concurrently; each owns its own guard.
        let _host_work = match &operation {
            Some(op) => Some(
                inner
                    .operations
                    .host_work(op, &self.extension, &self.session)
                    .ok_or_else(|| rpc(code::BAD_OPERATION, "that operation is not running"))?,
            ),
            None => None,
        };
        let (cancel, op_repository) = match &operation {
            Some(op) => inner
                .operations
                .with(op, &self.extension, &self.session, |o| {
                    let repository_id = match &o.kind {
                        Kind::Review { snapshot, .. } => Some(snapshot.repository_id.clone()),
                        Kind::Command { .. } => None,
                    };
                    (o.cancel.clone(), repository_id)
                })
                .ok_or_else(|| rpc(code::BAD_OPERATION, "that operation is not running"))?,
            None => (Arc::new(AtomicBool::new(false)), None),
        };

        // Where it runs, and which repository's grant it needs.
        let wants_workdir = profile.workdir != Some(crate::manifest::ProfileWorkdir::None);
        let (cwd, repository_id) = if wants_workdir {
            let key = param_str(params, "workdir")?;
            let handles = inner.handles.lock().expect("handles");
            if let Some(wd) = handles.workdirs.get(key) {
                if wd.extension != self.extension || wd.session != self.session {
                    return Err(rpc(code::BAD_HANDLE, "that directory handle is not yours"));
                }
                if operation.as_deref() != Some(wd.operation.as_str()) {
                    return Err(rpc(
                        code::BAD_HANDLE,
                        "that directory belongs to another operation",
                    ));
                }
                (wd.path.clone(), wd.repository_id.clone())
            } else if let Some(repo) = handles.repos.get(key) {
                if repo.extension != self.extension || repo.session != self.session {
                    return Err(rpc(code::BAD_HANDLE, "that repository handle is not yours"));
                }
                (repo.workdir.clone(), repo.repository_id.clone())
            } else {
                return Err(rpc(
                    code::BAD_HANDLE,
                    "that directory handle is unknown or has expired",
                ));
            }
        } else {
            let repository_id = op_repository.clone().unwrap_or_default();
            (entry.dir.clone(), repository_id)
        };
        if !repository_id.is_empty() || wants_workdir {
            self.require(inner, &repository_id, Capability::ToolsExecute)?;
        } else if !entry
            .manifest
            .capabilities
            .asks_for(Capability::ToolsExecute)
        {
            return Err(rpc(code::NOT_GRANTED, "tools.execute is not declared"));
        } else {
            // A package-directory run outside any repository (a version check,
            // a sign-in): allowed when the extension is enabled anywhere.
            let enabled_somewhere = inner.store.read(|s| {
                s.grants.iter().any(|g| {
                    g.extension == self.extension && g.capability == Capability::ToolsExecute
                })
            });
            if !enabled_somewhere {
                return Err(rpc(
                    code::NOT_GRANTED,
                    "tools.execute is not granted anywhere",
                ));
            }
        }

        let detected = self.detect_cached(inner, &tool);
        let program = match (&detected.path, detected.found) {
            (Some(path), true) => path.clone(),
            _ => {
                return Err(RpcError {
                    code: code::TOOL_MISSING,
                    message: detected
                        .reason
                        .clone()
                        .unwrap_or_else(|| format!("{} was not found", tool.label())),
                    data: Some(serde_json::to_value(&detected).unwrap_or(Value::Null)),
                })
            }
        };
        if detected.compatible == Some(false) {
            return Err(RpcError {
                code: code::TOOL_MISSING,
                message: detected
                    .reason
                    .clone()
                    .unwrap_or_else(|| "the installed version is not supported".into()),
                data: Some(serde_json::to_value(&detected).unwrap_or(Value::Null)),
            });
        }

        let run_id = inner.next("run");
        let worker = inner
            .slots
            .lock()
            .expect("slots")
            .get(&self.extension)
            .and_then(|s| s.worker.clone());
        let timeout = profile
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(inner.config.deadline);
        let mut forward = |line: &str| {
            if let Some(op) = &operation {
                inner
                    .operations
                    .with(op, &self.extension, &self.session, |o| {
                        o.last_activity = Instant::now()
                    });
            }
            if let Some(worker) = &worker {
                worker.notify(
                    "tools.output",
                    json!({"runId": run_id, "operationId": operation, "stream": "stdout", "line": line}),
                );
            }
        };
        let outcome = tools::run(
            &program,
            &args,
            Some(&cwd),
            &cancel,
            Some(timeout),
            &mut forward,
        )
        .map_err(|e| {
            rpc(
                code::TOOL_MISSING,
                format!("{} could not be started: {e}", tool.label()),
            )
        })?;

        let record = ToolRun {
            tool: tool.id.clone(),
            profile: profile.id.clone(),
            args: args.iter().map(|a| redact(a)).collect(),
            exit_code: outcome.exit_code,
            cancelled: outcome.cancelled,
            timed_out: outcome.timed_out,
            duration_ms: outcome.duration_ms,
            at: snapshot::now_iso(),
            stderr: outcome
                .stderr
                .chars()
                .rev()
                .take(4000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
        };
        if let Some(slot) = inner.slots.lock().expect("slots").get_mut(&self.extension) {
            slot.tool_runs.push_back(record);
            while slot.tool_runs.len() > 20 {
                slot.tool_runs.pop_front();
            }
        }
        if outcome.cancelled {
            return Err(rpc(code::CANCELLED, "the operation was cancelled"));
        }
        Ok(json!({
            "runId": run_id,
            "exitCode": outcome.exit_code,
            "signalled": outcome.signalled,
            "timedOut": outcome.timed_out,
            "durationMs": outcome.duration_ms,
            "stderr": outcome.stderr.chars().rev().take(4000).collect::<Vec<_>>().into_iter().rev().collect::<String>(),
        }))
    }

    fn storage(
        &self,
        inner: &Inner,
        method: &str,
        params: &Value,
    ) -> std::result::Result<Value, RpcError> {
        let key = param_str(params, "key")?;
        if key.is_empty() || key.len() > 128 {
            return Err(rpc(
                code::INVALID_PARAMS,
                "a key must be 1 to 128 characters",
            ));
        }
        let scope = match params.get("repository").and_then(Value::as_str) {
            Some(handle) => self.repo_handle(inner, handle)?.repository_id,
            None => "global".to_string(),
        };
        if method == "storage.get" {
            let value = inner.store.read(|s| {
                s.data
                    .get(&self.extension)
                    .and_then(|d| d.get(&scope))
                    .and_then(|d| d.get(key))
                    .cloned()
            });
            return Ok(json!({"value": value}));
        }
        let value = params.get("value").cloned().unwrap_or(Value::Null);
        if serde_json::to_vec(&value)
            .map(|b| b.len())
            .unwrap_or(usize::MAX)
            > STORAGE_VALUE
        {
            return Err(rpc(code::LIMIT, "a stored value may be at most 64 KiB"));
        }
        let extension = self.extension.clone();
        let stored = inner.store.update(|s| {
            let held = s
                .data
                .entry(extension)
                .or_default()
                .entry(scope)
                .or_default();
            if value.is_null() {
                held.remove(key);
                return Ok(());
            }
            if !held.contains_key(key) && held.len() >= STORAGE_KEYS {
                return Err(());
            }
            held.insert(key.to_string(), value);
            Ok(())
        });
        match stored {
            Ok(Ok(())) => Ok(json!({})),
            Ok(Err(())) => Err(rpc(code::LIMIT, "at most 256 keys may be stored")),
            Err(error) => Err(rpc(code::INTERNAL, error.to_string())),
        }
    }
}

impl Inbound for Link {
    fn notification(&self, method: &str, params: Value) {
        let Some(inner) = self.inner() else { return };
        if !self.current(&inner) {
            return;
        }
        let operation = params
            .get("operationId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        match method {
            "operation.progress" => {
                let message = params
                    .get("message")
                    .and_then(Value::as_str)
                    .map(|m| redact(&m.chars().take(300).collect::<String>()));
                let seen =
                    inner
                        .operations
                        .with(&operation, &self.extension, &self.session, |op| {
                            op.last_activity = Instant::now();
                            if message.is_some() && !op.is_cancelling() {
                                op.progress = message.clone();
                            }
                            (
                                op.started.elapsed().as_millis() as u64,
                                op.findings.len(),
                                op.is_cancelling(),
                            )
                        });
                match seen {
                    Some((elapsed_ms, findings, false)) if message.is_some() => {
                        inner.emit(HostEvent::OperationProgress {
                            operation,
                            extension: self.extension.clone(),
                            message,
                            elapsed_ms,
                            findings,
                        })
                    }
                    Some(_) => {}
                    None => self.log(
                        &inner,
                        format!("progress for an operation that is not running: {operation}"),
                    ),
                }
            }
            "review.findings" => {
                let list = params
                    .get("findings")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut rejected = Vec::new();
                let outcome =
                    inner
                        .operations
                        .with(&operation, &self.extension, &self.session, |op| {
                            if op.is_cancelling() || !matches!(op.kind, Kind::Review { .. }) {
                                return None;
                            }
                            op.last_activity = Instant::now();
                            for value in list {
                                match accept_finding(value) {
                                    Ok(finding)
                                        if op.findings.iter().any(|f| f.id == finding.id) =>
                                    {
                                        rejected.push(format!(
                                            "a finding id was repeated: {}",
                                            finding.id
                                        ))
                                    }
                                    Ok(_) if op.findings.len() >= operations::MAX_FINDINGS => {
                                        op.dropped_findings += 1
                                    }
                                    Ok(finding) => op.findings.push(finding),
                                    Err(why) => rejected.push(why),
                                }
                            }
                            Some((
                                op.started.elapsed().as_millis() as u64,
                                op.findings.len(),
                                op.progress.clone(),
                            ))
                        });
                for why in rejected {
                    self.log(&inner, format!("finding refused: {why}"));
                }
                match outcome {
                    Some(Some((elapsed_ms, findings, message))) => {
                        inner.emit(HostEvent::OperationProgress {
                            operation,
                            extension: self.extension.clone(),
                            message,
                            elapsed_ms,
                            findings,
                        })
                    }
                    _ => self.log(
                        &inner,
                        format!(
                            "findings for an operation that is not a running review: {operation}"
                        ),
                    ),
                }
            }
            "operation.complete" => {
                let parsed: std::result::Result<CompleteParams, _> = serde_json::from_value(params);
                let Ok(complete) = parsed else {
                    self.log(&inner, "an operation.complete could not be read".into());
                    return;
                };
                let owned = inner
                    .operations
                    .with(
                        &complete.operation_id,
                        &self.extension,
                        &self.session,
                        |_| (),
                    )
                    .is_some();
                if !owned {
                    let note = if inner.operations.recently_finished(&complete.operation_id) {
                        "a second completion was ignored"
                    } else {
                        "a completion for an unknown operation was ignored"
                    };
                    self.log(&inner, format!("{note}: {}", complete.operation_id));
                    return;
                }
                if let Some(op) = inner.operations.finish(&complete.operation_id) {
                    inner.conclude(
                        op,
                        Conclusion::Worker {
                            status: complete.status,
                            message: complete.message.map(|m| m.chars().take(2000).collect()),
                            review: complete.review,
                        },
                    );
                }
            }
            "log" => {
                let level = params
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("info");
                let message = params.get("message").and_then(Value::as_str).unwrap_or("");
                let line: String = message.chars().take(1000).collect();
                self.log(&inner, format!("[{level}] {}", redact(&line)));
            }
            other => self.log(
                &inner,
                format!("an unknown notification was ignored: {other}"),
            ),
        }
    }

    fn request(&self, method: &str, params: Value) -> std::result::Result<Value, RpcError> {
        let Some(inner) = self.inner() else {
            return Err(rpc(code::NOT_ACTIVE, "the host is closing"));
        };
        if !self.current(&inner) {
            return Err(rpc(code::NOT_ACTIVE, "this extension is not active"));
        }
        let _host_work = match params.get("operationId").and_then(Value::as_str) {
            Some(op) => Some(
                inner
                    .operations
                    .host_work(op, &self.extension, &self.session)
                    .ok_or_else(|| rpc(code::BAD_OPERATION, "That operation is not running."))?,
            ),
            None => None,
        };
        self.handle_request(&inner, method, &params)
    }

    fn disconnected(&self, reason: String) {
        let Some(inner) = self.inner() else { return };
        let crashed = {
            let mut slots = inner.slots.lock().expect("slots");
            let Some(slot) = slots.get_mut(&self.extension) else {
                return;
            };
            if slot.session != self.session {
                return;
            }
            let expected = matches!(slot.run(), Run::Stopping | Run::Idle);
            if let Some(worker) = slot.worker.take() {
                slot.last_stderr = worker.stderr_tail();
            }
            slot.activated = false;
            if expected {
                slot.run = Some(Run::Idle);
            } else {
                slot.run = Some(Run::Failed);
                slot.error = Some(format!("It stopped unexpectedly. {reason}"));
                slot.crashes.push_back(Instant::now());
                while slot.crashes.len() > 10 {
                    slot.crashes.pop_front();
                }
            }
            !expected
        };
        let message = if crashed {
            format!("The extension stopped unexpectedly. {reason}")
        } else {
            "The extension was stopped.".to_string()
        };
        for op in inner.operations.finish_all(&self.extension) {
            if op.session == self.session {
                inner.conclude(op, Conclusion::Failed(message.clone()));
            } else {
                inner.operations.insert(op);
            }
        }
        inner.forget_session(&self.extension, &self.session);
        inner.changed(&self.extension);
    }
}

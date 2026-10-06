// SPDX-License-Identifier: GPL-3.0-or-later

//! The extension host's Tauri layer (FEAT-096).
//!
//! Thin, like [`crate::farm`]: it builds the host with the application's
//! directories, forwards commands to it, turns its events into one webview
//! event, and supplies the two services only the desktop can — anything that
//! needs the forge token, which never leaves the backend, and anything that
//! needs the person, who is in the window.
//!
//! Every command is `#[tauri::command(async)]`: starting a worker waits on its
//! handshake, and that wait must cost a thread, not the window (BUG-020).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use spagitty_extensions::capabilities::Capability;
use spagitty_extensions::history::{Requester, ReviewRecord};
use spagitty_extensions::host::{
    Config, Events, ExtensionHost, HostEvent, InstallPreview, Invocation, Listing, Services,
    Started,
};
use spagitty_extensions::manifest::{Manifest, ReviewScope, ReviewTarget};
use spagitty_extensions::protocol::{code, RpcError};
use spagitty_extensions::registry::{Installed, Paths};
use spagitty_extensions::review::Disposition;
use spagitty_extensions::snapshot::{self, Preview};
use spagitty_extensions::tools::Detected;
use spagitty_extensions::Error;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// The one event the extension screens subscribe to.
pub const EVENT: &str = "extension-event";
/// Asks the window to show the person something and say yes or no.
pub const CONFIRM: &str = "extension-confirm";
/// How long a confirmation waits for an answer before it counts as no.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(300);

type Result<T> = std::result::Result<T, Error>;

/// The open host, built once at startup.
#[derive(Default)]
pub struct ExtensionsState {
    host: Mutex<Option<ExtensionHost>>,
    confirmations: Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>,
}

impl ExtensionsState {
    pub fn host(&self) -> Result<ExtensionHost> {
        self.host
            .lock()
            .expect("extension host lock")
            .clone()
            .ok_or_else(|| Error::Refused("Extensions are not available.".into()))
    }
}

#[derive(Debug)]
struct Emit<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> Events for Emit<R> {
    fn emit(&self, event: HostEvent) {
        let _ = self.app.emit(EVENT, &event);
    }
}

/// What the window is asked to confirm.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmation {
    pub id: String,
    pub extension: String,
    pub kind: &'static str,
    pub title: String,
    pub target: String,
    pub body: String,
}

struct Desktop<R: Runtime> {
    app: AppHandle<R>,
    confirmations: Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>,
}

impl<R: Runtime> Desktop<R> {
    /// Show the person `confirmation` and wait for their answer. No answer is
    /// a no.
    fn ask(&self, mut confirmation: Confirmation) -> bool {
        let id = format!(
            "confirm-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        confirmation.id = id.clone();
        let (send, receive) = mpsc::channel();
        self.confirmations
            .lock()
            .expect("confirmations")
            .insert(id.clone(), send);
        if self.app.emit(CONFIRM, &confirmation).is_err() {
            self.confirmations
                .lock()
                .expect("confirmations")
                .remove(&id);
            return false;
        }
        let answer = receive.recv_timeout(CONFIRM_TIMEOUT).unwrap_or(false);
        self.confirmations
            .lock()
            .expect("confirmations")
            .remove(&id);
        answer
    }
}

impl<R: Runtime> Services for Desktop<R> {
    fn pull_request_snapshot(
        &self,
        workdir: &Path,
        number: u64,
    ) -> std::result::Result<Value, RpcError> {
        crate::forge_bridge::pull_request_snapshot(&self.app, workdir, number)
    }

    fn post_pull_request_comment(
        &self,
        workdir: &Path,
        number: u64,
        body: &str,
        extension_name: &str,
    ) -> std::result::Result<Value, RpcError> {
        let target = crate::forge_bridge::describe_pull_request(workdir, number)?;
        let approved = self.ask(Confirmation {
            id: String::new(),
            extension: extension_name.to_string(),
            kind: "pullRequestComment",
            title: format!("{extension_name} wants to comment on {target}"),
            target,
            body: body.to_string(),
        });
        if !approved {
            return Err(RpcError::new(code::DECLINED, "the person declined"));
        }
        crate::forge_bridge::post_pull_request_comment(&self.app, workdir, number, body)
    }
}

/// Build the host. Called once, from `setup`.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    let state = ExtensionsState::default();
    let data = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("extensions"))
        .unwrap_or_else(|_| std::env::temp_dir().join("spagitty-extensions"));
    // Bundled packages are found through the resource API on every target,
    // never through the checkout (TASK-055).
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("extensions"))
        .filter(|dir| dir.is_dir());
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let paths = Paths {
        data,
        bundled,
        exe_dir,
    };
    let config = Config::new(&app.package_info().version.to_string(), paths);
    let host = ExtensionHost::new(
        config,
        Arc::new(Emit { app: app.clone() }),
        Arc::new(Desktop {
            app: app.clone(),
            confirmations: state.confirmations.clone(),
        }),
    );
    *state.host.lock().expect("extension host lock") = Some(host);
    app.manage(state);
}

/// Stop every worker. Called as the application exits.
pub fn shutdown<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<ExtensionsState>() {
        if let Ok(host) = state.host() {
            host.shutdown();
        }
    }
}

/// A review request, as the window sends it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    pub target: ReviewTarget,
    pub scope: ReviewScope,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub pull_request_number: Option<u64>,
}

impl From<ReviewRequest> for snapshot::Request {
    fn from(r: ReviewRequest) -> Self {
        snapshot::Request {
            target: r.target,
            scope: r.scope,
            base: r.base,
            task_id: r.task_id,
            pull_request_number: r.pull_request_number,
        }
    }
}

#[tauri::command(async)]
pub fn extensions_list(
    state: State<'_, ExtensionsState>,
    workdir: Option<PathBuf>,
) -> Result<Listing> {
    Ok(state.host()?.list(workdir.as_deref()))
}

#[tauri::command(async)]
pub fn extensions_inspect(
    state: State<'_, ExtensionsState>,
    path: PathBuf,
    workdir: Option<PathBuf>,
) -> Result<InstallPreview> {
    state.host()?.inspect_package(&path, workdir.as_deref())
}

#[tauri::command(async)]
pub fn extensions_install(state: State<'_, ExtensionsState>, token: String) -> Result<Installed> {
    state.host()?.install(&token)
}

#[tauri::command(async)]
pub fn extensions_rollback(state: State<'_, ExtensionsState>, id: String) -> Result<Installed> {
    state.host()?.rollback(&id)
}

#[tauri::command(async)]
pub fn extensions_uninstall(
    state: State<'_, ExtensionsState>,
    id: String,
    keep_history: bool,
    workdir: Option<PathBuf>,
) -> Result<()> {
    state
        .host()?
        .uninstall(&id, keep_history, workdir.as_deref())
}

#[tauri::command(async)]
pub fn extensions_attach(state: State<'_, ExtensionsState>, path: PathBuf) -> Result<Manifest> {
    state.host()?.attach_development(&path)
}

#[tauri::command(async)]
pub fn extensions_restart(state: State<'_, ExtensionsState>, id: String) -> Result<()> {
    state.host()?.restart(&id)
}

#[tauri::command(async)]
pub fn extensions_enable(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
    optional: Vec<Capability>,
    consent: Option<String>,
) -> Result<()> {
    state
        .host()?
        .enable(&id, &workdir, &optional, consent.as_deref())
}

#[tauri::command(async)]
pub fn extensions_disable(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
    forget: bool,
) -> Result<()> {
    state.host()?.disable(&id, &workdir, forget)
}

#[tauri::command(async)]
pub fn extensions_set_grant(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
    capability: Capability,
    granted: bool,
) -> Result<()> {
    state.host()?.set_grant(&id, &workdir, capability, granted)
}

#[tauri::command(async)]
pub fn extensions_set_setting(
    state: State<'_, ExtensionsState>,
    id: String,
    key: String,
    value: Value,
    workdir: Option<PathBuf>,
) -> Result<()> {
    state
        .host()?
        .set_setting(&id, &key, value, workdir.as_deref())
}

#[tauri::command(async)]
pub fn extensions_choose_executable(
    state: State<'_, ExtensionsState>,
    id: String,
    tool: String,
    path: Option<PathBuf>,
) -> Result<Detected> {
    state.host()?.choose_executable(&id, &tool, path.as_deref())
}

#[tauri::command(async)]
pub fn extensions_detect_tool(
    state: State<'_, ExtensionsState>,
    id: String,
    tool: String,
) -> Result<Detected> {
    state.host()?.detect_tool(&id, &tool)
}

#[tauri::command(async)]
pub fn extensions_run_command(
    state: State<'_, ExtensionsState>,
    id: String,
    command: String,
    invocation: Invocation,
) -> Result<Started> {
    state.host()?.run_command(&id, &command, &invocation)
}

#[tauri::command(async)]
pub fn extensions_preview_review(
    state: State<'_, ExtensionsState>,
    id: String,
    provider: String,
    request: ReviewRequest,
    workdir: PathBuf,
) -> Result<Preview> {
    state
        .host()?
        .preview_review(&id, &provider, &request.into(), &workdir)
}

#[tauri::command(async)]
pub fn extensions_start_review(
    state: State<'_, ExtensionsState>,
    id: String,
    provider: String,
    request: ReviewRequest,
    workdir: PathBuf,
) -> Result<Started> {
    state
        .host()?
        .start_review(&id, &provider, &request.into(), &workdir, Requester::Person)
}

#[tauri::command(async)]
pub fn extensions_cancel(state: State<'_, ExtensionsState>, operation: String) -> Result<()> {
    state.host()?.cancel(&operation)
}

#[tauri::command(async)]
pub fn extensions_reviews(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
) -> Result<Vec<ReviewRecord>> {
    state.host()?.reviews(&id, &workdir)
}

#[tauri::command(async)]
pub fn extensions_set_disposition(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
    review: String,
    finding: String,
    disposition: Disposition,
) -> Result<ReviewRecord> {
    state
        .host()?
        .set_disposition(&id, &workdir, &review, &finding, disposition)
}

#[tauri::command(async)]
pub fn extensions_delete_reviews(
    state: State<'_, ExtensionsState>,
    id: String,
    workdir: PathBuf,
    review: Option<String>,
) -> Result<()> {
    state
        .host()?
        .delete_reviews(&id, &workdir, review.as_deref())
}

#[tauri::command(async)]
pub fn extensions_panel(
    state: State<'_, ExtensionsState>,
    id: String,
    panel: String,
    invocation: Invocation,
) -> Result<Value> {
    state.host()?.resolve_panel(&id, &panel, &invocation)
}

#[tauri::command(async)]
pub fn extensions_suggested_bases(workdir: PathBuf) -> Result<Vec<String>> {
    snapshot::suggested_bases(&workdir)
}

/// The window's answer to a confirmation.
#[tauri::command(async)]
pub fn extensions_confirm(
    state: State<'_, ExtensionsState>,
    id: String,
    approved: bool,
) -> Result<()> {
    if let Some(waiter) = state
        .confirmations
        .lock()
        .expect("confirmations")
        .remove(&id)
    {
        let _ = waiter.send(approved);
    }
    Ok(())
}

/// The data directory, for the Settings section's "where are they kept".
#[tauri::command(async)]
pub fn extensions_location(state: State<'_, ExtensionsState>) -> Result<Value> {
    let host = state.host()?;
    Ok(json!({"data": host.paths().data, "bundled": host.paths().bundled}))
}

/// What sending findings to an agent did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    /// The farm task the findings went to.
    pub task: String,
    pub message: String,
}

/// Send selected findings from a review to an agent (FEAT-097).
///
/// A review of committed changes still at `HEAD` becomes a **Draft** farm task
/// whose description quotes the findings as evidence, never as orders; a person
/// readies it, and the farm cuts its worktree from `HEAD` then, so nothing here
/// touches the checkout. Anything else is refused with the reason.
#[tauri::command(async)]
pub fn extensions_send_findings(
    state: State<'_, ExtensionsState>,
    farm: State<'_, crate::farm::FarmState>,
    id: String,
    workdir: PathBuf,
    review: String,
    findings: Vec<String>,
) -> Result<Sent> {
    use spagitty_extensions::repair;
    let host = state.host()?;
    let record = host
        .reviews(&id, &workdir)?
        .into_iter()
        .find(|r| r.result.review_id == review)
        .ok_or_else(|| Error::Refused("That review is no longer in the history.".into()))?;
    let chosen = repair::selected(&record, &findings)?;
    match record.snapshot.target {
        ReviewTarget::WorkingCopy => {}
        ReviewTarget::FarmTask => {
            return Err(Error::Refused(
                "A farm task's findings go back to it from the Farm screen.".into(),
            ))
        }
        ReviewTarget::PullRequest => {
            return Err(Error::Refused(
                "Findings on a pull request are answered on the pull request.".into(),
            ))
        }
    }
    let head = spagitty_core::repo::open(&workdir)
        .ok()
        .and_then(|repo| spagitty_core::repo::head(&repo).id);
    if let Some(why) = repair::refuse(&record, head.as_deref()) {
        return Err(Error::Refused(why));
    }
    let service = farm
        .service_for(&workdir)
        .filter(|service| service.farm().is_some())
        .ok_or_else(|| {
            Error::Refused(
                "Create a farm for this repository on the Farm screen first; the repair task is added to it.".into(),
            )
        })?;
    let name = host
        .manifest(&id)
        .map(|m| m.name)
        .unwrap_or_else(|| id.clone());
    let words = repair::task(&record, &name, &chosen);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut task = spagitty_farm::model::Task::new(
        spagitty_farm::model::TaskId::new("TASK-0000"),
        words.title,
        now,
    );
    task.description = words.description;
    task.acceptance_criteria = words.acceptance_criteria;
    let added = service
        .add_task(task)
        .map_err(|error| Error::Refused(error.to_string()))?;
    for finding in &findings {
        host.set_disposition(&id, &workdir, &review, finding, Disposition::SentToAgent)?;
    }
    Ok(Sent {
        task: added.id.as_str().to_string(),
        message: format!(
            "{} is in the farm as a draft. Ready it there to start an agent.",
            added.id.as_str()
        ),
    })
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Calls, as the screen reaches them (FEAT-160).
//!
//! The rules are `spagitty_core::calls`: the file formats, resolving
//! variables, building and sending a request, history. This module is what
//! they need from the application: where the repository's calls folder is,
//! the person's own state and history in the data directory, Settings ›
//! Calls, and a thread for each send.
//!
//! # What is kept where
//!
//! - **The repository**, under the calls folder (`.spagitty/calls/` unless
//!   Settings says otherwise): collections, environments' plain values,
//!   shared variables, the repository's overrides. Meant to be committed.
//! - **The data directory**, under `calls/<repository>/` where
//!   `<repository>` is a hash of its path: `state.json` (the active
//!   environment, open calls, collapsed collections, the filter) and
//!   `history/`. Never in the repository, never sent anywhere.
//!
//! # A send
//!
//! [`calls_send`] builds the request on the command's own thread, so a
//! missing variable is an error the screen shows at once and nothing is
//! sent. Then a thread sends it; `calls-chunk` carries an event stream as it
//! arrives and `calls-done` the end, whatever it was. [`calls_cancel`] tells
//! the screen at once and lets the thread end on its own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use spagitty_core::calls::body::{self, Shape, Utf8Stream};
use spagitty_core::calls::build::{self, Shown};
use spagitty_core::calls::history::{Entry, History};
use spagitty_core::calls::model::{new_id, Call, Collection, Environment, Shared};
use spagitty_core::calls::send::{self, ClientCert, Failure};
use spagitty_core::calls::settings::{effective, host_of, CallsSettings};
use spagitty_core::calls::store::{Folder, Stored};
use spagitty_core::calls::vars::scope_of;
use spagitty_farm::assign::record::fnv;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::commands::AppState;
use crate::settings;

/// A piece of an event stream, as it arrived.
pub const CHUNK: &str = "calls-chunk";
/// A send ended: answered, failed or cancelled.
pub const DONE: &str = "calls-done";
/// How many responses' bytes are held for *Save as…*.
const HELD: usize = 8;

type Result<T> = std::result::Result<T, String>;

#[derive(Default)]
pub struct CallsState {
    /// The sends in flight, by the screen's id for each.
    sends: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// The last few responses' bytes, newest last, for *Save as…*.
    bodies: Mutex<Vec<(String, Arc<Vec<u8>>)>>,
}

pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(CallsState::default());
}

/// The folder a repository's state and history are in.
fn data<R: Runtime>(app: &AppHandle<R>, repo: &Path) -> Result<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir
        .join("calls")
        .join(format!("{:016x}", fnv(repo.to_string_lossy().as_bytes()))))
}

/// The person's Settings › Calls with the repository's overrides laid over
/// them, and the calls folder those settings name.
fn place<R: Runtime>(
    app: &AppHandle<R>,
    repo: &Path,
) -> Result<(Folder, CallsSettings, Vec<String>)> {
    let mine = settings::load(app).calls;
    // The overrides are in the folder the person's settings name, so they are
    // read from there; an override of the folder itself then moves it.
    let first = Folder::new(repo, &mine.folder).map_err(|e| e.to_string())?;
    let (taken, overridden) = effective(&mine, first.settings().as_ref());
    let folder = Folder::new(repo, &taken.folder).map_err(|e| e.to_string())?;
    Ok((folder, taken, overridden))
}

/// An open call: which collection, which call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub stem: String,
    pub id: String,
}

/// What is the person's, per repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersonState {
    /// The active environment's name.
    pub active: Option<String>,
    pub open: Vec<Opened>,
    pub current: Option<Opened>,
    /// Collections drawn closed, by stem.
    pub collapsed: Vec<String>,
    pub filter: String,
}

fn read_state(dir: &Path) -> PersonState {
    std::fs::read_to_string(dir.join("state.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_state(dir: &Path, state: &PersonState) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("state.json"), text).map_err(|e| e.to_string())
}

/// Everything the screen shows, in one answer.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    /// The calls folder, from the working tree.
    pub folder: String,
    pub collections: Vec<Stored<Collection>>,
    pub environments: Vec<Stored<Environment>>,
    pub shared: Shared,
    pub shared_error: Option<String>,
    pub state: PersonState,
    pub settings: CallsSettings,
    /// The settings the repository overrides.
    pub overridden: Vec<String>,
    /// Sends since midnight, for *History*'s count.
    pub today: usize,
}

#[tauri::command(async)]
pub fn calls_load<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<Loaded> {
    let repo = state.open_path("calls_load").map_err(|e| e.to_string())?;
    let (folder, settings, overridden) = place(&app, &repo)?;
    let dir = data(&app, &repo)?;
    let (shared, shared_error) = match folder.shared() {
        Ok(shared) => (shared, None),
        Err(e) => (Shared::default(), Some(e.to_string())),
    };
    Ok(Loaded {
        folder: folder.relative.clone(),
        collections: folder.collections(),
        environments: folder.environments(),
        shared,
        shared_error,
        state: read_state(&dir),
        settings,
        overridden,
        today: today(&History::new(&dir.join("history")).list()),
    })
}

/// Sends since local midnight, near enough: the last 24 hours' calendar day
/// in UTC is what the data has, and the count is a hint.
fn today(entries: &[Entry]) -> usize {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let midnight = now - now.rem_euclid(86_400_000);
    entries.iter().filter(|entry| entry.at >= midnight).count()
}

/// Save a collection. `stem` is the file it is written as, `was` the file it
/// was in before a rename; both absent for a new one, which is named from
/// the collection. Answers with the stem written.
#[tauri::command(async)]
pub fn calls_save_collection<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    collection: Collection,
    stem: Option<String>,
    was: Option<String>,
) -> Result<String> {
    let repo = state
        .open_path("calls_save_collection")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    folder
        .write_collection(&collection, stem.as_deref(), was.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn calls_delete_collection<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    stem: String,
) -> Result<()> {
    let repo = state
        .open_path("calls_delete_collection")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    folder.delete_collection(&stem).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn calls_save_state<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    person: PersonState,
) -> Result<()> {
    let repo = state
        .open_path("calls_save_state")
        .map_err(|e| e.to_string())?;
    write_state(&data(&app, &repo)?, &person)
}

/// A send the screen asked for.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAsk {
    /// The screen's id for this send, for the events and for *Cancel*.
    pub send_id: String,
    /// The collection the call is in, for its own variables.
    pub stem: Option<String>,
    /// The call as it is on screen, which may be ahead of the file.
    pub call: Call,
    pub environment: Option<String>,
}

/// A response, as the screen shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseView {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub millis: u64,
    pub size: u64,
    pub truncated: bool,
    pub shape: Shape,
    pub media_type: String,
    /// The body, when it is text. A binary body is held for *Save as…*.
    pub text: Option<String>,
    /// Over [`body::COLOUR_LIMIT`]: shown without colour.
    pub plain: bool,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub send_id: String,
    pub call_id: String,
    pub response: Option<ResponseView>,
    pub error: Option<String>,
    pub cancelled: bool,
    /// What was sent, with secrets as their names.
    pub sent: Shown,
    pub history_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub send_id: String,
    pub text: String,
}

/// The values of an environment's secrets and kept values, from the
/// keychain. FEAT-161 fills this.
fn private_values(_repo: &Path, _environment: Option<&str>) -> HashMap<String, String> {
    HashMap::new()
}

/// Build the call and start sending it. A variable with no value is an
/// error here, before anything is sent.
#[tauri::command(async)]
pub fn calls_send<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    calls: State<'_, CallsState>,
    ask: SendAsk,
) -> Result<()> {
    let repo = state.open_path("calls_send").map_err(|e| e.to_string())?;
    let (folder, settings, _) = place(&app, &repo)?;

    let environment = match &ask.environment {
        Some(name) => folder
            .environments()
            .into_iter()
            .find(|stored| stored.stem == *name)
            .and_then(|stored| stored.value),
        None => None,
    };
    let shared = folder.shared().unwrap_or_default();
    let collection_vars = ask
        .stem
        .as_deref()
        .and_then(|stem| folder.collection(stem).ok())
        .map(|collection| collection.vars)
        .unwrap_or_default();
    let env_private = private_values(&repo, environment.as_ref().map(|env| env.name.as_str()));
    let shared_private = private_values(&repo, None);
    let scope = scope_of(
        environment.as_ref().map(|env| (env, &env_private)),
        (&shared, &shared_private),
        &collection_vars,
    );

    let version = app.package_info().version.to_string();
    let built = build::build(
        &ask.call,
        &scope,
        &settings.every_headers(&version),
        &repo,
        None,
    )
    .map_err(|e| e.to_string())?;

    let mut options = settings.options(
        environment
            .as_ref()
            .map(|env| env.check_tls)
            .unwrap_or(true),
    );
    if let Some(cert) = settings.cert_for(&host_of(&built.out.url)) {
        let read = |path: &str| {
            std::fs::read_to_string(path)
                .map_err(|e| format!("the client certificate {path} could not be read: {e}"))
        };
        options.client_cert = Some(ClientCert {
            cert_pem: read(&cert.cert)?,
            key_pem: read(&cert.key)?,
        });
    }

    let cancel = Arc::new(AtomicBool::new(false));
    calls
        .sends
        .lock()
        .map_err(|e| e.to_string())?
        .insert(ask.send_id.clone(), cancel.clone());

    let history = History::new(&data(&app, &repo)?.join("history"));
    let private = scope.private_values();
    let cap = settings.history as usize;
    let environment_name = environment.as_ref().map(|env| env.name.clone());
    std::thread::spawn(move || {
        let mut stream = Utf8Stream::default();
        let sent = send::send(&built.out, &options, &cancel, |headers, piece| {
            let media = body::media_type(
                headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                    .map(|(_, value)| value.as_str()),
            );
            if media == "text/event-stream" {
                let text = stream.push(piece);
                if !text.is_empty() {
                    let _ = app.emit(
                        CHUNK,
                        &Chunk {
                            send_id: ask.send_id.clone(),
                            text: spagitty_core::calls::history::redact(&text, &private),
                        },
                    );
                }
            }
        });

        let calls = app.state::<CallsState>();
        if let Ok(mut sends) = calls.sends.lock() {
            sends.remove(&ask.send_id);
        }

        let mut entry = Entry {
            id: new_id().replacen("c_", "h_", 1),
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            call_id: Some(ask.call.id.clone()),
            collection: ask.stem.clone(),
            name: ask.call.name.clone(),
            environment: environment_name,
            request: built.shown.clone(),
            ..Entry::default()
        };

        let done = match sent {
            Err(Failure::Cancelled) => Done {
                send_id: ask.send_id.clone(),
                call_id: ask.call.id.clone(),
                response: None,
                error: None,
                cancelled: true,
                sent: built.shown.clone(),
                history_id: None,
            },
            Err(failure) => {
                entry.error = Some(failure.to_string());
                let kept = history.add(entry, &[], cap, &private).ok();
                Done {
                    send_id: ask.send_id.clone(),
                    call_id: ask.call.id.clone(),
                    response: None,
                    error: Some(spagitty_core::calls::history::redact(
                        &failure.to_string(),
                        &private,
                    )),
                    cancelled: false,
                    sent: built.shown.clone(),
                    history_id: kept.map(|entry| entry.id),
                }
            }
            Ok(incoming) => {
                let content_type = incoming.header("content-type").map(str::to_string);
                let shape = body::shape(content_type.as_deref(), &incoming.body);
                let text = match shape {
                    Shape::Binary => None,
                    _ => Some(String::from_utf8_lossy(&incoming.body).into_owned()),
                };
                entry.status = Some(incoming.status);
                entry.reason = incoming.reason.clone();
                entry.millis = incoming.millis;
                entry.size = incoming.body.len() as u64;
                entry.headers = incoming.headers.clone();
                let kept = history.add(entry, &incoming.body, cap, &private).ok();
                let bytes = Arc::new(incoming.body);
                if let Ok(mut held) = calls.bodies.lock() {
                    held.push((ask.send_id.clone(), bytes.clone()));
                    let over = held.len().saturating_sub(HELD);
                    held.drain(..over);
                }
                Done {
                    send_id: ask.send_id.clone(),
                    call_id: ask.call.id.clone(),
                    response: Some(ResponseView {
                        status: incoming.status,
                        reason: incoming.reason,
                        headers: incoming.headers,
                        millis: incoming.millis,
                        size: bytes.len() as u64,
                        truncated: incoming.truncated,
                        shape,
                        media_type: body::media_type(content_type.as_deref()),
                        plain: bytes.len() > body::COLOUR_LIMIT,
                        text,
                        url: incoming.url,
                    }),
                    error: None,
                    cancelled: false,
                    sent: built.shown.clone(),
                    history_id: kept.map(|entry| entry.id),
                }
            }
        };
        let _ = app.emit(DONE, &done);
    });
    Ok(())
}

/// Stop waiting for a send. The screen is told at once; the request may
/// still be finishing on its thread, and what it brings is dropped.
#[tauri::command]
pub fn calls_cancel<R: Runtime>(
    app: AppHandle<R>,
    calls: State<'_, CallsState>,
    send_id: String,
) -> Result<()> {
    let flag = calls
        .sends
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&send_id);
    if let Some(flag) = flag {
        flag.store(true, Ordering::Relaxed);
    }
    let _ = app.emit(
        DONE,
        &Done {
            send_id,
            call_id: String::new(),
            response: None,
            error: None,
            cancelled: true,
            sent: Shown::default(),
            history_id: None,
        },
    );
    Ok(())
}

/// Write a response's bytes, as they arrived, to `path`.
#[tauri::command(async)]
pub fn calls_save_body(calls: State<'_, CallsState>, send_id: String, path: String) -> Result<()> {
    let bytes = calls
        .bodies
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .find(|(id, _)| *id == send_id)
        .map(|(_, bytes)| bytes.clone())
        .ok_or_else(|| "that response is no longer held; send the call again".to_string())?;
    std::fs::write(&path, bytes.as_slice()).map_err(|e| format!("{path} could not be written: {e}"))
}

#[tauri::command(async)]
pub fn calls_history<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>> {
    let repo = state
        .open_path("calls_history")
        .map_err(|e| e.to_string())?;
    Ok(History::new(&data(&app, &repo)?.join("history")).list())
}

/// A send's response body as it was kept, up to 1 MB.
#[tauri::command(async)]
pub fn calls_history_body<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<String>> {
    let repo = state
        .open_path("calls_history_body")
        .map_err(|e| e.to_string())?;
    Ok(History::new(&data(&app, &repo)?.join("history"))
        .body(&id)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

#[tauri::command(async)]
pub fn calls_history_clear<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    let repo = state
        .open_path("calls_history_clear")
        .map_err(|e| e.to_string())?;
    History::new(&data(&app, &repo)?.join("history"))
        .clear()
        .map_err(|e| e.to_string())
}

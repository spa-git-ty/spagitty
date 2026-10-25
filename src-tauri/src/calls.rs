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
use spagitty_core::calls::keep::{self, Kept, KeptLog};
use spagitty_core::calls::model::{new_id, Call, Collection, Environment, Shared, Variable};
use spagitty_core::calls::send::{self, ClientCert, Failure};
use spagitty_core::calls::settings::{effective, host_of, CallsSettings};
use spagitty_core::calls::store::{Folder, Stored};
use spagitty_core::calls::vars::scope_of;
use spagitty_core::calls::vault::{self, Keychain};
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
    /// What the call's *every call* rules read (FEAT-161); a value of `None`
    /// found nothing and is now stale.
    pub kept: Vec<keep::Applied>,
    /// Why what they read could not be kept.
    pub kept_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub send_id: String,
    pub text: String,
}

/// The values of an environment's secrets and kept values, from the
/// keychain: `environment` is its stem, or `None` for the shared ones. Only
/// asked when `vars` has something private, so a machine with no keychain
/// still sends plain calls.
fn private_values(
    repo: &Path,
    environment: Option<&str>,
    vars: &[Variable],
) -> Result<HashMap<String, String>> {
    if !vars.iter().any(Variable::private) {
        return Ok(HashMap::new());
    }
    vault::values(&Keychain, &repo_key(repo), environment)
        .map(|values| values.into_iter().collect())
        .map_err(|e| format!("the keychain could not be read: {e}"))
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
    let env_stem = environment.as_ref().and(ask.environment.as_deref());
    let env_private = private_values(
        &repo,
        env_stem,
        environment
            .as_ref()
            .map(|env| env.vars.as_slice())
            .unwrap_or(&[]),
    )?;
    let shared_private = private_values(&repo, None, &shared.vars)?;
    // Where this send's kept values go, and which are switched off there.
    let keep_in = env_stem.map(str::to_string);
    let frozen: Vec<String> = environment
        .as_ref()
        .map(|env| &env.vars)
        .unwrap_or(&shared.vars)
        .iter()
        .filter(|var| var.frozen)
        .map(|var| var.name.clone())
        .collect();
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
                kept: Vec::new(),
                kept_error: None,
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
                    kept: Vec::new(),
                    kept_error: None,
                }
            }
            Ok(incoming) => {
                let content_type = incoming.header("content-type").map(str::to_string);
                let shape = body::shape(content_type.as_deref(), &incoming.body);
                let text = match shape {
                    Shape::Binary => None,
                    _ => Some(String::from_utf8_lossy(&incoming.body).into_owned()),
                };
                let applied = match &text {
                    Some(body) if !ask.call.keep.is_empty() => {
                        keep::apply(&ask.call.keep, shape, body, |var| {
                            frozen.iter().any(|name| name == var)
                        })
                    }
                    _ => Vec::new(),
                };
                let kept_error =
                    record_kept(&app, &repo, keep_in.as_deref(), &ask.call, &applied).err();
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
                    kept: applied,
                    kept_error,
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
            kept: Vec::new(),
            kept_error: None,
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

// --- Environments, secrets and kept values (FEAT-161) ----------------------

/// The repository's key in the vault: the same hash as its data directory.
fn repo_key(repo: &Path) -> String {
    format!("{:016x}", fnv(repo.to_string_lossy().as_bytes()))
}

fn kept_log(dir: &Path) -> PathBuf {
    dir.join("kept.json")
}

/// The environments and the shared variables, as the screen shows them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environments {
    pub environments: Vec<Stored<Environment>>,
    pub shared: Shared,
    pub shared_error: Option<String>,
}

fn environments_of(folder: &Folder) -> Environments {
    let (shared, shared_error) = match folder.shared() {
        Ok(shared) => (shared, None),
        Err(e) => (Shared::default(), Some(e.to_string())),
    };
    Environments {
        environments: folder.environments(),
        shared,
        shared_error,
    }
}

fn environment_named(folder: &Folder, stem: &str) -> Result<Environment> {
    folder
        .environments()
        .into_iter()
        .find(|stored| stored.stem == stem)
        .and_then(|stored| stored.value)
        .ok_or_else(|| format!("there is no environment {stem} that can be read"))
}

/// The variables of an environment, by stem, or the shared ones for `None`.
fn vars_of(folder: &Folder, environment: Option<&str>) -> Result<Vec<Variable>> {
    match environment {
        Some(stem) => environment_named(folder, stem).map(|env| env.vars),
        None => folder
            .shared()
            .map(|shared| shared.vars)
            .map_err(|e| e.to_string()),
    }
}

/// Change an environment's variables, or the shared ones, and write the file
/// when `change` says it changed something.
fn change_vars(
    folder: &Folder,
    environment: Option<&str>,
    change: impl FnOnce(&mut Vec<Variable>) -> bool,
) -> Result<()> {
    match environment {
        Some(stem) => {
            let mut env = environment_named(folder, stem)?;
            if change(&mut env.vars) {
                folder
                    .write_environment(&env, Some(stem))
                    .map_err(|e| e.to_string())?;
            }
        }
        None => {
            let mut shared = folder.shared().map_err(|e| e.to_string())?;
            if change(&mut shared.vars) {
                folder.write_shared(&shared).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// Declare `names` as kept in an environment's file, or `shared.json`.
fn declare_kept(folder: &Folder, environment: Option<&str>, names: &[&str]) -> Result<()> {
    change_vars(folder, environment, |vars| {
        let mut changed = false;
        for name in names {
            changed |= keep::declare(vars, name);
        }
        changed
    })
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// What a send did to the call's kept values: the values to the keychain,
/// where they came from to the log, and their names to the file.
fn record_kept<R: Runtime>(
    app: &AppHandle<R>,
    repo: &Path,
    environment: Option<&str>,
    call: &Call,
    applied: &[keep::Applied],
) -> Result<()> {
    if applied.is_empty() {
        return Ok(());
    }
    let found: Vec<(String, String)> = applied
        .iter()
        .filter_map(|done| done.value.clone().map(|value| (done.var.clone(), value)))
        .collect();
    vault::set_many(&Keychain, &repo_key(repo), environment, &found).map_err(|e| e.to_string())?;

    let dir = data(app, repo)?;
    let mut log = KeptLog::read(&kept_log(&dir));
    let env = environment.unwrap_or(vault::SHARED);
    let now = now_millis();
    for done in applied {
        if done.value.is_some() {
            log.set(
                env,
                &done.var,
                Kept {
                    at: now,
                    call_id: call.id.clone(),
                    call: call.name.clone(),
                    method: call.method().to_string(),
                    from: done.from.clone(),
                    stale: false,
                },
            );
        } else {
            log.stale(env, &done.var);
        }
    }
    log.write(&kept_log(&dir)).map_err(|e| e.to_string())?;

    let (folder, _, _) = place(app, repo)?;
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    declare_kept(&folder, environment, &names)
}

#[tauri::command(async)]
pub fn calls_environments<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Environments> {
    let repo = state
        .open_path("calls_environments")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    Ok(environments_of(&folder))
}

/// Save an environment. `was` is its stem before a rename, and its values in
/// the keychain move with it; `copyFrom` is the environment it duplicates,
/// whose values it is given.
#[tauri::command(async)]
pub fn calls_save_environment<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    environment: Environment,
    was: Option<String>,
    copy_from: Option<String>,
) -> Result<Environments> {
    let repo = state
        .open_path("calls_save_environment")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    folder
        .write_environment(&environment, was.as_deref())
        .map_err(|e| e.to_string())?;
    let key = repo_key(&repo);
    let dir = data(&app, &repo)?;
    if let Some(old) = was.as_deref().filter(|old| *old != environment.name) {
        vault::rename(&Keychain, &key, old, &environment.name).map_err(|e| e.to_string())?;
        let mut log = KeptLog::read(&kept_log(&dir));
        log.rename_environment(old, &environment.name);
        log.write(&kept_log(&dir)).map_err(|e| e.to_string())?;
    }
    if let Some(from) = copy_from.as_deref() {
        vault::copy(&Keychain, &key, from, &environment.name).map_err(|e| e.to_string())?;
    }
    Ok(environments_of(&folder))
}

/// Delete an environment's file, and its values from the keychain.
#[tauri::command(async)]
pub fn calls_delete_environment<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    stem: String,
) -> Result<Environments> {
    let repo = state
        .open_path("calls_delete_environment")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    folder
        .delete_environment(&stem)
        .map_err(|e| e.to_string())?;
    vault::drop_environment(&Keychain, &repo_key(&repo), &stem).map_err(|e| e.to_string())?;
    let dir = data(&app, &repo)?;
    let mut log = KeptLog::read(&kept_log(&dir));
    log.drop_environment(&stem);
    log.write(&kept_log(&dir)).map_err(|e| e.to_string())?;
    Ok(environments_of(&folder))
}

#[tauri::command(async)]
pub fn calls_save_shared<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    shared: Shared,
) -> Result<Environments> {
    let repo = state
        .open_path("calls_save_shared")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    folder.write_shared(&shared).map_err(|e| e.to_string())?;
    Ok(environments_of(&folder))
}

/// A kept value as the screen shows it: its value, in the clear, and where
/// it came from.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptView {
    pub name: String,
    pub value: Option<String>,
    #[serde(flatten)]
    pub kept: Kept,
}

/// An environment's private side. A secret's value is never in it, only
/// whether it has one; *Reveal* asks for one value at a time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Private {
    pub kept: Vec<KeptView>,
    /// The secrets that have a value in the keychain.
    pub secrets: Vec<String>,
}

#[tauri::command(async)]
pub fn calls_private<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    environment: Option<String>,
) -> Result<Private> {
    let repo = state
        .open_path("calls_private")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    let vars = vars_of(&folder, environment.as_deref())?;
    // An environment with nothing private never asks the keychain, so a
    // machine with none still sends plain calls.
    if !vars.iter().any(Variable::private) {
        return Ok(Private {
            kept: Vec::new(),
            secrets: Vec::new(),
        });
    }
    let values = vault::values(&Keychain, &repo_key(&repo), environment.as_deref())
        .map_err(|e| e.to_string())?;
    let log = KeptLog::read(&kept_log(&data(&app, &repo)?))
        .environment(environment.as_deref().unwrap_or(vault::SHARED));
    Ok(Private {
        kept: vars
            .iter()
            .filter(|var| var.kept)
            .map(|var| KeptView {
                name: var.name.clone(),
                value: values.get(&var.name).cloned(),
                kept: log.get(&var.name).cloned().unwrap_or_default(),
            })
            .collect(),
        secrets: vars
            .iter()
            .filter(|var| var.secret && values.contains_key(&var.name))
            .map(|var| var.name.clone())
            .collect(),
    })
}

/// One secret's value, while the eye is held on Environments.
#[tauri::command(async)]
pub fn calls_reveal(
    state: State<'_, AppState>,
    environment: Option<String>,
    name: String,
) -> Result<Option<String>> {
    let repo = state.open_path("calls_reveal").map_err(|e| e.to_string())?;
    Ok(
        vault::values(&Keychain, &repo_key(&repo), environment.as_deref())
            .map_err(|e| e.to_string())?
            .get(&name)
            .cloned(),
    )
}

/// Set a secret's value, or remove it with `None`.
#[tauri::command(async)]
pub fn calls_set_private(
    state: State<'_, AppState>,
    environment: Option<String>,
    name: String,
    value: Option<String>,
) -> Result<()> {
    let repo = state
        .open_path("calls_set_private")
        .map_err(|e| e.to_string())?;
    vault::set(
        &Keychain,
        &repo_key(&repo),
        environment.as_deref(),
        &name,
        value.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// A value the person clicked in a response, to keep.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepAsk {
    pub environment: Option<String>,
    pub var: String,
    pub value: String,
    pub call_id: String,
    pub call: String,
    pub method: String,
    pub from: String,
}

/// Keep a value: into the keychain, its name into the environment's file,
/// and where it came from into the log. The rule itself is on the call,
/// which the screen saves.
#[tauri::command(async)]
pub fn calls_keep<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    ask: KeepAsk,
) -> Result<Environments> {
    let repo = state.open_path("calls_keep").map_err(|e| e.to_string())?;
    let var = ask.var.trim();
    if var.is_empty() || var.contains(['{', '}']) || var.chars().any(char::is_whitespace) {
        return Err(format!("{var:?} cannot be a variable's name"));
    }
    let (folder, _, _) = place(&app, &repo)?;
    vault::set(
        &Keychain,
        &repo_key(&repo),
        ask.environment.as_deref(),
        var,
        Some(&ask.value),
    )
    .map_err(|e| e.to_string())?;
    let dir = data(&app, &repo)?;
    let mut log = KeptLog::read(&kept_log(&dir));
    log.set(
        ask.environment.as_deref().unwrap_or(vault::SHARED),
        var,
        Kept {
            at: now_millis(),
            call_id: ask.call_id.clone(),
            call: ask.call.clone(),
            method: ask.method.clone(),
            from: ask.from.clone(),
            stale: false,
        },
    );
    log.write(&kept_log(&dir)).map_err(|e| e.to_string())?;
    declare_kept(&folder, ask.environment.as_deref(), &[var])?;
    Ok(environments_of(&folder))
}

/// Forget a kept value: from the keychain, the log and the file.
#[tauri::command(async)]
pub fn calls_forget_kept<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    environment: Option<String>,
    name: String,
) -> Result<Environments> {
    let repo = state
        .open_path("calls_forget_kept")
        .map_err(|e| e.to_string())?;
    let (folder, _, _) = place(&app, &repo)?;
    vault::set(
        &Keychain,
        &repo_key(&repo),
        environment.as_deref(),
        &name,
        None,
    )
    .map_err(|e| e.to_string())?;
    let dir = data(&app, &repo)?;
    let mut log = KeptLog::read(&kept_log(&dir));
    log.forget(environment.as_deref().unwrap_or(vault::SHARED), &name);
    log.write(&kept_log(&dir)).map_err(|e| e.to_string())?;
    change_vars(&folder, environment.as_deref(), |vars| {
        let before = vars.len();
        vars.retain(|var| !(var.kept && var.name == name));
        vars.len() != before
    })?;
    Ok(environments_of(&folder))
}

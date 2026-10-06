// SPDX-License-Identifier: GPL-3.0-or-later

//! The CodeRabbit extension's worker (FEAT-097).
//!
//! It never starts a process itself. Everything CodeRabbit does, it asks
//! Spagitty to do through the profiles in `extension.json` — so the CLI runs
//! in the directory Spagitty approved, inside a process tree Spagitty can end,
//! with argv Spagitty built, and no code path here can add `--use-credits` or
//! `--api-key`, because no profile has them.

mod adapter;
mod connection;
mod rpc;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::{json, Value};

use adapter::{Adapter, Event, Exit};
use connection::State;
use rpc::{HostError, Rpc, ToolMessage, CANCELLED, METHOD_NOT_FOUND, NOT_GRANTED};

const TOOL: &str = "coderabbit";
/// Findings are sent in batches of at most this many.
const BATCH: usize = 20;
/// Every command and provider that needs the CLI.
const NEEDS_CLI: [&str; 5] = [
    "reviewChanges",
    "reviewTask",
    "checkSetup",
    "signIn",
    "runDiagnostics",
];

fn main() {
    let rpc = Rpc::new(Box::new(std::io::stdout()));
    let stdin = std::io::stdin();
    rpc.serve(stdin.lock(), handle);
}

fn refuse(code: i64, message: impl Into<String>) -> HostError {
    HostError {
        code,
        message: message.into(),
        data: None,
    }
}

fn handle(rpc: Arc<Rpc>, method: String, params: Value) -> Result<Value, HostError> {
    match method.as_str() {
        "extension.initialize" => {
            rpc.set_settings(params.get("settings").cloned().unwrap_or_else(|| json!({})));
            let granted = params
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(|c| c.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            rpc.set_capabilities(granted);
            Ok(json!({"apiVersion": "1.0.0"}))
        }
        "extension.activate" => Ok(activate(&rpc)),
        "command.execute" => {
            let operation = text(&params, "operationId");
            let command = text(&params, "command");
            if !matches!(command.as_str(), "checkSetup" | "signIn" | "runDiagnostics") {
                return Err(refuse(
                    METHOD_NOT_FOUND,
                    format!("CodeRabbit has no command called {command}"),
                ));
            }
            std::thread::spawn(move || {
                let result = match command.as_str() {
                    "checkSetup" => check_setup(&rpc, &operation),
                    "signIn" => sign_in(&rpc, &operation),
                    _ => diagnostics(&rpc, &operation),
                };
                let (status, message) = match result {
                    Ok(message) => ("completed", message),
                    Err(message) if rpc.cancel_flag(&operation).load(Ordering::Acquire) => {
                        ("cancelled", message)
                    }
                    Err(message) => ("failed", message),
                };
                rpc.notify(
                    "operation.complete",
                    json!({"operationId": operation, "status": status, "message": message}),
                );
                rpc.forget(&operation);
            });
            Ok(json!({"accepted": true}))
        }
        "review.start" => {
            if text(&params, "provider") != "review" {
                return Err(refuse(
                    METHOD_NOT_FOUND,
                    "CodeRabbit has one review provider, review",
                ));
            }
            std::thread::spawn(move || review(&rpc, &params));
            Ok(json!({"accepted": true}))
        }
        "panel.resolve" => match text(&params, "panel").as_str() {
            "connection" => Ok(connection_panel(&rpc)),
            other => Err(refuse(
                METHOD_NOT_FOUND,
                format!("CodeRabbit has no panel called {other}"),
            )),
        },
        other => Err(refuse(
            METHOD_NOT_FOUND,
            format!("CodeRabbit does not handle {other}"),
        )),
    }
}

fn text(params: &Value, key: &str) -> String {
    params
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// The tool's state, read locally: found, recent enough, allowed to run.
fn tool_state(rpc: &Rpc) -> (Option<State>, Option<String>) {
    if !rpc.granted("tools.execute") {
        return (Some(State::DeniedCapability), None);
    }
    match rpc.call("tools.detect", json!({"tool": TOOL})) {
        Ok(detected) => (
            connection::from_detection(&detected),
            detected
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
        Err(error) if error.code == NOT_GRANTED => (Some(State::DeniedCapability), None),
        Err(error) => (Some(State::FailedDiagnostics(error.message)), None),
    }
}

fn activate(rpc: &Rpc) -> Value {
    let (state, _) = tool_state(rpc);
    match state {
        Some(state) => json!({
            "unavailable": NEEDS_CLI.iter().map(|id| json!({"id": id, "reason": state.sentence()})).collect::<Vec<_>>()
        }),
        None => json!({}),
    }
}

/// Run a profile that ends quickly and collect its output.
fn run_profile(
    rpc: &Arc<Rpc>,
    operation: &str,
    profile: &str,
    options: Value,
) -> Result<(Vec<String>, Value), HostError> {
    let receiver = rpc.run_tool(
        operation,
        json!({"operationId": operation, "tool": TOOL, "profile": profile, "options": options}),
    );
    let mut lines = Vec::new();
    while let Ok(message) = receiver.recv() {
        match message {
            ToolMessage::Line(line) => lines.push(line),
            ToolMessage::Done(result) => return result.map(|done| (lines, done)),
        }
    }
    Err(refuse(
        rpc::INTERNAL,
        "the tool run ended without an answer",
    ))
}

fn sign_in_state(rpc: &Arc<Rpc>, operation: &str) -> Result<(State, connection::Auth), String> {
    let (lines, done) =
        run_profile(rpc, operation, "authStatus", json!({})).map_err(|e| e.message)?;
    let auth = connection::parse_auth(&lines);
    let state = connection::from_auth(&auth, done.get("exitCode").and_then(Value::as_i64));
    let _ = rpc.call(
        "storage.set",
        json!({"key": "auth", "value": {
            "authenticated": auth.authenticated,
            "region": auth.region,
            "account": auth.account,
        }}),
    );
    Ok((state, auth))
}

fn check_setup(rpc: &Arc<Rpc>, operation: &str) -> Result<String, String> {
    let (state, version) = tool_state(rpc);
    if let Some(state) = state {
        return Err(state.sentence());
    }
    let (state, auth) = sign_in_state(rpc, operation)?;
    let mut message = format!(
        "CodeRabbit CLI {}: {}",
        version.unwrap_or_default(),
        state.sentence()
    );
    if let Some(account) = auth.account {
        message.push_str(&format!(" Signed in as {account}."));
    }
    let _ = rpc.call(
        "ui.notify",
        json!({"level": if state == State::Ready { "info" } else { "warn" }, "message": message}),
    );
    if state == State::Ready {
        Ok(message)
    } else {
        Err(message)
    }
}

fn sign_in(rpc: &Arc<Rpc>, operation: &str) -> Result<String, String> {
    let (state, _) = tool_state(rpc);
    if let Some(state) = state {
        return Err(state.sentence());
    }
    let region = rpc
        .settings()
        .get("region")
        .and_then(Value::as_str)
        .filter(|r| matches!(*r, "us" | "eu"))
        .unwrap_or("us")
        .to_string();
    rpc.notify(
        "operation.progress",
        json!({"operationId": operation, "message": "Finish signing in in your browser"}),
    );
    let (lines, done) = run_profile(rpc, operation, "authLogin", json!({"region": region}))
        .map_err(|e| e.message)?;
    for line in lines.iter().take(50) {
        rpc.notify("log", json!({"level": "info", "message": line}));
    }
    if done.get("exitCode").and_then(Value::as_i64) != Some(0) {
        return Err("Signing in did not finish. See Diagnostics for what the CLI said.".into());
    }
    let (state, auth) = sign_in_state(rpc, operation)?;
    match state {
        State::Ready => Ok(match auth.account {
            Some(account) => format!("Signed in to CodeRabbit as {account}."),
            None => "Signed in to CodeRabbit.".into(),
        }),
        other => Err(other.sentence()),
    }
}

fn diagnostics(rpc: &Arc<Rpc>, operation: &str) -> Result<String, String> {
    let (state, _) = tool_state(rpc);
    if let Some(state @ (State::MissingTool(_) | State::DeniedCapability)) = state {
        return Err(state.sentence());
    }
    rpc.notify(
        "operation.progress",
        json!({"operationId": operation, "message": "Running CodeRabbit's diagnostics"}),
    );
    let (lines, done) = run_profile(rpc, operation, "doctor", json!({})).map_err(|e| e.message)?;
    // Through the host's log, which redacts and bounds what it keeps.
    for line in lines.iter().take(150) {
        rpc.notify(
            "log",
            json!({"level": "info", "message": format!("doctor: {line}")}),
        );
    }
    let passed = done.get("exitCode").and_then(Value::as_i64) == Some(0);
    let _ = rpc.call(
        "storage.set",
        json!({"key": "doctor", "value": {"passed": passed}}),
    );
    if passed {
        Ok("CodeRabbit's diagnostics passed.".into())
    } else {
        Err("CodeRabbit's diagnostics found a problem. Its report is in Settings › Extensions › CodeRabbit › Diagnostics.".into())
    }
}

fn connection_panel(rpc: &Rpc) -> Value {
    let (state, version) = tool_state(rpc);
    let region = rpc
        .settings()
        .get("region")
        .and_then(Value::as_str)
        .unwrap_or("us")
        .to_string();
    let stored = rpc
        .call("storage.get", json!({"key": "auth"}))
        .ok()
        .and_then(|v| v.get("value").cloned());
    let auth = stored.filter(|v| v.is_object()).map(|v| connection::Auth {
        authenticated: v.get("authenticated").and_then(Value::as_bool),
        region: v.get("region").and_then(Value::as_str).map(str::to_string),
        account: v.get("account").and_then(Value::as_str).map(str::to_string),
        problem: None,
    });
    let state = state.unwrap_or_else(|| match auth.as_ref().and_then(|a| a.authenticated) {
        Some(false) => State::SignedOut,
        _ => State::Ready,
    });
    connection::panel(&state, version.as_deref(), auth.as_ref(), &region)
}

/// One review, start to finish. Always ends with exactly one completion.
fn review(rpc: &Arc<Rpc>, params: &Value) {
    let operation = text(params, "operationId");
    let workdir = text(params, "workdir");
    let snapshot = params.get("snapshot").cloned().unwrap_or(Value::Null);
    let cancel = rpc.cancel_flag(&operation);
    let finish = |status: &str, outcome: Value| {
        let summary = outcome
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        rpc.notify(
            "operation.complete",
            json!({"operationId": operation, "status": status, "message": summary, "review": outcome}),
        );
        rpc.forget(&operation);
    };
    let stopped = |summary: &str, version: &str| json!({"status": "failed", "completeness": "unknown", "summary": summary, "providerVersion": version});
    let progress = |message: &str| {
        rpc.notify(
            "operation.progress",
            json!({"operationId": operation, "message": message}),
        )
    };

    progress("Checking the CodeRabbit CLI");
    let (state, version) = tool_state(rpc);
    let version = version.unwrap_or_else(|| "unknown".into());
    if let Some(state) = state {
        return finish("failed", stopped(&state.sentence(), &version));
    }

    progress("Checking sign-in");
    match sign_in_state(rpc, &operation) {
        Ok((State::Ready, _)) => {}
        Ok((State::SignedOut, _)) => {
            let message = "Sign in to CodeRabbit to review. Use “Sign in to CodeRabbit” in the command palette.";
            return finish(
                "completed",
                json!({"status": "actionRequired", "completeness": "unknown", "summary": message, "providerVersion": version,
                       "actionRequired": {"kind": "signIn", "message": message}}),
            );
        }
        Ok((other, _)) => return finish("failed", stopped(&other.sentence(), &version)),
        Err(message) => return finish("failed", stopped(&message, &version)),
    }
    if cancel.load(Ordering::Acquire) {
        return finish(
            "cancelled",
            json!({"status": "cancelled", "completeness": "unknown", "summary": "Cancelled.", "providerVersion": version}),
        );
    }

    let scope = snapshot
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("uncommitted");
    let mut options = json!({"scope": scope});
    if matches!(scope, "committed" | "tracked") {
        if let Some(base) = snapshot.get("baseCommit").and_then(Value::as_str) {
            options["baseCommit"] = json!(base);
        }
    }
    progress("Sending the changes to CodeRabbit");
    let receiver = rpc.run_tool(
        &operation,
        json!({"operationId": operation, "tool": TOOL, "profile": "review", "options": options, "workdir": workdir}),
    );

    let mut adapter = Adapter::new();
    let mut batch: Vec<Value> = Vec::new();
    let flush = |batch: &mut Vec<Value>| {
        if !batch.is_empty() && !cancel.load(Ordering::Acquire) {
            rpc.notify(
                "review.findings",
                json!({"operationId": operation, "findings": batch.clone()}),
            );
        }
        batch.clear();
    };
    let mut result = None;
    while let Ok(message) = receiver.recv() {
        match message {
            ToolMessage::Line(line) => {
                for event in adapter.feed(&line) {
                    match event {
                        Event::Progress(words) => progress(&words),
                        Event::Heartbeat => {
                            rpc.notify("operation.progress", json!({"operationId": operation}))
                        }
                        Event::Finding(finding) => {
                            batch.push(finding);
                            if batch.len() >= BATCH {
                                flush(&mut batch);
                            }
                        }
                        Event::Unknown(what) => rpc.notify(
                            "log",
                            json!({"level": "debug", "message": format!("CodeRabbit sent {what}")}),
                        ),
                    }
                }
                // Findings reach the screen as they arrive, not at the end.
                flush(&mut batch);
            }
            ToolMessage::Done(done) => {
                result = Some(done);
                break;
            }
        }
    }
    flush(&mut batch);

    let exit = match result {
        Some(Ok(done)) => Exit {
            code: done.get("exitCode").and_then(Value::as_i64),
            cancelled: cancel.load(Ordering::Acquire),
            timed_out: done
                .get("timedOut")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        Some(Err(error)) if error.code == CANCELLED || cancel.load(Ordering::Acquire) => Exit {
            code: None,
            cancelled: true,
            timed_out: false,
        },
        Some(Err(error)) => return finish("failed", stopped(&error.message, &version)),
        None => {
            return finish(
                "failed",
                stopped(
                    "The review ended without an answer from Spagitty.",
                    &version,
                ),
            )
        }
    };
    let outcome = adapter.finish(exit, &version);
    let status = match outcome.get("status").and_then(Value::as_str) {
        Some("cancelled") => "cancelled",
        Some("failed") => "failed",
        _ => "completed",
    };
    finish(status, outcome);
}

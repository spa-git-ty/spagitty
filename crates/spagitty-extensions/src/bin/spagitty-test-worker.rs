// SPDX-License-Identifier: GPL-3.0-or-later

//! A scripted extension worker for the host's own tests.
//!
//! It speaks the protocol from `schemas/extensions/protocol.v1.md` with only
//! `serde_json`, the way any third-party worker would, and misbehaves on
//! request. Its behaviour is read from a file called `mode` in its working
//! directory — the package directory the host starts it in — so tests running
//! in parallel cannot change each other's worker.
//!
//! Run with `--sleeper` it is not a worker at all but an "external tool": it
//! writes `sleeper-started` in its working directory, waits, and writes
//! `sleeper-survived` if nothing ended it first.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

type Waiters = Arc<Mutex<HashMap<String, mpsc::Sender<Value>>>>;

#[derive(Clone)]
struct Out {
    stdout: Arc<Mutex<std::io::Stdout>>,
    waiters: Waiters,
    next: Arc<AtomicU64>,
    cancelled: Arc<Mutex<HashMap<String, bool>>>,
}

impl Out {
    fn send(&self, message: Value) {
        let mut out = self.stdout.lock().unwrap();
        let _ = writeln!(out, "{message}");
        let _ = out.flush();
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn respond(&self, id: &Value, result: Value) {
        self.send(json!({"jsonrpc": "2.0", "id": id, "result": result}));
    }

    /// Ask the host something and wait for the whole answer message.
    fn call(&self, method: &str, params: Value) -> Value {
        let id = format!("w{}", self.next.fetch_add(1, Ordering::Relaxed));
        let (send, receive) = mpsc::channel();
        self.waiters.lock().unwrap().insert(id.clone(), send);
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        receive
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or(json!({"error": {"code": 0, "message": "no answer"}}))
    }

    fn is_cancelled(&self, operation: &str) -> bool {
        self.cancelled
            .lock()
            .unwrap()
            .get(operation)
            .copied()
            .unwrap_or(false)
    }

    fn complete(&self, operation: &str, status: &str, message: &str, review: Option<Value>) {
        let mut params = json!({"operationId": operation, "status": status, "message": message});
        if let Some(review) = review {
            params["review"] = review;
        }
        self.notify("operation.complete", params);
    }
}

fn sleeper() {
    let _ = std::fs::write("sleeper-started", "started");
    std::thread::sleep(Duration::from_secs(4));
    let _ = std::fs::write("sleeper-survived", "survived");
}

fn main() {
    if std::env::args().any(|a| a == "--sleeper") {
        return sleeper();
    }
    if std::env::args().any(|a| a == "--version") {
        println!("test-tool 1.2.3");
        return;
    }
    if std::env::args().any(|a| a == "--lines") {
        for i in 0..5 {
            println!("{{\"line\":{i}}}");
        }
        return;
    }

    let mode = std::fs::read_to_string("mode").unwrap_or_else(|_| "normal".into());
    let mode = mode.trim().to_string();
    eprintln!("test worker starting in mode {mode}; token ghp_0123456789abcdefghijklmnop must be redacted");

    if mode == "silent" {
        // Never answers anything: the handshake deadline must end it.
        std::thread::sleep(Duration::from_secs(60));
        return;
    }

    let out = Out {
        stdout: Arc::new(Mutex::new(std::io::stdout())),
        waiters: Arc::new(Mutex::new(HashMap::new())),
        next: Arc::new(AtomicU64::new(1)),
        cancelled: Arc::new(Mutex::new(HashMap::new())),
    };
    let ignore_cancel = Arc::new(AtomicBool::new(mode == "ignoreCancel"));

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string);
        let id = message.get("id").cloned();

        let Some(method) = method else {
            // A response to one of our requests.
            if let Some(id) = id.as_ref().and_then(Value::as_str) {
                if let Some(waiter) = out.waiters.lock().unwrap().remove(id) {
                    let _ = waiter.send(message.clone());
                }
            }
            continue;
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        match method.as_str() {
            "extension.initialize" => {
                let api = if mode == "badHandshake" {
                    "2.0.0"
                } else {
                    "1.0.0"
                };
                out.respond(id.as_ref().unwrap(), json!({"apiVersion": api}));
                if mode == "garbage" {
                    let mut stdout = out.stdout.lock().unwrap();
                    let _ = writeln!(stdout, "this is not a protocol message");
                    let _ = stdout.flush();
                }
            }
            "extension.activate" => {
                out.respond(
                    id.as_ref().unwrap(),
                    json!({"unavailable": [{"id": "broken", "reason": "the tool is missing"}]}),
                );
            }
            "extension.deactivate" => {
                out.respond(id.as_ref().unwrap(), json!({}));
                return;
            }
            "settings.changed" => {
                let _ = std::fs::write("settings-seen.json", params.to_string());
            }
            "operation.cancel" => {
                let operation = params["operationId"].as_str().unwrap_or("").to_string();
                out.cancelled.lock().unwrap().insert(operation, true);
            }
            "tools.output" => {
                let line = params["line"].as_str().unwrap_or("").to_string();
                let mut seen = std::fs::read_to_string("tool-lines").unwrap_or_default();
                seen.push_str(&line);
                seen.push('\n');
                let _ = std::fs::write("tool-lines", seen);
            }
            "panel.resolve" => {
                out.respond(
                    id.as_ref().unwrap(),
                    json!({"title": "Hello", "rows": [{"label": "Panel", "value": params["panel"]}], "text": "**hi**"}),
                );
            }
            "command.execute" | "review.start" => {
                out.respond(id.as_ref().unwrap(), json!({"accepted": true}));
                let out = out.clone();
                let ignore_cancel = ignore_cancel.clone();
                let mode = mode.clone();
                std::thread::spawn(move || {
                    operation(&out, &method, &params, &mode, &ignore_cancel)
                });
            }
            _ => {
                if let Some(id) = id {
                    out.send(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "unknown"}}));
                }
            }
        }
    }
}

fn error_code(answer: &Value) -> Option<i64> {
    answer
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Value::as_i64)
}

fn operation(out: &Out, method: &str, params: &Value, mode: &str, ignore_cancel: &AtomicBool) {
    let op = params["operationId"].as_str().unwrap_or("").to_string();
    let context = &params["context"];
    let repository = context["repository"]
        .as_str()
        .or(params["repository"].as_str())
        .map(str::to_string);

    if method == "review.start" {
        let provider = params["provider"].as_str().unwrap_or("");
        out.notify(
            "operation.progress",
            json!({"operationId": op, "message": "Reviewing"}),
        );
        if provider == "slowReview" {
            std::thread::sleep(Duration::from_millis(1500));
        }
        let snapshot = out.call("review.snapshot", json!({"operationId": op}));
        let scope = snapshot["result"]["scope"]
            .as_str()
            .unwrap_or("?")
            .to_string();
        out.notify(
            "review.findings",
            json!({"operationId": op, "findings": [
                {"id": "f1", "severity": "high", "providerSeverity": "major", "path": "core.txt",
                 "title": "A bug", "message": "Something is wrong", "disposition": "dismissed"},
                {"id": "f1", "severity": "low", "title": "Duplicate", "message": "repeated id"},
                {"id": "f2", "severity": "info", "path": "../escape.txt", "title": "Bad path", "message": "outside"},
                {"id": "f3", "severity": "unknown", "title": "Whole review", "message": "no location",
                 "sourceUrl": "javascript:alert(1)"}
            ]}),
        );
        let review = match provider {
            "partial" => {
                json!({"status": "completed", "completeness": "partial", "summary": "Some files were not reviewed", "providerVersion": "9.9.9"})
            }
            "billing" => {
                json!({"status": "actionRequired", "completeness": "unknown", "summary": "Payment needed",
                                "providerVersion": "9.9.9", "actionRequired": {"kind": "billing", "message": "Confirm usage credits"}})
            }
            _ => {
                json!({"status": "completed", "completeness": "complete", "summary": format!("Reviewed scope {scope}"), "providerVersion": "9.9.9"})
            }
        };
        out.complete(&op, "completed", "Review done", Some(review));
        return;
    }

    let command = params["command"].as_str().unwrap_or("");
    match command {
        "hello" => {
            out.call(
                "ui.notify",
                json!({"level": "info", "message": "Hello from the test worker"}),
            );
            let mut name = String::from("nowhere");
            if let Some(repository) = &repository {
                let described = out.call("repository.describe", json!({"repository": repository}));
                if let Some(branch) = described["result"]["branch"].as_str() {
                    name = branch.to_string();
                } else if let Some(code) = error_code(&described) {
                    name = format!("error {code}");
                }
            }
            out.complete(&op, "completed", &format!("Hello {name}"), None);
        }
        "denied" => {
            let answer = out.call(
                "forge.pullRequest.snapshot",
                json!({"repository": repository, "number": 1}),
            );
            let forged = out.call("repository.describe", json!({"repository": "repo:999999"}));
            out.complete(
                &op,
                "completed",
                &format!(
                    "codes {} {}",
                    error_code(&answer).unwrap_or(0),
                    error_code(&forged).unwrap_or(0)
                ),
                None,
            );
        }
        "concurrent" => {
            let handles: Vec<_> = (0..6)
                .map(|i| {
                    let out = out.clone();
                    let op = op.clone();
                    std::thread::spawn(move || {
                        out.notify(
                            "operation.progress",
                            json!({"operationId": op, "message": format!("step {i}")}),
                        );
                        out.call("storage.set", json!({"key": format!("k{i}"), "value": i}))
                    })
                })
                .collect();
            let ok = handles
                .into_iter()
                .filter_map(|h| h.join().ok())
                .filter(|a| a.get("result").is_some())
                .count();
            let read = out.call("storage.get", json!({"key": "k3"}));
            out.complete(
                &op,
                "completed",
                &format!("{ok} stored, k3={}", read["result"]["value"]),
                None,
            );
        }
        "twice" => {
            out.complete(&op, "completed", "first", None);
            out.complete(&op, "failed", "second", None);
            out.notify(
                "operation.progress",
                json!({"operationId": op, "message": "late"}),
            );
        }
        "slow" => {
            for _ in 0..600 {
                if out.is_cancelled(&op) && !ignore_cancel.load(Ordering::Relaxed) {
                    out.complete(&op, "cancelled", "stopped as asked", None);
                    return;
                }
                out.notify("operation.progress", json!({"operationId": op}));
                std::thread::sleep(Duration::from_millis(50));
            }
            out.complete(&op, "completed", "slow finished", None);
        }
        "quiet" => {
            // Says nothing at all: the inactivity limit must end it.
            std::thread::sleep(Duration::from_secs(30));
        }
        "crash" => {
            out.notify(
                "operation.progress",
                json!({"operationId": op, "message": "about to crash"}),
            );
            std::thread::sleep(Duration::from_millis(100));
            std::process::exit(3);
        }
        "tool" | "sleepTool" => {
            let profile = if command == "tool" { "lines" } else { "sleep" };
            let answer = out.call(
                "tools.run",
                json!({"operationId": op, "tool": "self", "profile": profile, "workdir": repository}),
            );
            let message = match answer.get("result") {
                Some(result) => format!("exit {}", result["exitCode"]),
                None => format!("error {}", error_code(&answer).unwrap_or(0)),
            };
            out.complete(&op, "completed", &message, None);
        }
        "badTool" => {
            let answer = out.call(
                "tools.run",
                json!({"operationId": op, "tool": "self", "profile": "lines", "options": {"x": "--evil"}, "workdir": repository}),
            );
            out.complete(
                &op,
                "completed",
                &format!("error {}", error_code(&answer).unwrap_or(0)),
                None,
            );
        }
        _ => out.complete(&op, "failed", "unknown command", None),
    }
    let _ = mode;
}

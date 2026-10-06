// SPDX-License-Identifier: GPL-3.0-or-later

//! The protocol's line framing, from the worker's side.
//!
//! Written against `schemas/extensions/protocol.v1.md` alone. The reader runs
//! on the main thread and never blocks on a handler: requests are answered on
//! threads of their own, responses are handed to whoever is waiting, and
//! `tools.output` lines are routed to the operation that asked for the tool.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

/// An error the host answered a callback with.
#[derive(Debug, Clone, PartialEq)]
pub struct HostError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INTERNAL: i64 = -32603;
pub const NOT_GRANTED: i64 = -32001;
pub const CANCELLED: i64 = -32008;

/// What a tool run streams back to the operation that started it.
#[derive(Debug)]
pub enum ToolMessage {
    Line(String),
    Done(Result<Value, HostError>),
}

type Waiter = mpsc::Sender<Result<Value, HostError>>;

pub struct Rpc {
    out: Mutex<Box<dyn Write + Send>>,
    pending: Mutex<HashMap<String, Waiter>>,
    next: AtomicU64,
    /// Operation id → where its tool output goes.
    tools: Mutex<HashMap<String, mpsc::Sender<ToolMessage>>>,
    /// Operation id → its cancellation flag.
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    settings: Mutex<Value>,
    capabilities: Mutex<Vec<String>>,
}

impl Rpc {
    pub fn new(out: Box<dyn Write + Send>) -> Arc<Rpc> {
        Arc::new(Rpc {
            out: Mutex::new(out),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            tools: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            settings: Mutex::new(json!({})),
            capabilities: Mutex::new(Vec::new()),
        })
    }

    fn send(&self, message: Value) {
        let mut out = self.out.lock().expect("stdout");
        let _ = writeln!(out, "{message}");
        let _ = out.flush();
    }

    pub fn notify(&self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    pub fn respond(&self, id: &Value, result: Result<Value, HostError>) {
        match result {
            Ok(value) => self.send(json!({"jsonrpc": "2.0", "id": id, "result": value})),
            Err(error) => {
                let mut body = json!({"code": error.code, "message": error.message});
                if let Some(data) = error.data {
                    body["data"] = data;
                }
                self.send(json!({"jsonrpc": "2.0", "id": id, "error": body}))
            }
        }
    }

    /// Ask the host something and wait for its answer.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, HostError> {
        let id = format!("w{}", self.next.fetch_add(1, Ordering::Relaxed));
        let (send, receive) = mpsc::channel();
        self.pending
            .lock()
            .expect("pending")
            .insert(id.clone(), send);
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        // A review can run for an hour; the host's own limits end it sooner.
        receive
            .recv_timeout(Duration::from_secs(2 * 60 * 60))
            .unwrap_or_else(|_| {
                Err(HostError {
                    code: INTERNAL,
                    message: "the host did not answer".into(),
                    data: None,
                })
            })
    }

    /// Run a tool and receive its lines on the returned channel, then its
    /// result as the last message.
    pub fn run_tool(
        self: &Arc<Self>,
        operation: &str,
        params: Value,
    ) -> mpsc::Receiver<ToolMessage> {
        let (send, receive) = mpsc::channel();
        self.tools
            .lock()
            .expect("tools")
            .insert(operation.to_string(), send.clone());
        let rpc = self.clone();
        let operation = operation.to_string();
        std::thread::spawn(move || {
            let result = rpc.call("tools.run", params);
            rpc.tools.lock().expect("tools").remove(&operation);
            let _ = send.send(ToolMessage::Done(result));
        });
        receive
    }

    pub fn cancel_flag(&self, operation: &str) -> Arc<AtomicBool> {
        self.cancels
            .lock()
            .expect("cancels")
            .entry(operation.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    pub fn forget(&self, operation: &str) {
        self.cancels.lock().expect("cancels").remove(operation);
    }

    pub fn settings(&self) -> Value {
        self.settings.lock().expect("settings").clone()
    }

    pub fn set_settings(&self, settings: Value) {
        *self.settings.lock().expect("settings") = settings;
    }

    pub fn granted(&self, capability: &str) -> bool {
        self.capabilities
            .lock()
            .expect("capabilities")
            .iter()
            .any(|c| c == capability)
    }

    pub fn set_capabilities(&self, list: Vec<String>) {
        *self.capabilities.lock().expect("capabilities") = list;
    }

    /// Read `input` until it closes, dispatching each message.
    pub fn serve(
        self: &Arc<Self>,
        input: impl BufRead,
        handle: impl Fn(Arc<Rpc>, String, Value) -> Result<Value, HostError> + Send + Sync + 'static,
    ) {
        let handle = Arc::new(handle);
        for line in input.lines() {
            let Ok(line) = line else { break };
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let Ok(message) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let method = message
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string);
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            match (method, message.get("id").cloned()) {
                (Some(method), Some(id)) => {
                    let rpc = self.clone();
                    let handle = handle.clone();
                    if method == "extension.deactivate" {
                        // Answered in order, then the loop ends when stdin closes.
                        for flag in rpc.cancels.lock().expect("cancels").values() {
                            flag.store(true, Ordering::Release);
                        }
                        rpc.respond(&id, Ok(json!({})));
                        continue;
                    }
                    std::thread::spawn(move || {
                        let answer = handle(rpc.clone(), method, params);
                        rpc.respond(&id, answer);
                    });
                }
                (Some(method), None) => self.notification(&method, params),
                (None, Some(id)) => {
                    let key = match &id {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    if let Some(waiter) = self.pending.lock().expect("pending").remove(&key) {
                        let result = match message.get("error") {
                            Some(error) => Err(HostError {
                                code: error
                                    .get("code")
                                    .and_then(Value::as_i64)
                                    .unwrap_or(INTERNAL),
                                message: error
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                data: error.get("data").cloned(),
                            }),
                            None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
                        };
                        let _ = waiter.send(result);
                    }
                }
                (None, None) => {}
            }
        }
        // stdin closed: anything still running is over.
        for flag in self.cancels.lock().expect("cancels").values() {
            flag.store(true, Ordering::Release);
        }
    }

    fn notification(&self, method: &str, params: Value) {
        match method {
            "operation.cancel" => {
                if let Some(op) = params.get("operationId").and_then(Value::as_str) {
                    self.cancel_flag(op).store(true, Ordering::Release);
                }
            }
            "settings.changed" => {
                if let Some(settings) = params.get("settings") {
                    self.set_settings(settings.clone());
                }
            }
            "tools.output" => {
                let operation = params
                    .get("operationId")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let line = params
                    .get("line")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if let Some(sender) = self.tools.lock().expect("tools").get(operation) {
                    let _ = sender.send(ToolMessage::Line(line));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Sink {
        fn lines(&self) -> Vec<Value> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    #[test]
    fn requests_are_answered_and_notifications_routed() {
        let sink = Sink::default();
        let rpc = Rpc::new(Box::new(sink.clone()));
        let input = [
            r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#,
            r#"{"jsonrpc":"2.0","method":"operation.cancel","params":{"operationId":"op-1"}}"#,
            r#"{"jsonrpc":"2.0","method":"settings.changed","params":{"settings":{"region":"eu"}}}"#,
            "not json is skipped",
        ]
        .join("\n");
        rpc.serve(Cursor::new(input), |_, method, _| {
            if method == "ping" {
                Ok(json!({"pong": true}))
            } else {
                Err(HostError {
                    code: METHOD_NOT_FOUND,
                    message: "no".into(),
                    data: None,
                })
            }
        });
        std::thread::sleep(Duration::from_millis(200));
        assert!(rpc.cancel_flag("op-1").load(Ordering::Acquire));
        assert_eq!(rpc.settings()["region"], "eu");
        let lines = sink.lines();
        assert_eq!(lines[0]["id"], 1);
        assert_eq!(lines[0]["result"]["pong"], true);
    }

    #[test]
    fn a_callback_id_starts_with_w() {
        let sink = Sink::default();
        let rpc = Rpc::new(Box::new(sink.clone()));
        let caller = rpc.clone();
        let call = std::thread::spawn(move || caller.call("ui.notify", json!({})));
        std::thread::sleep(Duration::from_millis(100));
        let id = sink.lines()[0]["id"].as_str().unwrap().to_string();
        assert!(id.starts_with('w'));
        rpc.serve(
            Cursor::new(format!(
                r#"{{"jsonrpc":"2.0","id":"{id}","error":{{"code":-32001,"message":"no"}}}}"#
            )),
            |_, _, _| Ok(Value::Null),
        );
        assert_eq!(call.join().unwrap().unwrap_err().code, NOT_GRANTED);
    }
}

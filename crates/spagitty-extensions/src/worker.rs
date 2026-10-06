// SPDX-License-Identifier: GPL-3.0-or-later

//! One running worker: its process tree, its stdin, and the thread reading its
//! stdout.
//!
//! **Nothing here can deadlock a review.** The reader thread never waits on the
//! host: a response is handed to whoever is waiting for it, a notification to
//! the host's handler (which only records it), and a request from the worker
//! is answered on a thread of its own. So a worker can be waiting for
//! `tools.run` to finish while it streams progress, and the host can be
//! waiting for a `command.execute` answer while the worker asks it something.
//!
//! **stdout and stderr stay apart.** stdout is parsed as protocol and nothing
//! else; one line that is not a message stops the worker. stderr is kept as a
//! bounded, redacted tail for the Diagnostics view and never parsed.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use spagitty_process::ProcessTree;

use crate::protocol::{code, FrameError, Id, Message, RpcError, MAX_MESSAGE_BYTES};
use crate::redact::redact;

/// How much stderr is kept.
pub const STDERR_KEPT: usize = 64 * 1024;
/// How many of the worker's own requests may be in flight at once.
pub const MAX_CALLBACKS: usize = 8;

/// What the worker says to the host. Implemented by [`crate::host`].
pub trait Inbound: Send + Sync {
    /// A notification. Called on the reader thread: it must not block.
    fn notification(&self, method: &str, params: Value);
    /// A request. Called on a thread of its own; may take as long as it needs.
    fn request(&self, method: &str, params: Value) -> Result<Value, RpcError>;
    /// The worker has gone: it exited, was stopped, or broke the protocol.
    fn disconnected(&self, reason: String);
}

/// Why a request to the worker did not get an answer.
#[derive(Debug, Clone, PartialEq)]
pub enum CallError {
    /// The worker answered with an error.
    Rpc(RpcError),
    TimedOut,
    Gone,
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Rpc(error) => write!(f, "{}", error.message),
            CallError::TimedOut => f.write_str("the extension did not answer in time"),
            CallError::Gone => f.write_str("the extension is not running"),
        }
    }
}

type Pending = Mutex<HashMap<u64, mpsc::Sender<Result<Value, RpcError>>>>;

pub struct Worker {
    pid: u32,
    tree: Arc<ProcessTree>,
    stdin: Mutex<Option<ChildStdin>>,
    pending: Arc<Pending>,
    next_id: AtomicU64,
    alive: Arc<AtomicBool>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
}

impl std::fmt::Debug for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worker").field("pid", &self.pid).finish()
    }
}

/// Write one message, with the newline, and flush.
fn write_message(stdin: &Mutex<Option<ChildStdin>>, message: &Message) -> std::io::Result<()> {
    let mut line = message.to_line();
    line.push('\n');
    let mut guard = stdin.lock().expect("worker stdin");
    let pipe = guard
        .as_mut()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin closed"))?;
    pipe.write_all(line.as_bytes())?;
    pipe.flush()
}

impl Worker {
    /// Start `program` in `cwd` and begin reading it.
    pub fn spawn(
        program: &Path,
        cwd: &Path,
        env: &[(String, String)],
        inbound: Arc<dyn Inbound>,
    ) -> std::io::Result<Arc<Worker>> {
        let mut command = Command::new(program);
        command
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in env {
            command.env(key, value);
        }
        ProcessTree::prepare(&mut command);
        let mut child = command.spawn()?;
        let tree = match ProcessTree::attach(&child) {
            Ok(tree) => Arc::new(tree),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr_pipe = child.stderr.take().expect("piped stderr");

        let worker = Arc::new(Worker {
            pid: child.id(),
            tree,
            stdin: Mutex::new(stdin),
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
            alive: Arc::new(AtomicBool::new(true)),
            stderr: Arc::new(Mutex::new(VecDeque::new())),
        });

        let tail = worker.stderr.clone();
        let stderr_thread = std::thread::spawn(move || drain_stderr(stderr_pipe, &tail));

        let reader = Reader {
            worker: Arc::downgrade(&worker),
            pending: worker.pending.clone(),
            alive: worker.alive.clone(),
            stderr: worker.stderr.clone(),
            inbound,
            callbacks: Arc::new(AtomicUsize::new(0)),
        };
        std::thread::spawn(move || reader.run(stdout, child, stderr_thread));
        Ok(worker)
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    /// Send a request and wait up to `timeout` for its answer.
    pub fn request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, CallError> {
        if !self.is_alive() {
            return Err(CallError::Gone);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = mpsc::channel();
        self.pending.lock().expect("pending").insert(id, send);
        if write_message(&self.stdin, &Message::request(id, method, params)).is_err() {
            self.pending.lock().expect("pending").remove(&id);
            return Err(CallError::Gone);
        }
        match receive.recv_timeout(timeout) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(CallError::Rpc(error)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.pending.lock().expect("pending").remove(&id);
                Err(CallError::TimedOut)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(CallError::Gone),
        }
    }

    /// Send a notification. A worker that has gone is not an error here: the
    /// disconnection is reported once, by the reader.
    pub fn notify(&self, method: &str, params: Value) {
        if self.is_alive() {
            let _ = write_message(&self.stdin, &Message::notification(method, params));
        }
    }

    /// Close stdin, the worker's signal to exit once it has finished.
    pub fn close_input(&self) {
        self.stdin.lock().expect("worker stdin").take();
    }

    /// End the worker and everything it started, now.
    pub fn terminate(&self) {
        self.close_input();
        self.tree.terminate();
    }

    /// Wait up to `timeout` for the worker to exit on its own.
    pub fn wait_exit(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while self.is_alive() {
            if std::time::Instant::now() > deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
    }

    /// The end of what the worker printed on stderr, redacted.
    pub fn stderr_tail(&self) -> String {
        tail_text(&self.stderr)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.tree.terminate();
    }
}

fn tail_text(buffer: &Mutex<VecDeque<u8>>) -> String {
    let bytes: Vec<u8> = buffer.lock().expect("stderr").iter().copied().collect();
    redact(&String::from_utf8_lossy(&bytes))
}

fn drain_stderr(mut pipe: impl Read, tail: &Mutex<VecDeque<u8>>) {
    let mut buffer = [0u8; 4096];
    while let Ok(read) = pipe.read(&mut buffer) {
        if read == 0 {
            break;
        }
        let mut tail = tail.lock().expect("stderr");
        tail.extend(&buffer[..read]);
        while tail.len() > STDERR_KEPT {
            tail.pop_front();
        }
    }
}

struct Reader {
    worker: std::sync::Weak<Worker>,
    pending: Arc<Pending>,
    alive: Arc<AtomicBool>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    inbound: Arc<dyn Inbound>,
    callbacks: Arc<AtomicUsize>,
}

impl Reader {
    fn run(self, stdout: impl Read, mut child: Child, stderr_thread: std::thread::JoinHandle<()>) {
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = Vec::new();
        let reason = loop {
            line.clear();
            match read_frame(&mut reader, &mut line) {
                Ok(None) => break None,
                Ok(Some(())) => {}
                Err(error) => break Some(error.to_string()),
            }
            let text = match std::str::from_utf8(&line) {
                Ok(text) => text.trim_end_matches(['\n', '\r']),
                Err(_) => break Some("a line on stdout was not UTF-8".into()),
            };
            if text.is_empty() {
                continue;
            }
            match Message::parse(text) {
                Ok(message) => self.dispatch(message),
                Err(error) => break Some(error.to_string()),
            }
        };

        // A protocol violation ends the worker; an exit already has.
        if reason.is_some() {
            if let Some(worker) = self.worker.upgrade() {
                worker.terminate();
            }
        }
        let status = child.wait();
        let _ = stderr_thread.join();
        self.alive.store(false, Ordering::Release);
        // Everybody still waiting for an answer learns there will not be one.
        self.pending.lock().expect("pending").clear();

        let stderr = tail_text(&self.stderr);
        let last_line = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();
        let mut message = match (reason, status) {
            (Some(reason), _) => format!("It broke the extension protocol: {reason}."),
            (None, Ok(status)) if status.success() => "It exited.".to_string(),
            (None, Ok(status)) => match status.code() {
                Some(code) => format!("It exited with code {code}."),
                None => "It was stopped.".to_string(),
            },
            (None, Err(error)) => format!("It could not be waited for: {error}."),
        };
        if !last_line.is_empty() {
            message.push_str(&format!(" Last message: {last_line}"));
        }
        self.inbound.disconnected(message);
    }

    fn dispatch(&self, message: Message) {
        match message {
            Message::Response {
                id: Id::Number(id),
                result,
            } => {
                if let Some(waiter) = self.pending.lock().expect("pending").remove(&id) {
                    let _ = waiter.send(result);
                }
            }
            Message::Response { .. } => {}
            Message::Notification { method, params } => self.inbound.notification(&method, params),
            Message::Request { id, method, params } => {
                let Some(worker) = self.worker.upgrade() else {
                    return;
                };
                let valid_id = matches!(&id, Id::Text(text) if text.starts_with('w'));
                if !valid_id {
                    let error = RpcError::new(
                        code::INVALID_REQUEST,
                        "a worker's request ids are strings beginning with w",
                    );
                    let _ = write_message(&worker.stdin, &Message::err(id, error));
                    return;
                }
                if self.callbacks.fetch_add(1, Ordering::AcqRel) >= MAX_CALLBACKS {
                    self.callbacks.fetch_sub(1, Ordering::AcqRel);
                    let error = RpcError::new(
                        code::LIMIT,
                        format!("at most {MAX_CALLBACKS} requests may be in flight"),
                    );
                    let _ = write_message(&worker.stdin, &Message::err(id, error));
                    return;
                }
                let inbound = self.inbound.clone();
                let callbacks = self.callbacks.clone();
                std::thread::spawn(move || {
                    let answer = match inbound.request(&method, params) {
                        Ok(value) => Message::ok(id, value),
                        Err(error) => Message::err(id, error),
                    };
                    let _ = write_message(&worker.stdin, &answer);
                    callbacks.fetch_sub(1, Ordering::AcqRel);
                });
            }
        }
    }
}

/// Read one line into `line`. `Ok(None)` at the end of the stream; an error
/// for a line longer than the protocol allows.
fn read_frame(reader: &mut impl BufRead, line: &mut Vec<u8>) -> Result<Option<()>, FrameError> {
    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(_) => return Ok(None),
        };
        if available.is_empty() {
            return Ok(if line.is_empty() { None } else { Some(()) });
        }
        let (take, found) = match available.iter().position(|b| *b == b'\n') {
            Some(at) => (at + 1, true),
            None => (available.len(), false),
        };
        if line.len() + take > MAX_MESSAGE_BYTES + 2 {
            return Err(FrameError::TooLong);
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if found {
            return Ok(Some(()));
        }
    }
}

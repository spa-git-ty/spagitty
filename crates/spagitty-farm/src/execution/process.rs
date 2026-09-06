// SPDX-License-Identifier: GPL-3.0-or-later

//! Running an agent, watching it, and stopping it.
//!
//! # Why one thread per run and not a runtime
//!
//! Everything this does is wait on a pipe and wait on a process. The rest of
//! Spagitty already solves that with an OS thread per long-running job —
//! `clone_worker`, `graph_worker`, `search_worker` — and a farm's concurrency is
//! measured in single digits. A thread each is the cheaper answer in every
//! sense that matters here, including the one where nobody has to learn a
//! runtime to read it.
//!
//! # Cancellation is a signal, not a request
//!
//! An agent is somebody else's program. There is no cooperative stop to ask
//! for, so [`Session::cancel`] kills the child, exactly as `clone_worker` does
//! for `git clone` and for the same reason. What follows the kill is careful:
//! the reader threads are joined before the outcome is decided, so a transcript
//! is never truncated by the reaper winning a race with the reader.
//!
//! # Killing one process is not enough
//!
//! An agent spawns processes constantly — `git`, `npm`, `cargo`, a language
//! server, a test suite. Killing only the process Spagitty started leaves all
//! of those running, and they inherited the pipe: the reader threads then block
//! until the orphans exit, so "stop" takes as long as whatever the agent
//! happened to leave behind. That is not a hypothetical — it turned a
//! cancellation test that should take milliseconds into a twenty-second wait.
//!
//! Unix runs use process groups; Windows runs enter a job before their primary
//! thread is resumed. The cancellation handle outlives transfer to a waiter.
//!
//! # stdout and stderr are one stream
//!
//! Agents narrate on both, and interleaving them in the order they arrived is
//! what a person reading the transcript wants. Two separate transcripts would
//! have to be merged by timestamp to be read, and the timestamps are the ones
//! we added rather than the ones the agent meant.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::tree::ProcessTree;
use crate::agent::AgentCommand;
use crate::error::{Error, Result};
use crate::execution::log::TranscriptWriter;
use crate::execution::narrate::Narrator;

/// What a run reports as it goes.
///
/// A trait rather than a channel, so this module has no opinion about where the
/// output ends up. The Tauri layer emits events; the tests collect a `Vec`.
pub trait Sink: Send + Sync + std::fmt::Debug {
    /// One line of the agent's output, in the order it arrived.
    fn line(&self, text: &str);
}

/// A sink that keeps everything, for tests and for a headless run.
#[derive(Debug, Default)]
pub struct Collected {
    lines: Mutex<Vec<String>>,
}

impl Collected {
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("collected lines").clone()
    }
}

impl Sink for Collected {
    fn line(&self, text: &str) {
        self.lines
            .lock()
            .expect("collected lines")
            .push(text.to_string());
    }
}

/// How a run ended, as the process saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    Ok,
    Failed { code: Option<i32>, message: String },
    Cancelled,
}

/// Cancellation remains accessible after the session moves to a waiter.
#[derive(Debug, Clone)]
pub struct Cancellation {
    child: Arc<Mutex<Child>>,
    cancelled: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    tree: Arc<ProcessTree>,
}

impl Cancellation {
    pub fn cancel(&self) {
        let mut child = self.child.lock().expect("agent child lock");
        if self.finished.load(Ordering::Acquire) {
            return;
        }
        self.cancelled.store(true, Ordering::Release);
        self.tree.terminate();
        let _ = child.kill();
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// A running agent.
///
/// Dropping one cancels it and waits, so a farm that goes away does not leave
/// four models running against a worktree that is about to be deleted. That is
/// the same contract `CloneWorker` has, and for the same reason.
#[derive(Debug)]
pub struct Session {
    cancellation: Cancellation,
    readers: Vec<JoinHandle<()>>,
}

impl Session {
    /// Ask the operating system to stop it, and everything it started.
    ///
    /// A child that has already exited is not an error: pressing stop as a run
    /// finishes is a race, not a failure.
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancellation.was_cancelled()
    }

    /// Wait for the agent to finish and say how it went.
    ///
    /// The reader threads are joined *before* the exit status is read, so the
    /// transcript is complete by the time anything looks at it. Getting that
    /// order wrong produces a handoff block that is sometimes missing, which is
    /// the worst kind of defect to chase.
    pub fn wait(mut self) -> Ended {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
        // Never hold the child mutex across a blocking wait: a program may
        // close its pipes and keep running, and Stop must still acquire it.
        let status = loop {
            let mut child = self.cancellation.child.lock().expect("agent child lock");
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.cancellation.finished.store(true, Ordering::Release);
                    break Ok(status);
                }
                Err(error) => break Err(error),
                Ok(None) => {}
            }
            drop(child);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let cancelled = self.was_cancelled();

        match status {
            _ if cancelled => Ended::Cancelled,
            Ok(status) if status.success() => Ended::Ok,
            Ok(status) => Ended::Failed {
                code: status.code(),
                message: format!("the agent exited with {status}"),
            },
            Err(error) => Ended::Failed {
                code: None,
                message: error.to_string(),
            },
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.readers.is_empty() {
            self.cancel();
            for reader in self.readers.drain(..) {
                let _ = reader.join();
            }
            let _ = self
                .cancellation
                .child
                .lock()
                .expect("agent child lock")
                .wait();
        }
    }
}

/// Start `command` in `workdir`, streaming to `sink` and `transcript`.
///
/// `narrator` sits between the pipe and both of them: the provider decides how
/// its output should be read, and what reaches the transcript file is what a
/// person would want to read rather than what the process happened to print.
/// See [`crate::execution::narrate`] for why that translation happens here and
/// not in the interface.
///
/// The environment is the one Spagitty was started with, minus nothing and plus
/// two: `SPAGITTY_FARM` so an agent can tell it is being run by a farm, and
/// `GIT_TERMINAL_PROMPT=0` so a git operation the agent performs cannot block
/// on a credential prompt no terminal will answer. Secrets are never added —
/// the plan says never to inject them, and an agent that needs credentials
/// should get them from the same helper the user's own git does.
pub fn start(
    command: &AgentCommand,
    workdir: &Path,
    transcript: TranscriptWriter,
    sink: Arc<dyn Sink>,
    narrator: Box<dyn Narrator>,
) -> Result<Session> {
    let mut process = Command::new(&command.program);
    process
        .args(&command.args)
        .current_dir(workdir)
        .env("SPAGITTY_FARM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // A group of its own, so cancelling reaches everything the agent starts.
    // Without it a stopped agent leaves `cargo test` running against a worktree
    // that is about to be deleted.
    ProcessTree::prepare(&mut process);

    process.stdin(if command.stdin.is_some() {
        Stdio::piped()
    } else {
        // Never inherited. An agent that reads standard input when nothing
        // is piped to it would read the terminal Spagitty was launched
        // from, which is either nothing or somebody else's keystrokes.
        Stdio::null()
    });

    let mut child = process.spawn().map_err(|error| {
        Error::Refused(format!(
            "could not start {}: {error}",
            command.program.display()
        ))
    })?;

    let tree = match ProcessTree::attach(&child) {
        Ok(tree) => Arc::new(tree),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Refused(format!(
                "could not contain agent process: {error}"
            )));
        }
    };

    if let Some(prompt) = &command.stdin {
        // Written and closed immediately: an agent reading its prompt from a
        // pipe waits for end-of-file, and a pipe held open by the parent is a
        // hang that looks exactly like a slow model.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(prompt.as_bytes());
        }
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let child = Arc::new(Mutex::new(child));
    let cancelled = Arc::new(AtomicBool::new(false));

    // One transcript, two readers. The writer is behind a mutex because both
    // reader threads append to it, and interleaving whole lines is exactly what
    // is wanted — the alternative is two files and a merge.
    let shared = Arc::new(Mutex::new(transcript));
    // One narrator for the run, not one per pipe: it carries the state that
    // stops a final event repeating what was already said, and stdout and
    // stderr are one stream by the time anyone reads them.
    let narrator = Arc::new(Mutex::new(narrator));
    let mut readers = Vec::new();
    for pipe in [stdout.map(Pipe::Out), stderr.map(Pipe::Err)] {
        let Some(pipe) = pipe else { continue };
        let sink = sink.clone();
        let shared = shared.clone();
        let narrator = narrator.clone();
        readers.push(
            std::thread::Builder::new()
                .name("spagitty-farm-agent".to_string())
                .spawn(move || pump(pipe, &*sink, &shared, &narrator))
                .expect("spawning an agent reader"),
        );
    }

    Ok(Session {
        cancellation: Cancellation {
            child,
            cancelled,
            tree,
            finished: Arc::new(AtomicBool::new(false)),
        },
        readers,
    })
}

enum Pipe {
    Out(std::process::ChildStdout),
    Err(std::process::ChildStderr),
}

/// Read one pipe to the end, line by line.
///
/// Invalid UTF-8 is replaced rather than fatal: an agent that prints a progress
/// spinner or a stray byte should not end the run, and `from_utf8_lossy` is
/// what every other reader in this repository does with the same problem.
fn pump(
    pipe: Pipe,
    sink: &dyn Sink,
    transcript: &Mutex<TranscriptWriter>,
    narrator: &Mutex<Box<dyn Narrator>>,
) {
    let reader: Box<dyn BufRead> = match pipe {
        Pipe::Out(out) => Box::new(BufReader::new(out)),
        Pipe::Err(err) => Box::new(BufReader::new(err)),
    };
    let mut buffer = Vec::new();
    let mut reader = reader;
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                let text = text.trim_end_matches(['\n', '\r']);
                // One raw line can be nothing (an event nobody needs to see) or
                // several lines (a message the agent wrote as a paragraph).
                let narrated = match narrator.lock() {
                    Ok(mut narrator) => narrator.narrate(text),
                    Err(_) => vec![text.to_string()],
                };
                for line in narrated {
                    if let Ok(mut transcript) = transcript.lock() {
                        transcript.line(&line);
                    }
                    sink.line(&line);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn shell(script: &str, stdin: Option<&str>) -> AgentCommand {
        AgentCommand {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            stdin: stdin.map(|text| text.to_string()),
        }
    }

    struct Run {
        _dir: tempfile::TempDir,
        sink: Arc<Collected>,
        log: PathBuf,
    }

    fn run(command: AgentCommand) -> (Run, Ended) {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("run.log");
        let sink = Arc::new(Collected::default());
        let session = start(
            &command,
            dir.path(),
            TranscriptWriter::create(&log).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();
        let ended = session.wait();
        (
            Run {
                _dir: dir,
                sink,
                log,
            },
            ended,
        )
    }

    #[test]
    fn output_reaches_the_sink_and_the_transcript() {
        let (run, ended) = run(shell("echo hello; echo world", None));
        assert_eq!(ended, Ended::Ok);
        assert_eq!(run.sink.lines(), ["hello", "world"]);
        assert_eq!(crate::execution::log::read(&run.log), "hello\nworld\n");
    }

    #[test]
    fn both_streams_land_in_one_transcript() {
        let (run, _) = run(shell("echo out; echo err >&2", None));
        let lines = run.sink.lines();
        assert!(lines.contains(&"out".to_string()), "{lines:?}");
        assert!(lines.contains(&"err".to_string()), "{lines:?}");
    }

    #[test]
    fn a_failing_agent_reports_its_exit_code() {
        let (_, ended) = run(shell("exit 3", None));
        match ended {
            Ended::Failed { code, .. } => assert_eq!(code, Some(3)),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn a_prompt_on_standard_input_reaches_the_agent() {
        let (run, ended) = run(shell("cat", Some("do the thing\n")));
        assert_eq!(ended, Ended::Ok);
        assert_eq!(run.sink.lines(), ["do the thing"]);
    }

    #[test]
    fn an_agent_reading_stdin_that_was_given_none_does_not_hang() {
        // stdin is `null`, so `cat` reads end-of-file immediately. Inheriting
        // it instead would block here forever.
        let (_, ended) = run(shell("cat", None));
        assert_eq!(ended, Ended::Ok);
    }

    #[test]
    fn a_command_that_does_not_exist_fails_to_start_rather_than_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let error = start(
            &AgentCommand {
                program: PathBuf::from("/nonexistent/agent"),
                args: vec![],
                stdin: None,
            },
            dir.path(),
            TranscriptWriter::create(&dir.path().join("run.log")).unwrap(),
            Arc::new(Collected::default()),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "refused");
    }

    #[test]
    fn cancelling_stops_the_agent_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(Collected::default());
        let session = start(
            &shell("echo started; sleep 60", None),
            dir.path(),
            TranscriptWriter::create(&dir.path().join("run.log")).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();

        // Wait for the agent to say something, so the kill lands on a running
        // process rather than on one that has not started yet.
        while sink.lines().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        session.cancel();
        assert_eq!(session.wait(), Ended::Cancelled);
    }

    #[test]
    fn a_cancelled_run_keeps_what_the_agent_had_already_said() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("run.log");
        let sink = Arc::new(Collected::default());
        let session = start(
            &shell("echo first; echo second; sleep 60", None),
            dir.path(),
            TranscriptWriter::create(&log).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();
        while sink.lines().len() < 2 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        session.cancel();
        session.wait();
        assert_eq!(crate::execution::log::read(&log), "first\nsecond\n");
    }

    #[test]
    fn stopping_an_agent_stops_what_the_agent_started() {
        // The defect this exists for: `sh` forks `sleep`, `sleep` inherits the
        // pipe, and killing only `sh` leaves the reader blocked until `sleep`
        // finishes on its own. Twenty seconds, for a stop that should be
        // instant.
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(Collected::default());
        let session = start(
            &shell("echo started; sleep 30; echo never", None),
            dir.path(),
            TranscriptWriter::create(&dir.path().join("run.log")).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();
        while sink.lines().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let started = std::time::Instant::now();
        session.cancel();
        assert_eq!(session.wait(), Ended::Cancelled);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "stopping took {:?}; a child process was left holding the pipe",
            started.elapsed()
        );
    }

    #[test]
    fn dropping_a_session_stops_the_agent() {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(Collected::default());
        let session = start(
            &shell("echo started; sleep 60", None),
            dir.path(),
            TranscriptWriter::create(&dir.path().join("run.log")).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();
        while sink.lines().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let started = std::time::Instant::now();
        drop(session);
        // If the drop waited for `sleep 60` this would take a minute.
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn the_agent_runs_in_the_directory_it_was_given() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("marker.txt"), "here").unwrap();
        let sink = Arc::new(Collected::default());
        let session = start(
            &shell("cat marker.txt", None),
            dir.path(),
            TranscriptWriter::create(&dir.path().join("run.log")).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::Verbatim),
        )
        .unwrap();
        assert_eq!(session.wait(), Ended::Ok);
        assert_eq!(sink.lines(), ["here"]);
    }

    #[test]
    fn an_agent_can_tell_it_is_being_run_by_a_farm() {
        let (run, _) = run(shell("echo $SPAGITTY_FARM", None));
        assert_eq!(run.sink.lines(), ["1"]);
    }

    #[test]
    fn a_line_that_is_not_utf8_does_not_end_the_run() {
        let (run, ended) = run(shell("printf 'good\\n\\xff\\nafter\\n'", None));
        assert_eq!(ended, Ended::Ok);
        let lines = run.sink.lines();
        assert_eq!(lines.first().unwrap(), "good");
        assert_eq!(lines.last().unwrap(), "after");
    }

    /// BUG-021, and the one that would have been found late.
    ///
    /// Streaming the provider's machine format is only safe because the
    /// transcript on disk is narrated first. If the raw JSON went to the file,
    /// the handoff block would be inside a JSON string and
    /// `Handoff::parse` — and with it every task's status — would quietly find
    /// nothing.
    #[test]
    fn a_streamed_run_writes_prose_to_the_transcript_and_keeps_its_handoff() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("run.log");
        let sink = Arc::new(Collected::default());
        // What `claude -p --output-format stream-json --verbose` actually
        // prints: an init event, a tool call, and the answer as one message.
        // `printf '%s\\n'` rather than `echo`: /bin/sh's echo expands the
        // `\\n` inside the JSON string and splits one event across four lines,
        // which tests the shell rather than the narrator.
        let script = concat!(
            r#"printf '%s\n' '{"type":"system","subtype":"init","model":"claude-opus-5"}'; "#,
            r#"printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/auth.rs"}}]}}'; "#,
            r#"printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"Rotated the tokens.\n```spagitty-handoff\n{\"status\":\"completed\",\"summary\":\"Rotated tokens\"}\n```"}]}}'; "#,
            r#"printf '%s\n' '{"type":"result","subtype":"success","result":"done","duration_ms":4000}'"#
        );
        let session = start(
            &shell(script, None),
            dir.path(),
            TranscriptWriter::create(&log).unwrap(),
            sink.clone(),
            Box::new(crate::execution::narrate::ClaudeStream::default()),
        )
        .unwrap();
        assert_eq!(session.wait(), Ended::Ok);

        let transcript = crate::execution::log::read(&log);
        assert!(
            !transcript.contains(r#""type":"assistant""#),
            "raw stream events reached the transcript:\n{transcript}"
        );
        assert!(transcript.contains("· Read src/auth.rs"), "{transcript}");

        let handoff = crate::model::Handoff::parse(&transcript);
        assert_eq!(handoff.status, crate::model::HandoffStatus::Completed);
        assert_eq!(handoff.summary, "Rotated tokens");

        // And the interface saw the same lines as they arrived, not at the end.
        assert!(sink.lines().iter().any(|line| line == "· Read src/auth.rs"));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Fetch and push, watched while they run (FEAT-018).
//!
//! # Why a worker
//!
//! Both were blocking commands: `--progress` was passed and nothing read it, so
//! git's output arrived all at once when the process ended. On a large fetch
//! that is a UI that says nothing for a minute and then says everything, which
//! is indistinguishable from a UI that has hung.
//!
//! The same reason as the rebase worker applies too: a blocking command holds
//! the session lock, so nothing else can ask the repository anything while a
//! slow network operation runs.
//!
//! # Why it is not the clone worker
//!
//! A clone is cancellable, owns a destination directory it may have to remove,
//! and produces a repository rather than changing one. Fetch and push change a
//! repository that is already open, and neither is cancelled — killing a push
//! mid-transfer is not something with a defined outcome, and a fetch that is
//! interrupted simply leaves the refs it had already written.
//!
//! What *is* shared is the progress parsing: `git fetch --progress` and
//! `git push --progress` write the same `phase: 42% (…)` lines a clone does, so
//! [`spagitty_core::clone::progress`] reads all three.

use std::path::PathBuf;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use spagitty_core::clone::{self, Progress};
use spagitty_core::shell;
use tauri::{AppHandle, Emitter, Runtime};

pub const PROGRESS_EVENT: &str = "network-progress";
pub const DONE_EVENT: &str = "network-done";

/// Which operation a worker is running, so one screen can label both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    Fetch,
    Push,
}

impl Operation {
    fn label(self) -> &'static str {
        match self {
            Operation::Fetch => "fetch",
            Operation::Push => "push",
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    token: u64,
    operation: Operation,
    #[serde(flatten)]
    step: Progress,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    token: u64,
    operation: Operation,
    ok: bool,
    /// git's own message, which for a refused push or a missing credential is
    /// the entire useful content of the failure.
    error: Option<String>,
    /// git's last words on success — "Everything up-to-date", a ref update
    /// summary — so a fetch that brought nothing can say so.
    summary: Option<String>,
}

/// A running fetch or push. Dropping it waits rather than stopping it.
pub struct NetworkWorker {
    handle: Option<JoinHandle<()>>,
}

impl Drop for NetworkWorker {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// What to run. Built by the command while it holds the session lock.
pub enum Job {
    Fetch {
        remote: String,
        prune: bool,
    },
    Push {
        remote: String,
        refspec: String,
        force: bool,
    },
}

impl Job {
    fn operation(&self) -> Operation {
        match self {
            Job::Fetch { .. } => Operation::Fetch,
            Job::Push { .. } => Operation::Push,
        }
    }

    fn start(&self, workdir: &std::path::Path) -> spagitty_core::Result<std::process::Child> {
        match self {
            Job::Fetch { remote, prune } => shell::fetch_spawn(workdir, remote, *prune),
            Job::Push {
                remote,
                refspec,
                force,
            } => shell::push_spawn(workdir, remote, refspec, *force),
        }
    }
}

/// Start the job and forward what git says about it.
///
/// Fails only if `git` could not be started; everything after that arrives on
/// [`DONE_EVENT`].
pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    workdir: PathBuf,
    job: Job,
    token: u64,
) -> spagitty_core::Result<NetworkWorker> {
    let operation = job.operation();
    let mut child = job.start(&workdir)?;
    let stderr = child.stderr.take();

    let handle = std::thread::Builder::new()
        .name(format!("spagitty-{}-{token}", operation.label()))
        .spawn(move || {
            let output = stderr
                .map(|stderr| report(&app, token, operation, stderr))
                .unwrap_or_default();
            let status = child.wait();

            let error = match &status {
                Ok(status) if status.success() => None,
                // Keep the refusal, even when git follows it with hints (BUG-081).
                Ok(status) => Some(
                    output
                        .error()
                        .unwrap_or_else(|| format!("git {} failed ({status})", operation.label())),
                ),
                Err(error) => Some(error.to_string()),
            };

            let _ = app.emit(
                DONE_EVENT,
                DoneEvent {
                    token,
                    operation,
                    ok: error.is_none(),
                    summary: if error.is_none() { output.last } else { None },
                    error,
                },
            );
        })
        .expect("spawning the network worker");

    Ok(NetworkWorker {
        handle: Some(handle),
    })
}

/// Git's last line for a successful operation, and its refusal for a failure.
#[derive(Default)]
struct Output {
    last: Option<String>,
    failures: Vec<String>,
    /// What the remote said that was neither progress nor an error of its
    /// own: a hook's words, when a hook refused the push.
    remote: Vec<String>,
}

/// How many of the remote's lines are kept. A hook can print a page.
const REMOTE_LINES: usize = 12;

impl Output {
    fn error(&self) -> Option<String> {
        if self.failures.is_empty() {
            return self.last.clone();
        }
        // `! [remote rejected] main -> main (pre-receive hook declined)` says
        // that the remote refused and not why. The why is what its hook
        // printed before it, and it is the only part a person can act on.
        let refused_there = self
            .failures
            .iter()
            .any(|line| line.contains("[remote rejected]"));
        let said: &[String] = if refused_there { &self.remote } else { &[] };
        let said = &said[said.len().saturating_sub(REMOTE_LINES)..];
        Some(
            said.iter()
                .chain(&self.failures)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

/// The phases git's own progress on the remote is reported under. A line of
/// one of these is the transfer, not something anybody wrote for a person.
const REMOTE_PROGRESS: &[&str] = &[
    "Enumerating objects",
    "Counting objects",
    "Compressing objects",
    "Receiving objects",
    "Writing objects",
    "Resolving deltas",
    "Checking connectivity",
    "Updating files",
];

/// Whether a line the remote sent is git's progress and not a message.
fn is_remote_progress(step: &Progress) -> bool {
    step.percent.is_some()
        || REMOTE_PROGRESS.contains(&step.phase.as_str())
        || step.line.starts_with("Total ")
}

/// Forward git's progress until it stops talking, keeping its error lines.
///
/// The same shape as the clone worker's, and for the same reason: git ends a
/// progress update with a carriage return rather than a newline, because it is
/// rewriting one line on a terminal, so both count as the end of a line.
fn report<R: Runtime>(
    app: &AppHandle<R>,
    token: u64,
    operation: Operation,
    stderr: impl std::io::Read,
) -> Output {
    use std::io::{BufReader, Read};

    let mut reader = BufReader::new(stderr);
    let mut line: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    let mut last: Option<Progress> = None;
    let mut output = Output::default();

    let mut flush = |line: &mut Vec<u8>, last: &mut Option<Progress>| {
        let text = String::from_utf8_lossy(line).into_owned();
        line.clear();

        // `clone::progress` takes the prefix off, so it is looked for first.
        let from_remote = text.trim_start().starts_with("remote:");
        let Some(step) = clone::progress(&text) else {
            return;
        };
        if step.line.starts_with("fatal:")
            || step.line.starts_with("error:")
            || step.line.starts_with("! [")
        {
            output.failures.push(step.line.clone());
        } else if from_remote && !step.line.starts_with("hint:") && !is_remote_progress(&step) {
            output.remote.push(step.line.clone());
        }
        output.last = Some(step.line.clone());
        if clone::is_new_step(last.as_ref(), &step) {
            let _ = app.emit(
                PROGRESS_EVENT,
                ProgressEvent {
                    token,
                    operation,
                    step: step.clone(),
                },
            );
        }
        *last = Some(step);
    };

    while reader.read(&mut byte).unwrap_or(0) == 1 {
        match byte[0] {
            b'\r' | b'\n' => flush(&mut line, &mut last),
            other => line.push(other),
        }
    }
    flush(&mut line, &mut last);

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{self, Emitted};
    use serde_json::Value;
    use spagitty_core::fixture::Fixture;

    #[test]
    fn a_refusal_survives_the_hints_that_follow_it() {
        let app = testing::app();
        let steps = Emitted::<Value>::on(app.handle(), PROGRESS_EVENT);
        let output = report(
            app.handle(),
            81,
            Operation::Push,
            &b"Writing objects: 50% (1/2)\rWriting objects: 100% (2/2)\r\n \
              ! [rejected] main -> main (non-fast-forward)\n\
              error: failed to push some refs\n\
              hint: See the 'Note about fast-forwards' in 'git push --help'.\n"[..],
        );

        assert_eq!(
            output.error().as_deref(),
            Some("! [rejected] main -> main (non-fast-forward)\nerror: failed to push some refs")
        );
        let steps = steps.all();
        assert_eq!(steps[0]["percent"], 50);
        assert_eq!(steps[1]["percent"], 100);
        assert_eq!(steps[0]["token"], 81);
        assert_eq!(steps[0]["operation"], "push");
    }

    /// A hook that refuses a push says why, and git passes it on. Without
    /// it the refusal says only that a hook declined.
    #[test]
    fn a_push_a_hook_refused_keeps_what_the_hook_said() {
        let app = testing::app();
        let output = report(
            app.handle(),
            85,
            Operation::Push,
            &b"Writing objects: 100% (3/3), 280 bytes | 280.00 KiB/s, done.\n\
              Total 3 (delta 0), reused 0 (delta 0), pack-reused 0\n\
              remote: Resolving deltas: 100% (1/1), done.\n\
              remote: Commit subject must include a ticket ID\n\
              remote: See https://wiki.example/commits\n\
              remote: \n\
              To example.com:team/repo.git\n \
              ! [remote rejected] main -> main (pre-receive hook declined)\n\
              error: failed to push some refs to 'example.com:team/repo.git'\n"[..],
        );

        assert_eq!(
            output.error().as_deref(),
            Some(
                "Commit subject must include a ticket ID\n\
                 See https://wiki.example/commits\n\
                 ! [remote rejected] main -> main (pre-receive hook declined)\n\
                 error: failed to push some refs to 'example.com:team/repo.git'"
            )
        );
    }

    /// What the remote said is the reason only when the remote refused. A
    /// push git itself refuses keeps git's own lines and nothing else.
    #[test]
    fn what_the_remote_said_is_not_added_to_a_refusal_that_was_not_its_own() {
        let app = testing::app();
        let output = report(
            app.handle(),
            86,
            Operation::Push,
            &b"remote: Welcome to the forge\n\
              remote: Counting objects: 5, done.\n\
              remote: hint: nothing to see\n \
              ! [rejected] main -> main (non-fast-forward)\n\
              error: failed to push some refs\n"[..],
        );

        assert_eq!(
            output.error().as_deref(),
            Some("! [rejected] main -> main (non-fast-forward)\nerror: failed to push some refs")
        );
        // A hook that prints a page is kept to its last lines.
        let mut long = String::new();
        for n in 0..40 {
            long.push_str(&format!("remote: line {n}\n"));
        }
        long.push_str(" ! [remote rejected] main -> main (pre-receive hook declined)\n");
        let said = report(app.handle(), 87, Operation::Push, long.as_bytes())
            .error()
            .expect("an error");
        assert_eq!(said.lines().count(), REMOTE_LINES + 1);
        assert!(said.starts_with("line 28\n"));
        assert!(said.ends_with("(pre-receive hook declined)"));
    }

    #[test]
    fn remote_errors_and_a_final_line_without_a_newline_are_kept() {
        let app = testing::app();
        let output = report(
            app.handle(),
            82,
            Operation::Fetch,
            &b"remote: error: access denied\r\nfatal: Could not read from remote repository.\n\
              Please make sure you have the correct access rights"[..],
        );
        assert_eq!(
            output.error().as_deref(),
            Some("error: access denied\nfatal: Could not read from remote repository.")
        );
        assert_eq!(
            output.last.as_deref(),
            Some("Please make sure you have the correct access rights")
        );
    }

    #[test]
    fn success_and_unclassified_failures_keep_the_last_nonempty_line() {
        let app = testing::app();
        let output = report(
            app.handle(),
            83,
            Operation::Push,
            &b"Enumerating objects: 2, done.\nEverything up-to-date\n\n"[..],
        );
        assert_eq!(output.last.as_deref(), Some("Everything up-to-date"));
        assert_eq!(output.error(), output.last);
        assert!(report(app.handle(), 84, Operation::Fetch, &b"\r\n"[..])
            .error()
            .is_none());
    }

    #[test]
    fn a_rejected_push_emits_gits_reason_and_leaves_the_remote_unchanged() {
        let app = testing::app();
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);
        let local = Fixture::empty();
        local.write("notes.txt", "Base\n");
        local.git(&["add", "notes.txt"]);
        local.commit("Base");
        let remote = tempfile::tempdir().expect("remote");
        let initialized = shell::program("git")
            .current_dir(remote.path())
            .args(["init", "--bare"])
            .output()
            .expect("bare remote");
        assert!(initialized.status.success());
        let remote_path = remote.path().to_str().expect("remote path");
        local.git(&["remote", "add", "origin", remote_path]);
        local.git(&["push", "--set-upstream", "origin", "main"]);

        let other = Fixture::empty();
        other.git(&["remote", "add", "origin", remote_path]);
        other.git(&["fetch", "origin"]);
        other.git(&["reset", "--hard", "origin/main"]);
        other.write("notes.txt", "Their change\n");
        let remote_head = other.commit_all("Their change");
        other.git(&["push", "origin", "main"]);
        local.write("notes.txt", "Our change\n");
        local.commit_all("Our change");

        let worker = spawn(
            app.handle().clone(),
            local.path().to_path_buf(),
            Job::Push {
                remote: "origin".into(),
                refspec: "main".into(),
                force: false,
            },
            85,
        )
        .expect("push worker");
        let events = done.at_least(1);
        drop(worker);
        assert_eq!(events[0]["ok"], false);
        assert!(events[0]["summary"].is_null());
        let error = events[0]["error"].as_str().expect("git's error");
        assert!(error.contains("! [rejected]"), "{error}");
        assert!(error.contains("error: failed to push"), "{error}");
        assert!(!error.contains("hint:"), "{error}");
        let remote_ref = local.git(&["ls-remote", "origin", "refs/heads/main"]);
        assert_eq!(
            remote_ref.split_whitespace().next(),
            Some(remote_head.as_str())
        );
    }
}

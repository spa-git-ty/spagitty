// SPDX-License-Identifier: GPL-3.0-or-later

//! One verification command, and how it is run.
//!
//! # Why there is no shell
//!
//! `cargo test` is three words and a program. `npm test && npm run check` is a
//! shell expression, and running it means handing a string to `sh -c`, which
//! means the farm's configuration can run anything at all with the user's
//! credentials. That is not a hypothetical for a feature whose whole subject is
//! running other people's programs.
//!
//! So a command is split on whitespace, honouring quotes, and executed
//! directly. Two commands means two entries in the list, which is also what
//! makes the interface able to say which one failed:
//!
//! ```text
//! ✓ cargo test
//! ✗ npm run check
//! ```
//!
//! A user who genuinely needs a shell can put one in the list explicitly —
//! `sh -c "..."` — and that is then a visible decision rather than a property of
//! every command they type.

use crate::execution::tree::ProcessTree;
use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// How long a verification command may take.
///
/// Twenty minutes. Long enough for a real test suite on a cold cache, short
/// enough that a command waiting on a prompt does not hold a task open until
/// somebody notices. A suite that genuinely takes longer should be split, and
/// the failure says so.
pub const TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// How much of a command's output is kept.
///
/// The tail, not the head: a failing test suite says what failed at the end,
/// and the first eight kilobytes of a compile log is the part nobody needs.
pub const OUTPUT_BYTES: usize = 8 * 1024;

/// What running one command produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub command: String,
    pub passed: bool,
    /// The tail of stdout and stderr, interleaved.
    pub output: String,
    pub duration_ms: u64,
}

/// Split a command line into a program and its arguments.
///
/// Handles single and double quotes so a path with a space works. Everything
/// else — pipes, redirections, variables, `&&` — is *not* interpreted, and ends
/// up as a literal argument, where it will fail loudly rather than quietly
/// doing something else.
pub fn split(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut any = false;

    for character in line.chars() {
        match (quote, character) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), c) => current.push(c),
            (None, c @ ('\'' | '"')) => {
                quote = Some(c);
                // An empty quoted string is still an argument.
                any = true;
            }
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() || any {
                    parts.push(std::mem::take(&mut current));
                    any = false;
                }
            }
            (None, c) => current.push(c),
        }
    }
    if !current.is_empty() || any {
        parts.push(current);
    }
    parts
}

/// Run one command in `workdir`.
///
/// A command that cannot be started at all is a *failure of that command*, not
/// an error: "npm is not installed" is exactly what the user needs to see in
/// the verification list, and raising it would stop the remaining commands from
/// running and hide it.
pub fn run(workdir: &Path, line: &str) -> CommandResult {
    run_cancellable(workdir, line, &AtomicBool::new(false), TIMEOUT)
}

pub fn run_cancellable(
    workdir: &Path,
    line: &str,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> CommandResult {
    let started = Instant::now();
    let result = |passed, output| CommandResult {
        command: line.to_string(),
        passed,
        output,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    if cancelled.load(Ordering::Acquire) {
        return result(false, "Verification stopped.".into());
    }
    let parts = split(line);
    let Some((program, args)) = parts.split_first() else {
        return result(false, "there is no command here".into());
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    ProcessTree::prepare(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return result(false, format!("could not run `{program}`: {error}")),
    };
    let tree = match ProcessTree::attach(&child) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return result(
                false,
                format!("could not contain verification process: {error}"),
            );
        }
    };
    let output = Arc::new(Mutex::new(VecDeque::with_capacity(OUTPUT_BYTES)));
    let readers = [
        drain(child.stdout.take().expect("piped stdout"), output.clone()),
        drain(child.stderr.take().expect("piped stderr"), output.clone()),
    ];
    let deadline = Instant::now() + timeout;
    let mut status = None;
    let mut stopped = None;
    loop {
        if cancelled.load(Ordering::Acquire) {
            stopped = Some("Verification stopped.".to_string());
        } else if Instant::now() >= deadline {
            stopped = Some(format!(
                "Verification timed out after {} seconds.",
                timeout.as_secs()
            ));
        }
        if stopped.is_some() {
            tree.terminate();
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(found) => status = found,
                Err(error) => {
                    stopped = Some(error.to_string());
                    tree.terminate();
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
        // An exited parent can leave descendants holding its pipes. Keep the
        // deadline and Stop active until both streams have reached EOF.
        if status.is_some() && readers.iter().all(|reader| reader.is_finished()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    for reader in readers {
        match reader.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                stopped.get_or_insert(error.to_string());
            }
            Err(_) => {
                stopped.get_or_insert("Could not read verification output.".into());
            }
        }
    }
    let bytes: Vec<u8> = output
        .lock()
        .expect("verification output lock")
        .iter()
        .copied()
        .collect();
    let text = tail(&String::from_utf8_lossy(&bytes));
    match stopped {
        Some(reason) => result(false, format!("{reason}\n{text}")),
        None => result(status.is_some_and(|status| status.success()), text),
    }
}

fn drain(
    mut pipe: impl Read + Send + 'static,
    output: Arc<Mutex<VecDeque<u8>>>,
) -> std::thread::JoinHandle<std::io::Result<()>> {
    std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            let read = match pipe.read(&mut buffer) {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                other => other?,
            };
            if read == 0 {
                return Ok(());
            }
            let mut output = output.lock().expect("verification output lock");
            let overflow = (output.len() + read).saturating_sub(OUTPUT_BYTES);
            output.drain(..overflow);
            output.extend(&buffer[..read]);
        }
    })
}

/// The last [`OUTPUT_BYTES`] of `text`, cut at a line boundary.
fn tail(text: &str) -> String {
    if text.len() <= OUTPUT_BYTES {
        return text.to_string();
    }
    // Byte index into a UTF-8 string has to land on a character boundary.
    let mut from = text.len() - OUTPUT_BYTES;
    while from < text.len() && !text.is_char_boundary(from) {
        from += 1;
    }
    let cut = &text[from..];
    match cut.find('\n') {
        Some(index) => cut[index + 1..].to_string(),
        None => cut.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_line() -> String {
        format!(
            "\"{}\" --exact verification::command::tests::command_fixture --nocapture",
            std::env::current_exe().unwrap().display()
        )
    }

    #[test]
    fn command_fixture() {
        let Ok(mode) = std::fs::read_to_string(".spagitty-verification-test-mode") else {
            return;
        };
        if mode == "echo" {
            println!("all good");
            return;
        }
        if mode == "failure" {
            eprintln!("2 tests failed");
            std::process::exit(1);
        }
        if mode == "cwd" {
            assert_eq!(std::fs::read_to_string("here.txt").unwrap(), "yes");
            return;
        }
        if mode == "hang" {
            std::thread::sleep(Duration::from_secs(3));
            return;
        }
        use std::io::Write;
        let bytes = vec![b'x'; 1024 * 1024];
        std::io::stdout().write_all(&bytes).unwrap();
        std::io::stderr().write_all(&bytes).unwrap();
        if mode == "fail" {
            std::process::exit(7);
        }
    }

    #[test]
    fn noisy_commands_finish_and_keep_only_a_bounded_tail() {
        for mode in ["pass", "fail"] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join(".spagitty-verification-test-mode"), mode).unwrap();
            let result = run_cancellable(
                dir.path(),
                &fixture_line(),
                &AtomicBool::new(false),
                Duration::from_secs(3),
            );
            assert_eq!(result.passed, mode == "pass", "{}", result.output);
            assert!(result.output.len() <= OUTPUT_BYTES);
            assert!(result.output.contains('x'));
        }
    }

    #[test]
    fn a_deadline_stops_a_check_promptly() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".spagitty-verification-test-mode"), "hang").unwrap();
        let result = run_cancellable(
            dir.path(),
            &fixture_line(),
            &AtomicBool::new(false),
            Duration::from_millis(50),
        );
        assert!(!result.passed);
        assert!(result.output.contains("timed out"));
        assert!(result.duration_ms < 2000);
    }

    #[test]
    fn a_running_check_can_be_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".spagitty-verification-test-mode"), "hang").unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let trigger = stop.clone();
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            trigger.store(true, Ordering::Release);
        });
        let result = run_cancellable(dir.path(), &fixture_line(), &stop, Duration::from_secs(3));
        worker.join().unwrap();
        assert!(!result.passed);
        assert!(result.output.contains("stopped"));
        assert!(result.duration_ms < 2000);
    }

    #[test]
    fn a_plain_command_splits_on_spaces() {
        assert_eq!(split("cargo test"), ["cargo", "test"]);
        assert_eq!(split("  npm   run   check  "), ["npm", "run", "check"]);
    }

    #[test]
    fn quotes_keep_an_argument_together() {
        assert_eq!(
            split(r#"./mvnw test -Dtest="Auth Service""#),
            ["./mvnw", "test", "-Dtest=Auth Service"]
        );
        assert_eq!(split("sh -c 'echo hi'"), ["sh", "-c", "echo hi"]);
    }

    #[test]
    fn an_empty_quoted_argument_survives() {
        assert_eq!(split(r#"thing --flag """#), ["thing", "--flag", ""]);
    }

    #[test]
    fn shell_operators_are_not_interpreted() {
        // They become literal arguments, which fails loudly. That is the point.
        assert_eq!(
            split("npm test && rm -rf /"),
            ["npm", "test", "&&", "rm", "-rf", "/"]
        );
    }

    #[test]
    fn a_passing_command_passes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".spagitty-verification-test-mode"), "echo").unwrap();
        let line = fixture_line();
        let result = run(dir.path(), &line);
        assert!(result.passed);
        assert!(result.output.contains("all good"));
        assert_eq!(result.command, line);
    }

    #[test]
    fn a_failing_command_keeps_what_it_said() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".spagitty-verification-test-mode"),
            "failure",
        )
        .unwrap();
        let result = run(dir.path(), &fixture_line());
        assert!(!result.passed);
        assert!(
            result.output.contains("2 tests failed"),
            "{}",
            result.output
        );
    }

    #[test]
    fn a_command_that_is_not_installed_fails_rather_than_raising() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(dir.path(), "definitely-not-a-real-program --version");
        assert!(!result.passed);
        assert!(result.output.contains("could not run"), "{}", result.output);
    }

    #[test]
    fn an_empty_command_is_a_failure_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(dir.path(), "   ");
        assert!(!result.passed);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn a_command_runs_where_it_was_told_to() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("here.txt"), "yes").unwrap();
        std::fs::write(dir.path().join(".spagitty-verification-test-mode"), "cwd").unwrap();
        assert!(run(dir.path(), &fixture_line()).passed);
    }

    #[test]
    fn only_the_end_of_a_long_output_is_kept() {
        let long: String = (0..5_000).map(|n| format!("line {n}\n")).collect();
        let cut = tail(&long);
        assert!(cut.len() <= OUTPUT_BYTES);
        assert!(cut.ends_with("line 4999\n"));
        assert!(cut.starts_with("line "), "cut mid-line: {:?}", &cut[..20]);
    }

    #[test]
    fn a_short_output_is_kept_whole() {
        assert_eq!(tail("two\nlines\n"), "two\nlines\n");
    }

    #[test]
    fn cutting_never_splits_a_character() {
        // A multi-byte character straddling the cut would panic on a naive
        // slice, which is a crash in the verification of somebody's task.
        let long = "é\n".repeat(OUTPUT_BYTES);
        let cut = tail(&long);
        assert!(cut.len() <= OUTPUT_BYTES);
    }
}

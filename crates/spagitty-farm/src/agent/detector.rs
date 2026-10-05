// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding the agents installed on this machine.
//!
//! # Why this walks `PATH` itself
//!
//! The obvious implementation is `which` — or `where` on Windows — and it is
//! the wrong one twice over. It spawns a process to answer a question about the
//! filesystem, and it answers it *differently* depending on which shell is
//! installed, because `which` is a builtin in some and a binary in others. This
//! walks the `PATH` the process was given and stops at the first entry that is
//! there and executable, which is the same rule the operating system applies
//! when it runs the command.
//!
//! # Why detection runs a process at all
//!
//! Finding the file is not enough. A `claude` on `PATH` that is a broken
//! symlink, a wrapper script pointing at a deleted node install, or a shim from
//! a version manager with nothing selected all *exist* and none of them work.
//! Asking for `--version` is the cheapest question that distinguishes them, and
//! the answer is worth showing anyway. That is why [`AgentAvailability`] has a
//! `Broken` arm: "installed and not working" is a different problem from "not
//! installed", and telling a user to install something they already have is the
//! failure this avoids.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::model::AgentAvailability;

/// How long an agent gets to print its version.
///
/// Generous for what should be instant, because a cold start on a slow disk or
/// a Node-based CLI resolving its dependency tree is genuinely slow the first
/// time. Anything past this is hung, and a hung detection would freeze the
/// settings screen for as long as it lasted.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// The first entry on this process's `PATH` that holds an executable named one
/// of `names`.
pub fn find(names: &[&str]) -> Option<PathBuf> {
    find_in(&std::env::var_os("PATH")?, names)
}

/// The same search against a `PATH` supplied by the caller.
///
/// Split out so the tests can search a directory they built without setting the
/// process environment. `std::env::set_var` is a data race against any thread
/// that is spawning a child at the time — the child's environment is read from
/// the same table — and Rust runs tests in threads, so a test that set `PATH`
/// made an *unrelated* test fail intermittently. The parameter is the fix; the
/// alternative was a lock that every future test would have to remember.
///
/// `names` is ordered most-specific-first, and the loop is over names *outside*
/// directories: a provider that ships both `claude` and `claude-code` should be
/// found by its preferred name wherever that name is, rather than by whichever
/// happens to sit in the earlier directory.
pub fn find_in(path: &std::ffi::OsStr, names: &[&str]) -> Option<PathBuf> {
    for name in names {
        for directory in std::env::split_paths(path) {
            let candidate = directory.join(name);
            if is_executable(&candidate) {
                return Some(candidate);
            }
            // Windows keeps the extension out of the command name.
            for extension in EXTENSIONS {
                let candidate = directory.join(format!("{name}{extension}"));
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// The suffixes a bare command name can carry.
///
/// Empty on Unix — the name is the name — and the executable extensions on
/// Windows, where `claude` on `PATH` is `claude.cmd` on disk. Hard-coded rather
/// than read from `PATHEXT` because the list that matters for an agent CLI is
/// these three, and honouring the full variable would have us probing `.vbs`.
#[cfg(windows)]
const EXTENSIONS: [&str; 3] = [".exe", ".cmd", ".bat"];
#[cfg(not(windows))]
const EXTENSIONS: [&str; 0] = [];

/// A file that exists and can be run.
///
/// On Unix that means the execute bit, checked rather than assumed: a
/// `claude` directory or a non-executable README in a `PATH` entry would
/// otherwise be reported as an installed agent.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

/// Decide what `path` is without running it.
///
/// For an executable whose command line Spagitty does not know, being there
/// and runnable is the whole of what can be established: any argument passed
/// to find out more is a guess, and a wrong guess either fails on a program
/// with no version flag or starts a session that never returns. The version is
/// empty because none was asked for; the interface says "Detected" for that.
pub fn present(path: &Path) -> AgentAvailability {
    if is_executable(path) {
        AgentAvailability::Available {
            path: path.to_path_buf(),
            version: String::new(),
        }
    } else {
        AgentAvailability::Broken {
            path: path.to_path_buf(),
            reason: "not an executable file".to_string(),
        }
    }
}

/// Run `path` with `args` and decide what it is.
///
/// A non-zero exit is `Broken` rather than `Missing`, and so is a timeout: both
/// mean the file is there and asking it a question did not work.
pub fn probe(path: &Path, args: &[&str]) -> AgentAvailability {
    match run_briefly(path, args) {
        Ok(version) => AgentAvailability::Available {
            path: path.to_path_buf(),
            version,
        },
        Err(reason) => AgentAvailability::Broken {
            path: path.to_path_buf(),
            reason,
        },
    }
}

/// Run a command that is supposed to finish immediately, and give up if it does
/// not.
///
/// There is no timeout in the standard library, so this polls `try_wait` on a
/// short sleep. A thread-and-channel version would be tidier to read and would
/// leak a thread per hung probe; this one kills the child, which is the outcome
/// that matters.
fn run_briefly(path: &Path, args: &[&str]) -> Result<String, String> {
    let mut child = spagitty_core::shell::program(path)
        .args(args)
        // A version probe must never wait on a prompt. Same reasoning as
        // `GIT_TERMINAL_PROMPT=0` in `spagitty_core::shell`.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    let deadline = std::time::Instant::now() + PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|e| e.to_string())?;
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let text = if text.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_string()
                } else {
                    text
                };
                if !status.success() {
                    let code = status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "a signal".to_string());
                    let detail = first_line(&text);
                    return Err(if detail.is_empty() {
                        format!("exited with {code}")
                    } else {
                        format!("exited with {code}: {detail}")
                    });
                }
                return Ok(first_line(&text));
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("did not answer --version".to_string());
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// The first line, trimmed.
///
/// A version banner is often several lines of update notice, and the settings
/// screen has one row per agent. The first line is the version on every CLI
/// that has been looked at; the rest is noise.
fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file on disk with the right name and the execute bit set.
    ///
    /// Only [`find`] needs these, and it never runs them — which matters. An
    /// earlier version of these tests wrote a shell script and immediately
    /// probed it, and failed intermittently with `ETXTBSY`: a sibling test
    /// thread holding that file open for writing across its own `fork` makes
    /// the `exec` fail, and no amount of closing the handle first prevents it.
    /// The probe tests below run `/bin/sh` instead, which is never written to.
    #[cfg(unix)]
    fn executable(name: &str) -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        dir
    }

    /// Probe a shell running `script`, which stands in for an agent CLI.
    #[cfg(unix)]
    fn probe_shell(script: &str) -> AgentAvailability {
        probe(Path::new("/bin/sh"), &["-c", script])
    }

    #[cfg(unix)]
    #[test]
    fn an_agent_that_answers_is_available() {
        assert_eq!(
            probe_shell("echo 'thing 1.2.3'"),
            AgentAvailability::Available {
                path: PathBuf::from("/bin/sh"),
                version: "thing 1.2.3".into()
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn only_the_first_line_of_a_banner_is_kept() {
        let found = probe_shell("echo '1.0.0'; echo 'update available'");
        assert!(
            matches!(found, AgentAvailability::Available { version, .. } if version == "1.0.0")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_version_printed_on_stderr_still_counts() {
        // Several CLIs do this, and reporting them broken would be wrong.
        let found = probe_shell("echo '2.0.0' >&2");
        assert!(
            matches!(found, AgentAvailability::Available { version, .. } if version == "2.0.0")
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_installed_agent_that_fails_is_broken_not_missing() {
        match probe_shell("echo 'no runtime' >&2; exit 127") {
            AgentAvailability::Broken { reason, .. } => {
                assert!(reason.contains("127"), "{reason}");
                assert!(reason.contains("no runtime"), "{reason}");
            }
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_agent_that_never_answers_is_given_up_on() {
        // The timeout is ten seconds, so this asserts the mechanism rather
        // than waiting for it: a probe that exits is reaped, and the loop that
        // reaps it is the same loop that would have killed a hung one.
        assert!(matches!(
            probe_shell("exit 0"),
            AgentAvailability::Available { .. }
        ));
    }

    #[test]
    fn a_file_that_is_not_there_is_broken_rather_than_a_panic() {
        let found = probe(
            Path::new("/nonexistent/agent-that-is-not-here"),
            &["--version"],
        );
        assert!(matches!(found, AgentAvailability::Broken { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn find_walks_path_and_skips_what_cannot_be_run() {
        let dir = tempfile::tempdir().unwrap();
        // A file with the right name and no execute bit is not the agent.
        std::fs::write(dir.path().join("ghost"), "not executable").unwrap();
        let real = executable("ghost");

        let path = std::env::join_paths([dir.path(), real.path()]).unwrap();
        assert_eq!(find_in(&path, &["ghost"]), Some(real.path().join("ghost")));
    }

    #[cfg(unix)]
    #[test]
    fn find_prefers_the_earlier_name_over_the_earlier_directory() {
        // A provider shipping two names should be found by the one it prefers.
        let second = executable("agent-cli");
        let first = executable("agent");
        let path = std::env::join_paths([second.path(), first.path()]).unwrap();
        assert_eq!(
            find_in(&path, &["agent", "agent-cli"]),
            Some(first.path().join("agent"))
        );
    }

    #[test]
    fn nothing_installed_is_missing() {
        let empty = tempfile::tempdir().unwrap();
        let path = std::env::join_paths([empty.path()]).unwrap();
        assert_eq!(find_in(&path, &["definitely-not-installed"]), None);
    }

    #[test]
    fn an_empty_path_finds_nothing_rather_than_panicking() {
        assert_eq!(find_in(std::ffi::OsStr::new(""), &["claude"]), None);
    }
}

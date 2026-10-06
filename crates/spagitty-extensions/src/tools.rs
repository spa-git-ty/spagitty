// SPDX-License-Identifier: GPL-3.0-or-later

//! External tools an extension declares, run by the host on its behalf.
//!
//! An extension never hands the host a command line. It names a tool and one
//! of that tool's **profiles** from its manifest, and fills in the profile's
//! typed options. The host builds argv from those — a fixed prefix, then the
//! fixed flags each option maps to — runs the executable the *user* chose or
//! that was found on `PATH` under a declared name, with no shell, in the
//! directory the host approved for the operation, inside a process tree it can
//! end.
//!
//! On Windows only `.exe` files are run: a `.bat` or `.cmd` is interpreted by
//! `cmd.exe`, whose quoting rules are how an argument becomes a command.

use std::collections::BTreeMap;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use spagitty_process::ProcessTree;

use crate::manifest::{ExternalTool, ToolOption};

/// How long a version probe may take.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(15);
/// The longest line of tool output forwarded; the rest of the line is dropped.
pub const MAX_LINE: usize = 1024 * 1024;
/// How much of a tool's stderr is kept for diagnostics.
pub const STDERR_KEPT: usize = 16 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Whether the version meets the manifest's minimum. `None` when either is
    /// unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatible: Option<bool>,
    /// Why it cannot be used, when it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// True when the path came from the user's choice rather than `PATH`.
    pub chosen: bool,
}

/// Whether `path` is something this host will run.
pub fn runnable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(windows)]
    {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(any(windows, unix)))]
    {
        true
    }
}

/// The first of `names` found on `path_var`, searched like a shell would.
pub fn search(names: &[String], path_var: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let path_var = path_var?;
    for dir in std::env::split_paths(&path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for name in names {
            let mut candidates = vec![dir.join(name)];
            if cfg!(windows) && Path::new(name).extension().is_none() {
                candidates.insert(0, dir.join(format!("{name}.exe")));
            }
            if let Some(found) = candidates.into_iter().find(|c| runnable(c)) {
                return Some(found);
            }
        }
    }
    None
}

/// The first `x.y.z` in a tool's version output.
pub fn parse_version(output: &str) -> Option<semver::Version> {
    output
        .split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')')
        .map(|word| word.trim_start_matches('v'))
        .find_map(|word| semver::Version::parse(word).ok())
}

/// Find a tool and ask it its version.
pub fn detect(tool: &ExternalTool, chosen: Option<&Path>) -> Detected {
    let (path, from_choice) = match chosen {
        Some(path) if runnable(path) => (Some(path.to_path_buf()), true),
        Some(path) => {
            return Detected {
                found: false,
                path: Some(path.to_path_buf()),
                reason: Some(format!(
                    "{} is not a program this computer can run.",
                    path.display()
                )),
                chosen: true,
                ..Detected::default()
            }
        }
        None => (
            search(&tool.executable_names, std::env::var_os("PATH")),
            false,
        ),
    };
    let Some(path) = path else {
        return Detected {
            found: false,
            reason: Some(format!(
                "{} is not installed, or not on PATH. Install it, or choose where it is.",
                tool.label()
            )),
            ..Detected::default()
        };
    };

    let mut detected = Detected {
        found: true,
        path: Some(path.clone()),
        chosen: from_choice,
        ..Detected::default()
    };
    if tool.version_args.is_empty() {
        return detected;
    }
    let mut out = String::new();
    let outcome = run(
        &path,
        &tool.version_args,
        None,
        &Arc::new(AtomicBool::new(false)),
        Some(VERSION_TIMEOUT),
        &mut |line| {
            if out.len() < 4096 {
                out.push_str(line);
                out.push('\n');
            }
        },
    );
    match outcome {
        Ok(outcome) if outcome.exit_code == Some(0) => {
            let version = parse_version(&out);
            detected.version = version.as_ref().map(|v| v.to_string());
            if let Some(minimum) = tool
                .minimum_version
                .as_deref()
                .and_then(|m| semver::Version::parse(m).ok())
            {
                detected.compatible = version.as_ref().map(|v| v >= &minimum);
                match &version {
                    Some(v) if v < &minimum => {
                        detected.reason = Some(format!(
                            "{} {v} is older than {minimum}, the oldest version this extension supports. Update it.",
                            tool.label()
                        ))
                    }
                    None => {
                        detected.reason = Some(format!("{} did not say which version it is.", tool.label()))
                    }
                    _ => {}
                }
            }
        }
        Ok(outcome) => {
            detected.reason = Some(format!(
                "{} did not answer its version check (exit {}).",
                tool.label(),
                outcome
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "none".into())
            ))
        }
        Err(error) => {
            detected.reason = Some(format!("{} could not be started: {error}", tool.label()))
        }
    }
    detected
}

/// Whether `text` may be passed as a revision: a branch, tag or commit name
/// that cannot be read as an option, a range or a path traversal.
pub fn valid_revision(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 255
        && !text.starts_with('-')
        && !text.contains("..")
        && !text.contains("@{")
        && !text.ends_with('.')
        && !text.ends_with('/')
        && !text.ends_with(".lock")
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'))
}

/// A full commit id, SHA-1 or SHA-256.
pub fn valid_commit(text: &str) -> bool {
    (text.len() == 40 || text.len() == 64)
        && text
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// argv for one profile run, after the executable.
pub fn build_args(
    tool: &ExternalTool,
    profile: &str,
    options: &BTreeMap<String, Value>,
) -> Result<Vec<String>, String> {
    let profile = tool
        .profile(profile)
        .ok_or_else(|| format!("{} has no profile called {profile}", tool.label()))?;
    for name in options.keys() {
        if !profile.options.contains_key(name) {
            return Err(format!(
                "the {} profile has no option called {name}",
                profile.id
            ));
        }
    }
    let mut args = profile.args.clone();
    for (name, option) in &profile.options {
        let value = options.get(name).filter(|v| !v.is_null());
        let Some(value) = value else {
            if option.required() {
                return Err(format!("the {name} option is required"));
            }
            continue;
        };
        let text = value
            .as_str()
            .ok_or_else(|| format!("the {name} option must be a string"))?;
        match option {
            ToolOption::Enum { values, .. } => {
                let mapped = values
                    .get(text)
                    .ok_or_else(|| format!("{text} is not a value of the {name} option"))?;
                args.extend(mapped.iter().cloned());
            }
            ToolOption::Revision { flag, .. } => {
                if !valid_revision(text) {
                    return Err(format!("{text:?} is not a revision this host will pass on"));
                }
                args.push(flag.clone());
                args.push(text.to_string());
            }
            ToolOption::Commit { flag, .. } => {
                if !valid_commit(text) {
                    return Err(format!("{text:?} is not a full commit id"));
                }
                args.push(flag.clone());
                args.push(text.to_string());
            }
        }
    }
    Ok(args)
}

/// How a tool run ended.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub exit_code: Option<i32>,
    /// Ended by a signal or by the host, rather than by exiting.
    pub signalled: bool,
    pub cancelled: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    /// The end of what it printed on stderr, redacted.
    #[serde(skip)]
    pub stderr: String,
}

/// Run `program` with `args` in `cwd`, handing each stdout line to `on_line`
/// as it arrives. Blocks until the process ends; setting `cancel`, or the
/// `timeout` passing, ends its whole tree.
pub fn run(
    program: &Path,
    args: &[String],
    cwd: Option<&Path>,
    cancel: &Arc<AtomicBool>,
    timeout: Option<Duration>,
    on_line: &mut dyn FnMut(&str),
) -> std::io::Result<Outcome> {
    let started = Instant::now();
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
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

    let stderr_tail = Arc::new(Mutex::new(String::new()));
    let stderr_reader = child.stderr.take().map(|mut stderr| {
        let tail = stderr_tail.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while let Ok(read) = stderr.read(&mut buffer) {
                if read == 0 {
                    break;
                }
                let mut tail = tail.lock().expect("stderr tail");
                tail.push_str(&String::from_utf8_lossy(&buffer[..read]));
                if tail.len() > STDERR_KEPT * 2 {
                    let cut = tail.len() - STDERR_KEPT;
                    let cut = (cut..tail.len())
                        .find(|i| tail.is_char_boundary(*i))
                        .unwrap_or(tail.len());
                    tail.drain(..cut);
                }
            }
        })
    });

    // The watchdog ends the tree on cancellation or timeout, which closes
    // stdout and so ends the read loop below. It stops when `done` is set.
    let done = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let timed_out = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let (tree, done, cancel) = (tree.clone(), done.clone(), cancel.clone());
        let (cancelled, timed_out) = (cancelled.clone(), timed_out.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                if cancel.load(Ordering::Acquire) {
                    cancelled.store(true, Ordering::Release);
                    tree.terminate();
                    return;
                }
                if timeout.is_some_and(|t| started.elapsed() > t) {
                    timed_out.store(true, Ordering::Release);
                    tree.terminate();
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        })
    };

    if let Some(stdout) = child.stdout.take() {
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = Vec::new();
        loop {
            line.clear();
            match read_line_bounded(&mut reader, &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    // Output that arrives after a cancellation belongs to an
                    // operation that is over.
                    if cancel.load(Ordering::Acquire) {
                        continue;
                    }
                    let text = String::from_utf8_lossy(&line);
                    on_line(text.trim_end_matches(['\n', '\r']));
                }
            }
        }
    }
    let status = child.wait()?;
    done.store(true, Ordering::Release);
    let _ = watchdog.join();
    // Descendants that outlived the parent go with it.
    tree.terminate();
    if let Some(reader) = stderr_reader {
        let _ = reader.join();
    }

    let cancelled = cancelled.load(Ordering::Acquire) || cancel.load(Ordering::Acquire);
    let timed_out = timed_out.load(Ordering::Acquire);
    let stderr = crate::redact::redact(&stderr_tail.lock().expect("stderr tail"));
    Ok(Outcome {
        exit_code: if cancelled || timed_out {
            None
        } else {
            status.code()
        },
        signalled: status.code().is_none() || cancelled || timed_out,
        cancelled,
        timed_out,
        duration_ms: started.elapsed().as_millis() as u64,
        stderr,
    })
}

/// Read one line of at most [`MAX_LINE`] bytes, discarding the rest of a
/// longer one.
fn read_line_bounded(reader: &mut impl BufRead, line: &mut Vec<u8>) -> std::io::Result<usize> {
    let mut total = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(total);
        }
        let (take, found) = match available.iter().position(|b| *b == b'\n') {
            Some(at) => (at + 1, true),
            None => (available.len(), false),
        };
        let room = MAX_LINE.saturating_sub(line.len());
        line.extend_from_slice(&available[..take.min(room)]);
        reader.consume(take);
        total += take;
        if found {
            return Ok(total);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Profile;

    fn tool() -> ExternalTool {
        let mut options = BTreeMap::new();
        options.insert(
            "scope".to_string(),
            ToolOption::Enum {
                values: [
                    ("committed".to_string(), vec!["--committed".to_string()]),
                    (
                        "untracked".to_string(),
                        vec![
                            "--uncommitted".to_string(),
                            "--include-untracked".to_string(),
                        ],
                    ),
                ]
                .into(),
                required: true,
            },
        );
        options.insert(
            "base".to_string(),
            ToolOption::Revision {
                flag: "--base".into(),
                required: false,
            },
        );
        options.insert(
            "baseCommit".to_string(),
            ToolOption::Commit {
                flag: "--base-commit".into(),
                required: false,
            },
        );
        ExternalTool {
            id: "cr".into(),
            name: Some("Reviewer".into()),
            executable_names: vec!["definitely-not-installed-tool".into()],
            version_args: vec!["--version".into()],
            minimum_version: Some("0.7.7".into()),
            install_url: None,
            profiles: vec![Profile {
                id: "review".into(),
                args: vec!["review".into(), "--agent".into()],
                options,
                workdir: None,
                timeout_ms: None,
            }],
        }
    }

    fn opts(pairs: &[(&str, &str)]) -> BTreeMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(*v)))
            .collect()
    }

    #[test]
    fn argv_is_built_from_the_profile_and_nothing_else() {
        let args = build_args(
            &tool(),
            "review",
            &opts(&[("scope", "untracked"), ("base", "feature/x")]),
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "review",
                "--agent",
                "--base",
                "feature/x",
                "--uncommitted",
                "--include-untracked"
            ]
        );
    }

    #[test]
    fn values_that_could_become_options_or_commands_are_refused() {
        for base in [
            "--output=/etc/passwd",
            "-x",
            "a b",
            "a;rm",
            "main..evil",
            "x@{1}",
            "$(id)",
            "`id`",
            "a\nb",
            "",
        ] {
            let result = build_args(
                &tool(),
                "review",
                &opts(&[("scope", "committed"), ("base", base)]),
            );
            assert!(result.is_err(), "{base:?} was passed through");
        }
        assert!(build_args(&tool(), "review", &opts(&[("scope", "everything")])).is_err());
        assert!(build_args(
            &tool(),
            "review",
            &opts(&[("scope", "committed"), ("extra", "x")])
        )
        .is_err());
        assert!(
            build_args(&tool(), "review", &opts(&[])).is_err(),
            "required option"
        );
        assert!(
            build_args(&tool(), "deploy", &opts(&[])).is_err(),
            "undeclared profile"
        );
        assert!(build_args(
            &tool(),
            "review",
            &opts(&[("scope", "committed"), ("baseCommit", "abc")])
        )
        .is_err());
        let full = "a".repeat(40);
        assert!(build_args(
            &tool(),
            "review",
            &opts(&[("scope", "committed"), ("baseCommit", &full)])
        )
        .is_ok());
    }

    #[test]
    fn versions_are_read_out_of_whatever_a_tool_prints() {
        assert_eq!(parse_version("0.8.1\n").unwrap().to_string(), "0.8.1");
        assert_eq!(
            parse_version("coderabbit v0.7.7 (build 12)")
                .unwrap()
                .to_string(),
            "0.7.7"
        );
        assert!(parse_version("no version here").is_none());
    }

    #[test]
    fn a_missing_tool_is_reported_with_what_to_do() {
        let detected = detect(&tool(), None);
        assert!(!detected.found);
        assert!(detected.reason.unwrap().contains("not installed"));
        let detected = detect(&tool(), Some(Path::new("/definitely/not/here")));
        assert!(!detected.found && detected.chosen);
    }

    #[test]
    fn path_search_finds_the_first_runnable_name() {
        let dir = tempfile::tempdir().unwrap();
        let exe = std::env::current_exe().unwrap();
        let name = if cfg!(windows) {
            "fake-tool.exe"
        } else {
            "fake-tool"
        };
        std::fs::copy(&exe, dir.path().join(name)).unwrap();
        let found = search(
            &["missing".into(), "fake-tool".into()],
            Some(dir.path().as_os_str().to_owned()),
        );
        assert_eq!(found.unwrap().file_name().unwrap(), name);
        assert!(search(&["fake-tool".into()], None).is_none());
    }

    #[test]
    fn revisions_and_commits_are_validated_strictly() {
        assert!(valid_revision("main"));
        assert!(valid_revision("origin/feature/x-1.2"));
        assert!(!valid_revision("main.lock"));
        assert!(valid_commit(&"0123456789abcdef".repeat(4)[..40]));
        assert!(!valid_commit(&"A".repeat(40)));
        assert!(!valid_commit("abc"));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! A command-line agent, run once per step.
//!
//! The farm's adapters already know each provider's command line; this asks
//! them for it, adds the provider's read-only mode where it has one, and runs
//! it in the assignment's scratch worktree through the farm's own process
//! runner — the same stream capture, narration and cancellation a farm task
//! gets. A custom agent can promise no read-only mode, which is why the
//! engine checks the worktree after every step rather than trusting any of
//! them ([`super::guard`]).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::engine::{Ask, Driver, Reply, World};
use super::protocol::Answer;
use crate::agent::{adapter_for, AgentCommand, AgentRunRequest};
use crate::execution::log::TranscriptWriter;
use crate::execution::narrate::Narrator;
use crate::execution::process::{self, Ended, Sink};
use crate::model::{AgentDefinition, AgentProvider};

/// How many of the agent's last lines a failure quotes from.
const LAST_LINES: usize = 6;

pub struct LocalDriver {
    definition: AgentDefinition,
    /// Where each step's transcript is written.
    logs: PathBuf,
    runs: usize,
}

impl LocalDriver {
    pub fn new(definition: AgentDefinition, logs: PathBuf) -> LocalDriver {
        LocalDriver {
            definition,
            logs,
            runs: 0,
        }
    }
}

/// The provider's own read-only mode, where it has one. Keep options outside
/// the prompt, whether it is a positional argument, a flag's value or stdin.
pub fn read_only(provider: AgentProvider, mut command: AgentCommand) -> AgentCommand {
    // Settings may explicitly opt Codex into Full Access. Do not add a
    // conflicting sandbox argument to that saved choice.
    if provider == AgentProvider::Codex
        && command.args.iter().any(|arg| {
            arg == "--sandbox"
                || arg == "-s"
                || arg.starts_with("--sandbox=")
                || arg == "--dangerously-bypass-approvals-and-sandbox"
        })
    {
        return command;
    }
    let flags: &[&str] = match provider {
        AgentProvider::ClaudeCode => &["--permission-mode", "plan"],
        AgentProvider::Codex => &["--sandbox", "read-only"],
        AgentProvider::Agy => &["--mode", "plan"],
        AgentProvider::OhMyPi => &["--tools", "read,grep,glob"],
        _ => &[],
    };
    if flags.is_empty() {
        return command;
    }
    let at = match provider {
        // `codex exec` must stay first: the flags belong to the subcommand.
        AgentProvider::Codex => 1.min(command.args.len()),
        AgentProvider::Agy => 0,
        _ if command.stdin.is_some() => command.args.len(),
        _ => command.args.len().saturating_sub(1),
    };
    for (offset, flag) in flags.iter().enumerate() {
        command.args.insert(at + offset, (*flag).to_string());
    }
    command
}

/// The provider's narrator, keeping the raw stream as well: the answer block
/// may be inside a machine event the narration leaves out.
#[derive(Debug)]
struct Capture {
    inner: Box<dyn Narrator>,
    raw: Arc<Mutex<String>>,
}

impl Narrator for Capture {
    fn narrate(&mut self, raw: &str) -> Vec<String> {
        if let Ok(mut kept) = self.raw.lock() {
            kept.push_str(raw);
            kept.push('\n');
        }
        self.inner.narrate(raw)
    }
}

#[derive(Debug)]
struct Lines(Mutex<mpsc::Sender<String>>);

impl Sink for Lines {
    fn line(&self, text: &str) {
        if let Ok(sender) = self.0.lock() {
            let _ = sender.send(text.to_string());
        }
    }
}

impl Driver for LocalDriver {
    fn run(
        &mut self,
        ask: &Ask,
        _world: &dyn World,
        hear: &mut dyn FnMut(&str),
        cancel: &Arc<AtomicBool>,
    ) -> Result<Reply, String> {
        let adapter = adapter_for(self.definition.provider);
        let request = AgentRunRequest {
            workdir: ask.workdir.clone(),
            prompt: ask.prompt.clone(),
            unattended: false,
        };
        let command = read_only(
            self.definition.provider,
            adapter.command(&self.definition, &request),
        );
        let display = command.display();
        hear(&format!("$ {}", shorten(&display)));

        self.runs += 1;
        let transcript =
            TranscriptWriter::create(&self.logs.join(format!("run-{}.log", self.runs)))
                .map_err(|error| error.to_string())?;
        let raw = Arc::new(Mutex::new(String::new()));
        let narrator = Box::new(Capture {
            inner: adapter.narrator(),
            raw: raw.clone(),
        });
        let (sender, receiver) = mpsc::channel();
        let sink: Arc<dyn Sink> = Arc::new(Lines(Mutex::new(sender)));
        let session = process::start(&command, &ask.workdir, transcript, sink, narrator)
            .map_err(|error| error.to_string())?;
        let cancellation = session.cancellation();

        let mut last: Vec<String> = Vec::new();
        loop {
            if cancel.load(Ordering::Acquire) {
                cancellation.cancel();
            }
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => {
                    hear(&line);
                    last.push(line);
                    if last.len() > LAST_LINES {
                        last.remove(0);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let ended = session.wait();
        let raw = raw.lock().map(|text| text.clone()).unwrap_or_default();
        if self.definition.provider == AgentProvider::Codex {
            if let Some(reason) = codex_launch_failure(&raw) {
                return Err(reason);
            }
        }
        if self.definition.provider == AgentProvider::Agy {
            if let Some(reason) = agy_permission_failure(&raw) {
                return Err(reason);
            }
        }
        match ended {
            Ended::Ok => Ok(Reply {
                answer: Answer::find(&raw),
                last_words: last.join("\n"),
                tokens: Default::default(),
                command: Some(display),
                sent: Vec::new(),
            }),
            Ended::Cancelled => Err("was stopped".into()),
            Ended::Failed {
                code: Some(code), ..
            } => Err(format!("exited with {code}: {}", last.join("\n"))),
            Ended::Failed { message, .. } => Err(message),
        }
    }
}

/// Codex can exit zero after its shell failed to start and still emit an
/// answer. A broken sandbox must stop the step, not become a clean review.
fn codex_launch_failure(raw: &str) -> Option<String> {
    raw.lines().find(|line| line.contains("setup refresh had errors") &&
        (line.contains("ERROR codex_core") || line.starts_with("Failed to create unified exec process")))
        .map(|line| format!("Codex could not start its Windows sandbox: {line}\nRepair the Codex sandbox or select Full Access in Settings > Agents > Codex, then resume."))
}

fn agy_permission_failure(raw: &str) -> Option<String> {
    raw.lines()
        .find(|line| line.starts_with("jetski: no output produced") && line.contains("headless mode cannot prompt"))
        .map(|line| format!("{line}\nConfigure agy's permissions.allow rules or enable Auto-approve tools in Settings > Agents > agy, then resume."))
}

/// The command line as the timeline shows it: the prompt is long and is in
/// the transcript already.
fn shorten(line: &str) -> String {
    if line.chars().count() <= 160 {
        return line.to_string();
    }
    let head: String = line.chars().take(157).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_headless_agy_permission_denial_is_a_failed_step() {
        let raw = "jetski: no output produced — a tool required the command permission that headless mode cannot prompt for, so it was auto-denied.";
        assert!(agy_permission_failure(raw)
            .unwrap()
            .contains("Auto-approve tools"));
        assert!(
            agy_permission_failure("An ordinary finding about headless mode cannot prompt")
                .is_none()
        );
    }

    #[test]
    fn a_zero_exit_with_a_broken_sandbox_is_a_failed_step() {
        let raw = "ERROR codex_core::tools::router: error=exec_command failed: setup refresh had errors\nNo findings.";
        assert!(codex_launch_failure(raw)
            .unwrap()
            .contains("Settings > Agents > Codex"));
        assert!(codex_launch_failure("The file discusses setup refresh had errors").is_none());
    }

    fn command(args: &[&str]) -> AgentCommand {
        AgentCommand {
            program: PathBuf::from("x"),
            args: args.iter().map(|a| a.to_string()).collect(),
            stdin: None,
        }
    }

    #[test]
    fn claude_code_reviews_in_plan_mode_with_the_prompt_last() {
        let out = read_only(
            AgentProvider::ClaudeCode,
            command(&["-p", "--verbose", "PROMPT"]),
        );
        assert_eq!(
            out.args,
            ["-p", "--verbose", "--permission-mode", "plan", "PROMPT"]
        );
    }

    #[test]
    fn codex_keeps_exec_first_and_reads_only() {
        let out = read_only(AgentProvider::Codex, command(&["exec", "PROMPT"]));
        assert_eq!(out.args, ["exec", "--sandbox", "read-only", "PROMPT"]);
    }

    #[test]
    fn codex_full_access_is_not_overridden_by_review_mode() {
        let out = read_only(
            AgentProvider::Codex,
            command(&["exec", "--sandbox", "danger-full-access", "PROMPT"]),
        );
        assert_eq!(
            out.args,
            ["exec", "--sandbox", "danger-full-access", "PROMPT"]
        );
    }

    #[test]
    fn agy_reviews_in_plan_mode() {
        let out = read_only(AgentProvider::Agy, command(&["--print", "PROMPT"]));
        assert_eq!(out.args, ["--mode", "plan", "--print", "PROMPT"]);
    }

    #[test]
    fn omp_print_mode_limits_tools_and_preserves_stdin() {
        let mut cmd = command(&["--print"]);
        cmd.stdin = Some("PROMPT".into());
        let out = read_only(AgentProvider::OhMyPi, cmd);
        assert_eq!(out.args, ["--print", "--tools", "read,grep,glob"]);
        assert_eq!(out.stdin.as_deref(), Some("PROMPT"));
    }

    #[test]
    fn a_provider_with_no_read_only_mode_is_left_as_it_is() {
        let out = read_only(AgentProvider::Custom, command(&["--diff", "PROMPT"]));
        assert_eq!(out.args, ["--diff", "PROMPT"]);
    }

    #[test]
    fn a_long_command_line_is_shortened_for_the_timeline() {
        let long = "x ".repeat(200);
        assert!(shorten(&long).ends_with('…'));
        assert_eq!(shorten("claude -p"), "claude -p");
    }
}

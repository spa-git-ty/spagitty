// SPDX-License-Identifier: GPL-3.0-or-later

//! Claude Code.
//!
//! Invoked in print mode — `claude -p <prompt>` — which runs the whole turn to
//! completion with no terminal attached and writes the transcript to standard
//! output. That is exactly the shape the farm's runner wants, and it is why
//! this is the provider the first end-to-end path was built against.
//!
//! # Why the output format is `stream-json`
//!
//! Print mode's default format writes *once*, when the turn is over. A task
//! then sits at Running for the length of a real piece of work with an empty
//! transcript behind it, which is what the farm looked like until BUG-021:
//! nothing to watch, nothing to cancel on, and no way to tell a slow run from a
//! stuck one. `--output-format stream-json` emits an event per step as it
//! happens instead, and [`crate::execution::narrate::ClaudeStream`] turns those
//! back into lines a person reads. `--verbose` is not optional: print mode
//! refuses `stream-json` without it.
//!
//! The prompt goes on the command line rather than through standard input
//! because print mode treats piped stdin as *additional context* rather than as
//! the instruction, and a farm prompt is the instruction.

use std::path::PathBuf;

use crate::agent::adapter::{prompt_needs_stdin, AgentAdapter, AgentCommand, AgentRunRequest};
use crate::execution::narrate::{ClaudeStream, Narrator};
use crate::model::{
    AgentCapability, AgentDefinition, AgentId, AgentInputMode, AgentProvider, AgentRole,
    AgentTraits,
};

pub struct ClaudeAdapter;

impl AgentAdapter for ClaudeAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::ClaudeCode
    }

    fn executables(&self) -> &'static [&'static str] {
        &["claude"]
    }

    fn default_definition(&self, executable: PathBuf) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("claude"),
            provider: AgentProvider::ClaudeCode,
            display_name: "Claude Code".into(),
            executable,
            capabilities: [
                AgentCapability::Planning,
                AgentCapability::Coding,
                AgentCapability::Review,
                AgentCapability::Testing,
                AgentCapability::Documentation,
                AgentCapability::Research,
                AgentCapability::LongContext,
                AgentCapability::ToolUse,
            ]
            .into_iter()
            .collect(),
            input_mode: AgentInputMode::CliPrompt,
            traits: AgentTraits {
                resumable_sessions: true,
                streaming: true,
                structured_output: true,
                tool_use: true,
                headless: true,
            },
            // The default reviewer, because review is the job that most wants
            // a long context and the whole diff at once.
            role: AgentRole::Architect,
            extra_args: Vec::new(),
            enabled: true,
        }
    }

    fn narrator(&self) -> Box<dyn Narrator> {
        Box::new(ClaudeStream::default())
    }

    fn command(&self, definition: &AgentDefinition, request: &AgentRunRequest) -> AgentCommand {
        let mut args = vec![
            "-p".to_string(),
            // Streamed rather than delivered at the end. See the header.
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if request.unattended {
            // The farm has already decided what this agent may do — it is
            // running in a worktree of its own, under the farm's permissions.
            // Asking again through a terminal nobody is watching would hang.
            args.push("--permission-mode".into());
            args.push("acceptEdits".into());
        }

        args.extend(definition.extra_args.iter().cloned());
        // With no prompt argument, print mode takes stdin as the instruction.
        let on_stdin = prompt_needs_stdin(&definition.executable, &request.prompt);
        if !on_stdin {
            args.push(request.prompt.clone());
        }

        AgentCommand {
            program: definition.executable.clone(),
            args,
            stdin: on_stdin.then(|| request.prompt.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn definition() -> AgentDefinition {
        ClaudeAdapter.default_definition(PathBuf::from("/usr/bin/claude"))
    }

    fn request(unattended: bool) -> AgentRunRequest {
        AgentRunRequest {
            workdir: PathBuf::from("/tmp/task-1"),
            prompt: "Do the thing".into(),
            unattended,
        }
    }

    #[test]
    fn the_prompt_is_the_last_argument() {
        let command = ClaudeAdapter.command(&definition(), &request(false));
        assert_eq!(command.args.first().unwrap(), "-p");
        assert_eq!(command.args.last().unwrap(), "Do the thing");
        assert_eq!(command.stdin, None);
    }

    #[test]
    fn the_run_streams_rather_than_reporting_once_at_the_end() {
        // BUG-021: without these three flags a run says nothing for minutes.
        let command = ClaudeAdapter.command(&definition(), &request(true));
        let joined = command.args.join(" ");
        assert!(joined.contains("--output-format stream-json"), "{joined}");
        assert!(
            command.args.iter().any(|arg| arg == "--verbose"),
            "{joined}"
        );
    }

    #[test]
    fn an_unattended_run_does_not_stop_to_ask() {
        let command = ClaudeAdapter.command(&definition(), &request(true));
        assert!(command.args.iter().any(|arg| arg == "--permission-mode"));
    }

    #[test]
    fn an_attended_run_keeps_the_default_permissions() {
        let command = ClaudeAdapter.command(&definition(), &request(false));
        assert!(!command.args.iter().any(|arg| arg == "--permission-mode"));
    }

    #[test]
    fn extra_arguments_land_before_the_prompt() {
        // After the flags and before the positional, which is the only place
        // an added flag can go without being read as part of the prompt.
        let mut definition = definition();
        definition.extra_args = vec!["--model".into(), "opus".into()];
        let command = ClaudeAdapter.command(&definition, &request(false));
        let model = command.args.iter().position(|a| a == "--model").unwrap();
        assert!(model < command.args.len() - 1);
        assert_eq!(command.args.last().unwrap(), "Do the thing");
    }

    #[test]
    fn the_default_definition_can_review() {
        // The routing rule `Review -> Claude` depends on this.
        assert!(definition().capabilities.contains(&AgentCapability::Review));
        assert!(definition().traits.headless);
    }

    #[test]
    fn detection_looks_for_the_command_people_have() {
        assert_eq!(ClaudeAdapter.executables(), &["claude"]);
        assert!(ClaudeAdapter.is_headless(&definition()));
        assert_eq!(
            ClaudeAdapter
                .command(&definition(), &request(false))
                .program,
            Path::new("/usr/bin/claude")
        );
    }

    #[test]
    fn a_windows_shim_gets_a_many_line_prompt_on_stdin() {
        let mut def = ClaudeAdapter.default_definition("C:/npm/claude.cmd".into());
        def.extra_args.clear();
        let req = AgentRunRequest {
            workdir: "/tmp".into(),
            prompt: "one\ntwo".into(),
            unattended: false,
        };
        let out = ClaudeAdapter.command(&def, &req);
        assert_eq!(out.args.iter().any(|a| a == "one\ntwo"), !cfg!(windows));
        assert_eq!(out.stdin.is_some(), cfg!(windows));
    }
}

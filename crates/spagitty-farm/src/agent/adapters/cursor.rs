// SPDX-License-Identifier: GPL-3.0-or-later

//! Cursor.
//!
//! The command is `cursor-agent`, not `cursor`: `cursor` is the editor, and
//! running it here would open a window rather than do the work. Both names are
//! searched, preferred name first, so a machine with only the editor installed
//! reports the agent as missing rather than as an agent that opens a GUI.
//!
//! `-p` is its print mode, the same idea as Claude Code's.

use std::path::PathBuf;

use crate::agent::adapter::{prompt_needs_stdin, AgentAdapter, AgentCommand, AgentRunRequest};
use crate::model::{
    AgentCapability, AgentDefinition, AgentId, AgentInputMode, AgentProvider, AgentRole,
    AgentTraits,
};

pub struct CursorAdapter;

impl AgentAdapter for CursorAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Cursor
    }

    fn executables(&self) -> &'static [&'static str] {
        &["cursor-agent"]
    }

    fn default_definition(&self, executable: PathBuf) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("cursor"),
            provider: AgentProvider::Cursor,
            display_name: "Cursor".into(),
            executable,
            capabilities: [
                AgentCapability::Coding,
                AgentCapability::Frontend,
                AgentCapability::Testing,
                AgentCapability::ToolUse,
            ]
            .into_iter()
            .collect(),
            input_mode: AgentInputMode::CliPrompt,
            traits: AgentTraits {
                resumable_sessions: false,
                streaming: true,
                structured_output: false,
                tool_use: true,
                headless: true,
            },
            role: AgentRole::Frontend,
            extra_args: Vec::new(),
            enabled: true,
        }
    }

    fn command(&self, definition: &AgentDefinition, request: &AgentRunRequest) -> AgentCommand {
        let mut args = vec!["-p".to_string()];
        if request.unattended {
            args.push("--force".into());
        }
        args.extend(definition.extra_args.iter().cloned());
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

    #[test]
    fn the_editor_is_not_mistaken_for_the_agent() {
        assert_eq!(CursorAdapter.executables(), &["cursor-agent"]);
        assert!(!CursorAdapter.executables().contains(&"cursor"));
    }

    #[test]
    fn print_mode_and_the_prompt_bracket_everything_else() {
        let definition = CursorAdapter.default_definition(PathBuf::from("cursor-agent"));
        let command = CursorAdapter.command(
            &definition,
            &AgentRunRequest {
                workdir: PathBuf::from("/tmp/t"),
                prompt: "Build the login screen".into(),
                unattended: true,
            },
        );
        assert_eq!(command.args.first().unwrap(), "-p");
        assert_eq!(command.args.last().unwrap(), "Build the login screen");
        assert!(command.args.iter().any(|a| a == "--force"));
    }

    #[test]
    fn the_default_role_is_the_one_the_routing_table_names() {
        let definition = CursorAdapter.default_definition(PathBuf::from("cursor-agent"));
        assert_eq!(definition.role, AgentRole::Frontend);
        assert!(definition.capabilities.contains(&AgentCapability::Frontend));
    }
}

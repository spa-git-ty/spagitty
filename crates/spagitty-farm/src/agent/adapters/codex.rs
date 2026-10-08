// SPDX-License-Identifier: GPL-3.0-or-later

//! Codex.
//!
//! `codex exec <prompt>` is the non-interactive subcommand: it runs the turn
//! and exits, which is what the farm needs. The interactive `codex` with no
//! subcommand opens a full-screen session and would hang with no terminal
//! attached — so the subcommand is not optional here, and is added before
//! anything the user configured.

use std::path::PathBuf;

use crate::agent::adapter::{AgentAdapter, AgentCommand, AgentRunRequest};
use crate::model::{
    AgentCapability, AgentDefinition, AgentId, AgentInputMode, AgentProvider, AgentRole,
    AgentTraits,
};

pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Codex
    }

    fn executables(&self) -> &'static [&'static str] {
        &["codex"]
    }

    fn default_definition(&self, executable: PathBuf) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("codex"),
            provider: AgentProvider::Codex,
            display_name: "Codex".into(),
            executable,
            capabilities: [
                AgentCapability::Coding,
                AgentCapability::Testing,
                AgentCapability::Review,
                AgentCapability::Backend,
                AgentCapability::ToolUse,
            ]
            .into_iter()
            .collect(),
            input_mode: AgentInputMode::CliPrompt,
            traits: AgentTraits {
                resumable_sessions: true,
                streaming: true,
                structured_output: false,
                tool_use: true,
                headless: true,
            },
            role: AgentRole::Backend,
            extra_args: Vec::new(),
            enabled: true,
        }
    }

    fn command(&self, definition: &AgentDefinition, request: &AgentRunRequest) -> AgentCommand {
        let mut args = vec![
            "exec".to_string(),
            "--skip-git-repo-check".into(),
            "--cd".into(),
            request.workdir.to_string_lossy().into_owned(),
            "-c".into(),
            "approval_policy=\"never\"".into(),
        ];

        if request.unattended {
            // Headless execution cannot answer approval prompts. Keep the
            // sandbox; a worktree alone is not an OS security boundary.
            args.extend(["--sandbox".into(), "workspace-write".into()]);
        }

        args.extend(definition.extra_args.iter().cloned());
        args.push(request.prompt.clone());

        AgentCommand {
            program: definition.executable.clone(),
            args,
            stdin: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> AgentDefinition {
        CodexAdapter.default_definition(PathBuf::from("codex"))
    }

    fn request(unattended: bool) -> AgentRunRequest {
        AgentRunRequest {
            workdir: PathBuf::from("/tmp/task-2"),
            prompt: "Implement rotation".into(),
            unattended,
        }
    }

    #[test]
    fn the_non_interactive_subcommand_comes_first() {
        // Without it the process opens a full-screen session and never exits.
        let command = CodexAdapter.command(&definition(), &request(false));
        assert_eq!(command.args.first().unwrap(), "exec");
    }

    #[test]
    fn a_user_flag_cannot_displace_the_subcommand() {
        let mut definition = definition();
        definition.extra_args = vec!["--model".into(), "o4".into()];
        let command = CodexAdapter.command(&definition, &request(true));
        assert_eq!(command.args.first().unwrap(), "exec");
        assert_eq!(command.args.last().unwrap(), "Implement rotation");
    }

    #[test]
    fn an_unattended_run_asks_for_no_approvals() {
        assert!(CodexAdapter
            .command(&definition(), &request(true))
            .args
            .iter()
            .any(|arg| arg == "workspace-write"));
        assert!(!CodexAdapter
            .command(&definition(), &request(false))
            .args
            .iter()
            .any(|arg| arg == "workspace-write"));
    }

    #[test]
    fn temporary_directories_and_scratch_worktrees_can_start_headless() {
        let out = CodexAdapter.command(&definition(), &request(false));
        assert!(out.args.iter().any(|arg| arg == "--skip-git-repo-check"));
        assert!(out
            .args
            .windows(2)
            .any(|args| args == ["--cd", "/tmp/task-2"]));
        assert!(out
            .args
            .iter()
            .any(|arg| arg == "approval_policy=\"never\""));
        assert!(!out.args.iter().any(|arg| arg == "--full-auto"));
        assert!(!out.args.iter().any(|arg| arg == "danger-full-access"));
    }

    #[test]
    fn the_default_role_is_the_one_the_routing_table_names() {
        assert_eq!(definition().role, AgentRole::Backend);
        assert_eq!(definition().provider.slug(), "codex");
    }
}

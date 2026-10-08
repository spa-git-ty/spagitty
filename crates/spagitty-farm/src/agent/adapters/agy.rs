// SPDX-License-Identifier: GPL-3.0-or-later

//! agy CLI: print mode runs one turn without an interactive terminal.

use crate::agent::{AgentAdapter, AgentCommand, AgentRunRequest};
use crate::model::{
    AgentCapability, AgentDefinition, AgentId, AgentInputMode, AgentProvider, AgentRole,
    AgentTraits,
};
use std::path::PathBuf;

pub struct AgyAdapter;

impl AgentAdapter for AgyAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::Agy
    }
    fn executables(&self) -> &'static [&'static str] {
        &["agy"]
    }
    fn default_definition(&self, executable: PathBuf) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("agy"),
            provider: AgentProvider::Agy,
            display_name: "agy".into(),
            executable,
            capabilities: [
                AgentCapability::Coding,
                AgentCapability::Review,
                AgentCapability::Testing,
                AgentCapability::ToolUse,
            ]
            .into_iter()
            .collect(),
            input_mode: AgentInputMode::CliPrompt,
            traits: AgentTraits {
                resumable_sessions: true,
                streaming: false,
                structured_output: false,
                tool_use: true,
                headless: true,
            },
            role: AgentRole::General,
            extra_args: Vec::new(),
            enabled: true,
        }
    }
    fn command(&self, definition: &AgentDefinition, request: &AgentRunRequest) -> AgentCommand {
        let mut args = Vec::new();
        if request.unattended
            && !definition
                .extra_args
                .iter()
                .any(|arg| arg == "--dangerously-skip-permissions")
        {
            args.push("--dangerously-skip-permissions".into());
        }
        args.extend(definition.extra_args.iter().cloned());
        // Unlike a boolean -p flag, agy's --print consumes the next argument.
        args.push("--print".into());
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
    #[test]
    fn print_mode_and_permissions_match_the_requested_run() {
        let def = AgyAdapter.default_definition("agy".into());
        let mut request = AgentRunRequest {
            workdir: "/tmp/review".into(),
            prompt: "Read the diff".into(),
            unattended: false,
        };
        assert_eq!(
            AgyAdapter.command(&def, &request).args,
            ["--print", "Read the diff"]
        );
        request.unattended = true;
        assert_eq!(
            AgyAdapter.command(&def, &request).args,
            ["--dangerously-skip-permissions", "--print", "Read the diff"]
        );
        assert_eq!(def.provider.slug(), "agy");
        assert!(def.capabilities.contains(&AgentCapability::Review));
    }

    #[test]
    fn saved_auto_approval_stays_before_the_print_prompt() {
        let mut definition = AgyAdapter.default_definition("agy".into());
        definition
            .extra_args
            .push("--dangerously-skip-permissions".into());
        let mut request = AgentRunRequest {
            workdir: "/tmp/t".into(),
            prompt: "ok".into(),
            unattended: false,
        };
        for unattended in [false, true] {
            request.unattended = unattended;
            assert_eq!(
                AgyAdapter.command(&definition, &request).args,
                ["--dangerously-skip-permissions", "--print", "ok"]
            );
        }
    }
}

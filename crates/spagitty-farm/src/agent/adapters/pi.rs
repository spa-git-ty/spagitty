// SPDX-License-Identifier: GPL-3.0-or-later

//! Oh My Pi.
//!
//! `omp --print` runs one turn and exits. Existing stdin definitions stay
//! compatible; the prompt can also be passed as a positional argument.

use std::path::PathBuf;

use crate::agent::adapter::{AgentAdapter, AgentCommand, AgentRunRequest};
use crate::model::{
    AgentCapability, AgentDefinition, AgentId, AgentInputMode, AgentProvider, AgentRole,
    AgentTraits,
};

pub struct PiAdapter;

impl AgentAdapter for PiAdapter {
    fn provider(&self) -> AgentProvider {
        AgentProvider::OhMyPi
    }

    fn executables(&self) -> &'static [&'static str] {
        &["omp", "pi", "ohmypi"]
    }

    fn default_definition(&self, executable: PathBuf) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("pi"),
            provider: AgentProvider::OhMyPi,
            display_name: "Oh My Pi".into(),
            executable,
            capabilities: [
                AgentCapability::Coding,
                AgentCapability::Review,
                AgentCapability::Research,
                AgentCapability::Documentation,
            ]
            .into_iter()
            .collect(),
            input_mode: AgentInputMode::Stdin,
            traits: AgentTraits {
                resumable_sessions: false,
                streaming: true,
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
        // The definition decides, not this function: a user who has switched
        // the input mode gets what they asked for rather than what this
        // provider shipped with.
        let on_stdin = definition.input_mode == AgentInputMode::Stdin;
        let mut args = vec!["--print".into()];
        if request.unattended {
            args.push("--auto-approve".into());
        }
        args.extend(definition.extra_args.clone());
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

    fn definition() -> AgentDefinition {
        PiAdapter.default_definition(PathBuf::from("pi"))
    }

    fn request() -> AgentRunRequest {
        AgentRunRequest {
            workdir: PathBuf::from("/tmp/t"),
            prompt: "Write the docs".into(),
            unattended: true,
        }
    }

    #[test]
    fn the_prompt_goes_down_the_pipe_by_default() {
        let command = PiAdapter.command(&definition(), &request());
        assert_eq!(command.stdin.as_deref(), Some("Write the docs"));
        assert_eq!(command.args, ["--print", "--auto-approve"]);
    }

    #[test]
    fn switching_the_input_mode_moves_the_prompt_to_the_command_line() {
        let mut definition = definition();
        definition.input_mode = AgentInputMode::CliPrompt;
        let command = PiAdapter.command(&definition, &request());
        assert_eq!(command.stdin, None);
        assert_eq!(command.args.last().unwrap(), "Write the docs");
    }

    #[test]
    fn omp_is_preferred_to_legacy_names() {
        assert_eq!(PiAdapter.executables(), &["omp", "pi", "ohmypi"]);
        assert!(definition().capabilities.contains(&AgentCapability::Review));
    }
}

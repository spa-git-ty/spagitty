// SPDX-License-Identifier: GPL-3.0-or-later

//! Oh My Pi.
//!
//! `omp --print` runs one turn and exits. Existing stdin definitions stay
//! compatible; the prompt can also be passed as a positional argument.

use std::path::PathBuf;

use crate::agent::adapter::{prompt_needs_stdin, AgentAdapter, AgentCommand, AgentRunRequest};
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
        let on_stdin = definition.input_mode == AgentInputMode::Stdin
            || prompt_needs_stdin(&definition.executable, &request.prompt);
        let mut args = vec!["--print".into()];
        // `--auto-approve` is Oh My Pi's. The plain `pi` it grew out of is found
        // by this adapter too and names its approval flags differently, so it
        // is given none it may not know.
        if request.unattended && !is_plain_pi(&definition.executable) {
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

/// Whether the executable is `pi` itself and not Oh My Pi (`omp`, `ohmypi`).
fn is_plain_pi(executable: &std::path::Path) -> bool {
    executable
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case("pi"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> AgentDefinition {
        PiAdapter.default_definition(PathBuf::from("omp"))
    }

    #[test]
    fn plain_pi_is_not_given_oh_my_pis_approval_flag() {
        for executable in ["pi", "/usr/local/bin/pi", "C:/npm/pi.cmd"] {
            let command = PiAdapter.command(
                &PiAdapter.default_definition(PathBuf::from(executable)),
                &request(),
            );
            assert_eq!(command.args, ["--print"], "{executable}");
        }
        let omp = PiAdapter.command(
            &PiAdapter.default_definition(PathBuf::from("/opt/ohmypi")),
            &request(),
        );
        assert_eq!(omp.args, ["--print", "--auto-approve"]);
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

    #[test]
    fn a_windows_shim_gets_a_many_line_prompt_on_stdin() {
        let def = PiAdapter.default_definition("C:/npm/omp.cmd".into());
        let req = AgentRunRequest {
            workdir: "/tmp".into(),
            prompt: "one\ntwo".into(),
            unattended: false,
        };
        let out = PiAdapter.command(&def, &req);
        assert_eq!(
            out.stdin.is_some(),
            cfg!(windows) || def.input_mode == AgentInputMode::Stdin
        );
    }
}

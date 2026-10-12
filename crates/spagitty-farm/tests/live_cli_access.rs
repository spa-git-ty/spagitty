// SPDX-License-Identifier: GPL-3.0-or-later

//! Opt-in checks against installed and authenticated CLIs. No repository edits.

use spagitty_farm::agent::{adapter_for, AgentRunRequest};
use spagitty_farm::assign::local::read_only;
use spagitty_farm::execution::log::TranscriptWriter;
use spagitty_farm::execution::process::{self, Collected, Ended};
use spagitty_farm::model::AgentProvider;
use std::sync::Arc;
use std::time::Duration;

fn reads_file(provider: AgentProvider, expanded_access: bool) {
    let adapter = adapter_for(provider);
    let availability = adapter.detect();
    assert!(availability.is_available(), "{availability:?}");
    let dir = tempfile::tempdir().unwrap();
    let marker = format!(
        "SPAGITTY_ACCESS_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::fs::write(dir.path().join("access.txt"), &marker).unwrap();
    let mut definition = adapter.default_definition(availability.path().unwrap().clone());
    if provider == AgentProvider::OhMyPi {
        for (flag, env) in [
            ("--model", "SPAGITTY_TEST_OMP_MODEL"),
            ("--profile", "SPAGITTY_TEST_OMP_PROFILE"),
        ] {
            if let Ok(value) = std::env::var(env) {
                if !value.trim().is_empty() {
                    definition
                        .extra_args
                        .extend([flag.into(), value.trim().into()]);
                }
            }
        }
    }
    if expanded_access && provider == AgentProvider::Codex {
        definition
            .extra_args
            .extend(["--sandbox".into(), "danger-full-access".into()]);
    }
    if expanded_access && provider == AgentProvider::Agy {
        definition
            .extra_args
            .push("--dangerously-skip-permissions".into());
    }
    let prompt = if expanded_access && provider == AgentProvider::Agy {
        "Repository access test only. Use your command/terminal tool to run a read-only shell command that prints access.txt in the working directory and reply with its exact contents. Do not edit anything, use subagents, or access the network. Stop after reading the file."
    } else {
        "Repository access test only. Use your file-reading tool to read access.txt in the working directory and reply with its exact contents. Do not edit anything, use subagents, or access the network. Stop after reading the file."
    };
    let request = AgentRunRequest {
        workdir: dir.path().to_path_buf(),
        prompt: prompt.into(),
        unattended: false,
    };
    let command = read_only(provider, adapter.command(&definition, &request));
    let output = Arc::new(Collected::default());
    let session = process::start(
        &command,
        dir.path(),
        TranscriptWriter::create(&dir.path().join("test.log")).unwrap(),
        output.clone(),
        adapter.narrator(),
    )
    .unwrap();
    let cancel = session.cancellation();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(90));
        cancel.cancel();
    });
    let ended = session.wait();
    let text = output.lines().join("\n");
    assert!(matches!(ended, Ended::Ok), "{ended:?}\n{text}");
    assert!(text.contains(&marker), "File was not read:\n{text}");
}

#[test]
#[ignore = "requires authenticated Codex; explicitly tests opt-in Full Access"]
fn codex_full_access_reads_a_temporary_workspace() {
    reads_file(AgentProvider::Codex, true);
}

#[test]
#[ignore = "requires authenticated OMP and Bun"]
fn omp_is_detected_and_reads_in_print_mode() {
    reads_file(AgentProvider::OhMyPi, false);
}

#[test]
#[ignore = "requires authenticated agy"]
fn agy_is_detected_and_reads_in_print_plan_mode() {
    reads_file(AgentProvider::Agy, false);
}

#[test]
#[ignore = "requires authenticated agy; explicitly tests opt-in tool approval"]
fn agy_auto_approved_command_reads_a_temporary_workspace() {
    reads_file(AgentProvider::Agy, true);
}

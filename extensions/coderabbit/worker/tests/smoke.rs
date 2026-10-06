// SPDX-License-Identifier: GPL-3.0-or-later

//! The real CodeRabbit CLI, against a throwaway repository. Opt-in.
//!
//! ```sh
//! CODERABBIT_SMOKE=1 cargo test -p coderabbit-extension --test smoke -- --nocapture
//! ```
//!
//! Needs `coderabbit` on `PATH`, signed in. It reviews a two-line change in a
//! temporary repository — **this sends that change to CodeRabbit and may use
//! the account's review allowance** — and writes what the CLI printed, with
//! secrets redacted, to `extensions/coderabbit/fixtures/captured/` named after
//! the CLI's version, so the documentation-derived fixtures can be replaced by
//! captured ones. Without `CODERABBIT_SMOKE=1` it does nothing, so CI stays
//! deterministic and never needs an account.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use spagitty_core::fixture::Fixture;
use spagitty_extensions::history::Requester;
use spagitty_extensions::host::{Config, Events, ExtensionHost, HostEvent, Services};
use spagitty_extensions::manifest::{ReviewScope, ReviewTarget};
use spagitty_extensions::protocol::RpcError;
use spagitty_extensions::registry::Paths;
use spagitty_extensions::snapshot::Request;

struct Quiet;
impl Events for Quiet {
    fn emit(&self, event: HostEvent) {
        if let HostEvent::OperationProgress {
            message: Some(m), ..
        } = event
        {
            eprintln!("progress: {m}");
        }
    }
}
impl Services for Quiet {
    fn pull_request_snapshot(&self, _: &std::path::Path, _: u64) -> Result<Value, RpcError> {
        Err(RpcError::new(-32010, "none"))
    }
    fn post_pull_request_comment(
        &self,
        _: &std::path::Path,
        _: u64,
        _: &str,
        _: &str,
        _cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Value, RpcError> {
        Err(RpcError::new(-32011, "none"))
    }
}

fn redact(text: &str) -> String {
    spagitty_extensions::redact::redact(text)
}

#[test]
fn the_real_cli_reviews_a_small_change() {
    if std::env::var("CODERABBIT_SMOKE").as_deref() != Ok("1") {
        eprintln!("skipped: set CODERABBIT_SMOKE=1 to run against the real CodeRabbit CLI");
        return;
    }
    let version = Command::new("coderabbit")
        .arg("--version")
        .output()
        .expect("coderabbit on PATH");
    let version = String::from_utf8_lossy(&version.stdout).trim().to_string();
    eprintln!("CodeRabbit CLI {version}");

    let repo = Fixture::woven();
    repo.write("core.txt", "line 1\nline 2 has a typo: recieve\n");

    // Capture the raw stream once, directly, for fixtures.
    let raw = Command::new("coderabbit")
        .args(["review", "--agent", "--uncommitted"])
        .current_dir(repo.path())
        .output()
        .expect("review ran");
    let captured = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/captured");
    std::fs::create_dir_all(&captured).unwrap();
    let name = format!(
        "cli-{}-uncommitted-exit-{}.ndjson",
        version.replace(' ', "_"),
        raw.status.code().unwrap_or(-1)
    );
    std::fs::write(
        captured.join(&name),
        redact(&String::from_utf8_lossy(&raw.stdout)),
    )
    .unwrap();
    eprintln!("captured {name}; review it for anything private before committing it");

    // And through the host, the way Spagitty runs it.
    let resources = tempfile::tempdir().unwrap();
    let package = resources.path().join("coderabbit");
    std::fs::create_dir_all(&package).unwrap();
    let manifest_text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../extension.json"),
    )
    .unwrap();
    std::fs::write(package.join("extension.json"), &manifest_text).unwrap();
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    let entry = manifest["runtime"]["entrypoints"][spagitty_extensions::version::current_target()]
        .as_str()
        .unwrap();
    std::fs::create_dir_all(package.join(entry).parent().unwrap()).unwrap();
    std::fs::copy(
        env!("CARGO_BIN_EXE_coderabbit-extension"),
        package.join(entry),
    )
    .unwrap();
    let data = tempfile::tempdir().unwrap();
    let host = ExtensionHost::new(
        Config::new(
            "0.9.0",
            Paths {
                data: data.path().join("x"),
                bundled: Some(resources.path().into()),
                exe_dir: None,
            },
        ),
        Arc::new(Quiet),
        Arc::new(Quiet),
    );
    host.enable("spagitty.coderabbit", repo.path(), &[], Some("smoke test"))
        .unwrap();
    let started = host
        .start_review(
            "spagitty.coderabbit",
            "review",
            &Request {
                target: ReviewTarget::WorkingCopy,
                scope: ReviewScope::Uncommitted,
                base: None,
                task_id: None,
                pull_request_number: None,
            },
            repo.path(),
            Requester::Person,
        )
        .unwrap();
    let record = host
        .await_review(
            &started.review_id.unwrap(),
            Duration::from_secs(45 * 60),
            &AtomicBool::new(false),
        )
        .expect("finished");
    eprintln!(
        "status {:?}, {} findings: {}",
        record.result.status,
        record.result.findings.len(),
        record.result.summary
    );
    host.shutdown();
}

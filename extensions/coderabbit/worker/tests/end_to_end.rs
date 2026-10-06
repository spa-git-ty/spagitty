// SPDX-License-Identifier: GPL-3.0-or-later

//! The CodeRabbit extension, end to end: the real worker binary, bundled the
//! way a release bundles it, started by the real host, driving a stand-in CLI
//! (`fake-coderabbit`) that replays the documentation-derived fixtures.
//!
//! What these prove is the plumbing and the decisions — argv, scope, sign-in,
//! outcomes, cancellation, staleness, attribution. What they cannot prove is
//! that the real CLI says what its documentation says; see
//! `extensions/coderabbit/fixtures/README.md`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use spagitty_core::fixture::Fixture;
use spagitty_extensions::capabilities::Capability;
use spagitty_extensions::history::{Requester, ReviewRecord};
use spagitty_extensions::host::{
    Config, Events, ExtensionHost, HostEvent, Invocation, PullRequestRef, Services, State,
};
use spagitty_extensions::manifest::{Context, ReviewScope, ReviewTarget};
use spagitty_extensions::protocol::RpcError;
use spagitty_extensions::registry::{Paths, Provenance};
use spagitty_extensions::review::{Gate, RunStatus, Severity};
use spagitty_extensions::snapshot::Request;

const ID: &str = "spagitty.coderabbit";

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fixture(name: &str) -> String {
    package_dir()
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[derive(Default)]
struct Collected(Mutex<Vec<HostEvent>>);

impl Events for Collected {
    fn emit(&self, event: HostEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct NoForge;

impl Services for NoForge {
    fn pull_request_snapshot(&self, _: &Path, _: u64) -> Result<Value, RpcError> {
        Err(RpcError::new(-32010, "none"))
    }
    fn post_pull_request_comment(
        &self,
        _: &Path,
        _: u64,
        _: &str,
        _: &str,
        _cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Value, RpcError> {
        Err(RpcError::new(-32011, "none"))
    }
}

struct Rig {
    host: ExtensionHost,
    events: Arc<Collected>,
    repo: Fixture,
    cli_dir: tempfile::TempDir,
    _resources: tempfile::TempDir,
    _data: tempfile::TempDir,
}

impl Rig {
    fn new(scenario: Value) -> Rig {
        Self::with_services(scenario, Arc::new(NoForge), &[])
    }

    fn with_services(scenario: Value, services: Arc<dyn Services>, grants: &[Capability]) -> Rig {
        // The package as a release lays it out: the manifest in resources,
        // the program at its entrypoint for this target.
        let resources = tempfile::tempdir().unwrap();
        let package = resources.path().join("coderabbit");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::copy(
            package_dir().join("extension.json"),
            package.join("extension.json"),
        )
        .unwrap();
        let manifest: Value =
            serde_json::from_str(&std::fs::read_to_string(package.join("extension.json")).unwrap())
                .unwrap();
        let target = spagitty_extensions::version::current_target();
        let entry = manifest["runtime"]["entrypoints"][target]
            .as_str()
            .expect("an entrypoint for this target");
        let program = package.join(entry);
        std::fs::create_dir_all(program.parent().unwrap()).unwrap();
        std::fs::copy(env!("CARGO_BIN_EXE_coderabbit-extension"), &program).unwrap();

        // The stand-in CLI, under the name the manifest looks for.
        let cli_dir = tempfile::tempdir().unwrap();
        let cli = cli_dir.path().join(if cfg!(windows) {
            "coderabbit.exe"
        } else {
            "coderabbit"
        });
        std::fs::copy(env!("CARGO_BIN_EXE_fake-coderabbit"), &cli).unwrap();
        std::fs::write(cli_dir.path().join("scenario.json"), scenario.to_string()).unwrap();

        let data = tempfile::tempdir().unwrap();
        let mut config = Config::new(
            "0.9.0",
            Paths {
                data: data.path().join("extensions"),
                bundled: Some(resources.path().to_path_buf()),
                exe_dir: None,
            },
        );
        config.cancel_grace = Duration::from_millis(1500);
        let events = Arc::new(Collected::default());
        let host = ExtensionHost::new(config, events.clone(), services);
        let repo = Fixture::dirty();
        host.enable(
            ID,
            repo.path(),
            grants,
            Some("Reviews send code to CodeRabbit."),
        )
        .unwrap();
        host.choose_executable(ID, "coderabbit", Some(&cli))
            .unwrap();
        Rig {
            host,
            events,
            repo,
            cli_dir,
            _resources: resources,
            _data: data,
        }
    }

    fn review_with(&self, scope: ReviewScope, base: Option<&str>) -> ReviewRecord {
        let started = self
            .host
            .start_review(
                ID,
                "review",
                &request(scope, base),
                self.repo.path(),
                Requester::Person,
            )
            .unwrap();
        self.host
            .await_review(
                &started.review_id.unwrap(),
                Duration::from_secs(60),
                &AtomicBool::new(false),
            )
            .expect("the review finished")
    }

    fn review(&self) -> ReviewRecord {
        self.review_with(ReviewScope::Uncommitted, None)
    }

    fn argv(&self) -> Vec<Vec<String>> {
        std::fs::read_to_string(self.cli_dir.path().join("args.log"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn review_argv(&self) -> Vec<String> {
        self.argv()
            .into_iter()
            .find(|a| a.first().map(String::as_str) == Some("review"))
            .expect("a review run")
    }

    fn command(&self, command: &str) -> (String, String) {
        let started = self
            .host
            .run_command(
                ID,
                command,
                &Invocation {
                    kind: Context::Global,
                    workdir: Some(self.repo.path().to_path_buf()),
                    task_id: None,
                    pull_request: None,
                },
            )
            .unwrap();
        self.finished(&started.operation)
    }

    fn finished(&self, operation_id: &str) -> (String, String) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let found = self.events.0.lock().unwrap().iter().find_map(|e| match e {
                HostEvent::OperationFinished {
                    operation,
                    status,
                    message,
                    ..
                } if operation == operation_id => Some((status.clone(), message.clone())),
                _ => None,
            });
            if let Some(found) = found {
                return found;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

fn request(scope: ReviewScope, base: Option<&str>) -> Request {
    Request {
        target: ReviewTarget::WorkingCopy,
        scope,
        base: base.map(str::to_string),
        task_id: None,
        pull_request_number: None,
    }
}

fn scenario(review: &str) -> Value {
    json!({
        "version": "0.8.1",
        "auth": fixture("documented-auth-signed-in.json"),
        "review": fixture(review),
        "exit": 0
    })
}

#[test]
fn the_bundled_package_is_official_and_starts_nothing_until_used() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    let listing = rig.host.list(Some(rig.repo.path()));
    let view = listing.extensions.iter().find(|e| e.id == ID).unwrap();
    assert_eq!(view.provenance, Provenance::Bundled);
    assert!(view.official);
    assert_eq!(view.state, State::Installed);
    assert!(rig.host.worker_pid(ID).is_none());
    assert!(
        rig.argv()
            .iter()
            .all(|a| a != &vec!["review".to_string(), "--agent".to_string()]),
        "enabling reviews nothing"
    );
}

#[test]
fn a_review_of_uncommitted_changes_runs_exactly_the_documented_command() {
    let rig = Rig::new(scenario("documented-clean-with-findings.ndjson"));
    let record = rig.review();
    assert_eq!(rig.review_argv(), ["review", "--agent", "--uncommitted"]);
    assert!(rig
        .argv()
        .iter()
        .flatten()
        .all(|a| a != "--use-credits" && a != "--api-key"));
    assert!(
        rig.argv()
            .contains(&vec!["auth".into(), "status".into(), "--agent".into()]),
        "sign-in was checked first"
    );

    assert_eq!(record.result.status, RunStatus::Completed);
    assert_eq!(record.result.provider_version, "0.8.1");
    assert_eq!(record.result.findings.len(), 3);
    let severities: Vec<Severity> = record.result.findings.iter().map(|f| f.severity).collect();
    assert_eq!(severities, [Severity::High, Severity::Low, Severity::Info]);
    assert_eq!(
        record.result.findings[0].provider_severity.as_deref(),
        Some("major")
    );
    assert!(
        record
            .result
            .findings
            .iter()
            .all(|f| f.start_line.is_none()),
        "no line is invented"
    );
    assert_eq!(
        record.gate.unwrap().gate,
        Gate::ChangesRequested,
        "a high finding is at or above medium"
    );
}

#[test]
fn a_committed_review_passes_the_pinned_base_commit_and_nothing_else() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    rig.repo.git(&["switch", "-q", "-c", "work"]);
    rig.repo.git(&["stash", "-u", "-q"]);
    rig.repo.write("new.txt", "x\n");
    rig.repo.git(&["add", "new.txt"]);
    rig.repo.commit("Work");
    let record = rig.review_with(ReviewScope::Committed, Some("main"));
    let base = record.snapshot.base_commit.clone();
    // Options are appended in the order of their names (baseCommit, scope).
    assert_eq!(
        rig.review_argv(),
        [
            "review",
            "--agent",
            "--base-commit",
            base.as_str(),
            "--committed"
        ]
    );
    assert_eq!(record.result.status, RunStatus::Completed);
    assert_eq!(record.gate.unwrap().gate, Gate::Pass);
}

#[test]
fn including_untracked_files_adds_exactly_one_flag() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    rig.review_with(ReviewScope::IncludeUntracked, None);
    assert_eq!(
        rig.review_argv(),
        ["review", "--agent", "--uncommitted", "--include-untracked"]
    );
}

#[test]
fn every_documented_outcome_lands_where_it_should() {
    for (stream, exit, status) in [
        (
            "documented-clean-no-findings.ndjson",
            0,
            RunStatus::Completed,
        ),
        ("documented-skipped.ndjson", 0, RunStatus::Skipped),
        (
            "documented-failed-after-findings.ndjson",
            1,
            RunStatus::Failed,
        ),
        (
            "documented-incomplete-unreviewed.ndjson",
            1,
            RunStatus::Failed,
        ),
        (
            "documented-billing-action-required.ndjson",
            0,
            RunStatus::ActionRequired,
        ),
        ("documented-too-many-files.ndjson", 1, RunStatus::Failed),
        ("documented-malformed.ndjson", 0, RunStatus::Incomplete),
        (
            "documented-conflicting-complete.ndjson",
            0,
            RunStatus::Incomplete,
        ),
    ] {
        let mut s = scenario(stream);
        s["exit"] = json!(exit);
        let rig = Rig::new(s);
        let record = rig.review();
        assert_eq!(record.result.status, status, "{stream}");
        if status != RunStatus::Completed {
            assert_ne!(
                record.gate.as_ref().unwrap().gate,
                Gate::Pass,
                "{stream} must never pass a gate"
            );
        }
    }
}

#[test]
fn findings_that_arrived_before_a_failure_are_kept() {
    let mut s = scenario("documented-failed-after-findings.ndjson");
    s["exit"] = json!(1);
    let rig = Rig::new(s);
    let record = rig.review();
    assert_eq!(record.result.findings.len(), 1);
    assert!(record.result.summary.contains("closed the connection"));
}

#[test]
fn an_unknown_severity_blocks_until_someone_looks() {
    let rig = Rig::new(scenario("documented-unknown-severity.ndjson"));
    let record = rig.review();
    assert_eq!(record.result.findings[0].severity, Severity::Unknown);
    assert_eq!(
        record.result.findings[0].provider_severity.as_deref(),
        Some("blocker")
    );
    assert_eq!(record.gate.unwrap().gate, Gate::Blocked);
}

#[test]
fn a_signed_out_cli_asks_for_sign_in_and_reviews_nothing() {
    let mut s = scenario("documented-clean-with-findings.ndjson");
    s["auth"] = json!(fixture("documented-auth-signed-out.json"));
    let rig = Rig::new(s);
    let record = rig.review();
    assert_eq!(record.result.status, RunStatus::ActionRequired);
    assert_eq!(record.result.action_required.unwrap().kind, "signIn");
    assert!(rig
        .argv()
        .iter()
        .all(|a| a.first().map(String::as_str) != Some("review")));
}

#[test]
fn an_old_cli_is_refused_with_update_guidance() {
    let mut s = scenario("documented-clean-with-findings.ndjson");
    s["version"] = json!("0.7.6");
    let rig = Rig::new(s);
    let record = rig.review();
    assert_eq!(record.result.status, RunStatus::Failed);
    assert!(
        record.result.summary.contains("0.7.7"),
        "{}",
        record.result.summary
    );
    assert!(rig
        .argv()
        .iter()
        .all(|a| a.first().map(String::as_str) != Some("review")));
}

#[test]
fn cancelling_a_review_ends_the_cli_and_its_late_output_is_ignored() {
    let mut s = scenario("documented-clean-with-findings.ndjson");
    s["delayMs"] = json!(200);
    s["sleepMs"] = json!(8000);
    let rig = Rig::new(s);
    let started = rig
        .host
        .start_review(
            ID,
            "review",
            &request(ReviewScope::Uncommitted, None),
            rig.repo.path(),
            Requester::Person,
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    rig.host.cancel(&started.operation).unwrap();
    let record = rig
        .host
        .await_review(
            &started.review_id.unwrap(),
            Duration::from_secs(20),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(record.result.status, RunStatus::Cancelled);
    std::thread::sleep(Duration::from_secs(8));
    assert!(
        !rig.cli_dir.path().join("review-survived").exists(),
        "the CLI's process was ended"
    );
}

#[test]
fn an_edit_during_the_review_makes_the_result_stale() {
    let mut s = scenario("documented-clean-with-findings.ndjson");
    s["delayMs"] = json!(300);
    let rig = Rig::new(s);
    let started = rig
        .host
        .start_review(
            ID,
            "review",
            &request(ReviewScope::Uncommitted, None),
            rig.repo.path(),
            Requester::Person,
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(600));
    rig.repo.write("core.txt", "changed during the review\n");
    let record = rig
        .host
        .await_review(
            &started.review_id.unwrap(),
            Duration::from_secs(30),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(record.result.status, RunStatus::Stale);
}

#[test]
fn setup_commands_check_sign_in_sign_in_and_run_diagnostics_only_when_asked() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    let (status, message) = rig.command("checkSetup");
    assert_eq!(status, "completed", "{message}");
    assert!(message.contains("Signed in as example-user"), "{message}");
    assert!(
        rig.argv().iter().all(|a| a != &vec!["doctor".to_string()]),
        "diagnostics did not run on their own"
    );

    let (status, message) = rig.command("signIn");
    assert_eq!(status, "completed", "{message}");
    assert!(rig.argv().contains(&vec![
        "auth".into(),
        "login".into(),
        "--agent".into(),
        "--region".into(),
        "us".into()
    ]));

    let (status, _) = rig.command("runDiagnostics");
    assert_eq!(status, "completed");
    let logs = rig
        .host
        .list(Some(rig.repo.path()))
        .extensions
        .remove(0)
        .diagnostics
        .logs;
    assert!(
        logs.iter()
            .any(|l| l.contains("doctor: Service reachability")),
        "{logs:?}"
    );
}

#[test]
fn the_region_setting_reaches_the_sign_in() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    rig.host
        .set_setting(ID, "region", json!("eu"), Some(rig.repo.path()))
        .unwrap();
    rig.command("signIn");
    assert!(rig.argv().contains(&vec![
        "auth".into(),
        "login".into(),
        "--agent".into(),
        "--region".into(),
        "eu".into()
    ]));
}

#[test]
fn a_credential_store_problem_is_not_called_signed_out() {
    let mut s = scenario("documented-clean-no-findings.ndjson");
    s["auth"] = json!(fixture("documented-auth-credentials-unavailable.json"));
    let rig = Rig::new(s);
    let record = rig.review();
    assert_eq!(record.result.status, RunStatus::Failed);
    assert!(
        record.result.summary.contains("credentials"),
        "{}",
        record.result.summary
    );
}

#[test]
fn the_setup_panel_shows_state_without_contacting_anything() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    let data = rig
        .host
        .resolve_panel(
            ID,
            "connection",
            &Invocation {
                kind: Context::Global,
                workdir: Some(rig.repo.path().to_path_buf()),
                task_id: None,
                pull_request: None,
            },
        )
        .unwrap();
    let text = data.to_string();
    assert!(text.contains("CodeRabbit CLI 0.8.1"));
    assert!(text.contains("Not checked"));
    assert!(
        rig.argv()
            .iter()
            .all(|a| a == &vec!["--version".to_string()]),
        "only local version checks ran: {:?}",
        rig.argv()
    );
}

#[test]
fn a_missing_cli_is_reported_before_activation_finishes() {
    let rig = Rig::new(scenario("documented-clean-no-findings.ndjson"));
    rig.host.choose_executable(ID, "coderabbit", None).unwrap();
    // PATH has no coderabbit on a test runner; if one ever does, this is moot.
    if std::env::var_os("PATH").map(|p| {
        std::env::split_paths(&p).any(|d| {
            d.join("coderabbit").exists()
                || d.join("coderabbit.exe").exists()
                || d.join("cr.exe").exists()
                || d.join("cr").exists()
        })
    }) == Some(true)
    {
        return;
    }
    let record = rig.review();
    assert_eq!(record.result.status, RunStatus::Failed);
    assert!(
        record.result.summary.contains("not installed"),
        "{}",
        record.result.summary
    );
}

struct Forge {
    snapshot: Mutex<Value>,
    posted: Mutex<Vec<String>>,
    uncertain: AtomicBool,
    invalid_receipt: AtomicBool,
}
impl Forge {
    fn new() -> Self {
        Self {
            snapshot: Mutex::new(
                json!({"headSha":"new","baseSha":"base","revisionCurrent":true,"discussion":{"complete":true,"items":[]},"findings":{"complete":true,"items":[]},"reviews":{"complete":true,"items":[]},"checks":{"complete":true,"items":[]}}),
            ),
            posted: Mutex::new(Vec::new()),
            uncertain: AtomicBool::new(false),
            invalid_receipt: AtomicBool::new(false),
        }
    }
}
impl Services for Forge {
    fn pull_request_snapshot(&self, _: &Path, number: u64) -> Result<Value, RpcError> {
        assert_eq!(number, 7);
        Ok(self.snapshot.lock().unwrap().clone())
    }
    fn post_pull_request_comment(
        &self,
        _: &Path,
        number: u64,
        body: &str,
        _: &str,
        cancel: &AtomicBool,
    ) -> Result<Value, RpcError> {
        assert_eq!(number, 7);
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(RpcError::new(-32008, "Cancelled"));
        }
        self.posted.lock().unwrap().push(body.into());
        Ok(
            if self
                .invalid_receipt
                .load(std::sync::atomic::Ordering::Acquire)
            {
                json!({"status":"unknown"})
            } else if self.uncertain.load(std::sync::atomic::Ordering::Acquire) {
                json!({"status":"uncertain","headSha":"new","message":"Delivery uncertain"})
            } else {
                json!({"status":"posted","headSha":"new","commentId":1,"url":"https://github.com/o/r/pull/7#issuecomment-1"})
            },
        )
    }
}
fn pr_context(r: &Rig) -> Invocation {
    Invocation {
        kind: Context::PullRequest,
        workdir: Some(r.repo.path().into()),
        task_id: None,
        pull_request: Some(PullRequestRef {
            number: 7,
            head_sha: Some("new".into()),
        }),
    }
}
fn pr_command(r: &Rig, name: &str) -> (String, String) {
    let start = r.host.run_command(ID, name, &pr_context(r)).unwrap();
    r.finished(&start.operation)
}
#[test]
fn pr_requests_are_requested_until_exact_head_evidence_arrives() {
    let forge = Arc::new(Forge::new());
    let r = Rig::with_services(
        scenario("documented-clean-no-findings.ndjson"),
        forge.clone(),
        &[
            Capability::ForgePullRequestRead,
            Capability::ForgePullRequestComment,
        ],
    );
    assert_eq!(pr_command(&r, "requestIncremental").0, "completed");
    assert_eq!(
        r.host
            .resolve_panel(ID, "pullRequest", &pr_context(&r))
            .unwrap()["state"],
        "requested"
    );
    assert_eq!(pr_command(&r, "requestFull").0, "completed");
    assert_eq!(
        *forge.posted.lock().unwrap(),
        vec!["@coderabbitai review", "@coderabbitai full review"]
    );
    forge.snapshot.lock().unwrap()["checks"]["items"] =
        json!([{"id":1,"appSlug":"coderabbitai","appId":9,"commitSha":"new","state":"completed"}]);
    assert_eq!(
        r.host
            .resolve_panel(ID, "pullRequest", &pr_context(&r))
            .unwrap()["state"],
        "completed"
    );
    forge.snapshot.lock().unwrap()["headSha"] = json!("pushed");
    assert_eq!(
        r.host
            .resolve_panel(ID, "pullRequest", &pr_context(&r))
            .unwrap()["state"],
        "stale"
    );
}
#[test]
fn an_uncertain_pr_write_is_never_retried_without_a_complete_refresh() {
    let forge = Arc::new(Forge::new());
    forge
        .uncertain
        .store(true, std::sync::atomic::Ordering::Release);
    let r = Rig::with_services(
        scenario("documented-clean-no-findings.ndjson"),
        forge.clone(),
        &[
            Capability::ForgePullRequestRead,
            Capability::ForgePullRequestComment,
        ],
    );
    assert!(pr_command(&r, "requestIncremental").1.contains("uncertain"));
    assert_eq!(pr_command(&r, "requestIncremental").0, "failed");
    assert_eq!(forge.posted.lock().unwrap().len(), 1);
    forge.snapshot.lock().unwrap()["discussion"]["complete"] = json!(false);
    r.host
        .resolve_panel(ID, "pullRequest", &pr_context(&r))
        .unwrap();
    assert_eq!(pr_command(&r, "requestIncremental").0, "failed");
    assert_eq!(forge.posted.lock().unwrap().len(), 1);
    forge.snapshot.lock().unwrap()["discussion"]["complete"] = json!(true);
    r.host
        .resolve_panel(ID, "pullRequest", &pr_context(&r))
        .unwrap();
    forge
        .uncertain
        .store(false, std::sync::atomic::Ordering::Release);
    assert_eq!(pr_command(&r, "requestFull").0, "completed");
    assert_eq!(forge.posted.lock().unwrap().len(), 2);
}
#[test]
fn pr_writes_without_the_optional_grant_never_reach_the_service() {
    let forge = Arc::new(Forge::new());
    let r = Rig::with_services(
        scenario("documented-clean-no-findings.ndjson"),
        forge.clone(),
        &[Capability::ForgePullRequestRead],
    );
    assert_eq!(pr_command(&r, "requestFull").0, "failed");
    assert!(forge.posted.lock().unwrap().is_empty());
}

#[test]
fn an_unreadable_delivery_receipt_keeps_uncertainty_and_prevents_a_duplicate() {
    let forge = Arc::new(Forge::new());
    forge
        .invalid_receipt
        .store(true, std::sync::atomic::Ordering::Release);
    let r = Rig::with_services(
        scenario("documented-clean-no-findings.ndjson"),
        forge.clone(),
        &[
            Capability::ForgePullRequestRead,
            Capability::ForgePullRequestComment,
        ],
    );
    assert_eq!(pr_command(&r, "requestIncremental").0, "failed");
    assert_eq!(pr_command(&r, "requestIncremental").0, "failed");
    assert_eq!(forge.posted.lock().unwrap().len(), 1);
}

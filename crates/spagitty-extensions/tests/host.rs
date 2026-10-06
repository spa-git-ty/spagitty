// SPDX-License-Identifier: GPL-3.0-or-later

//! The extension host against real worker processes and real repositories.
//!
//! Every test starts `spagitty-test-worker` — a scripted worker built with
//! this crate — from a development directory of its own, and drives it
//! through the host's public API: the same calls the desktop makes.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use spagitty_core::fixture::Fixture;
use spagitty_extensions::capabilities::Capability;
use spagitty_extensions::history::Requester;
use spagitty_extensions::host::{
    Config, Events, ExtensionHost, HostEvent, Invocation, Services, State,
};
use spagitty_extensions::manifest::{Context, ReviewScope, ReviewTarget};
use spagitty_extensions::protocol::{code, RpcError};
use spagitty_extensions::registry::Paths;
use spagitty_extensions::review::{Gate, RunStatus, Severity};
use spagitty_extensions::snapshot::Request;

const ID: &str = "com.example.test";

#[derive(Default)]
struct Collected(Mutex<Vec<HostEvent>>);

impl Events for Collected {
    fn emit(&self, event: HostEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Collected {
    fn all(&self) -> Vec<HostEvent> {
        self.0.lock().unwrap().clone()
    }

    /// Wait for the operation to finish; return its status and message.
    fn finished(&self, operation: &str, within: Duration) -> (String, String) {
        let deadline = Instant::now() + within;
        loop {
            for event in self.all() {
                if let HostEvent::OperationFinished {
                    operation: o,
                    status,
                    message,
                    ..
                } = event
                {
                    if o == operation {
                        return (status, message);
                    }
                }
            }
            assert!(
                Instant::now() < deadline,
                "{operation} did not finish; events: {:?}",
                self.all()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn finishes_for(&self, operation: &str) -> usize {
        self.all()
            .iter()
            .filter(|e| matches!(e, HostEvent::OperationFinished { operation: o, .. } if o == operation))
            .count()
    }
}

struct FakeServices;

impl Services for FakeServices {
    fn pull_request_snapshot(&self, _: &Path, number: u64) -> Result<Value, RpcError> {
        Ok(json!({"number": number}))
    }
    fn post_pull_request_comment(
        &self,
        _: &Path,
        _: u64,
        _: &str,
        _: &str,
    ) -> Result<Value, RpcError> {
        Err(RpcError::new(code::DECLINED, "declined"))
    }
}

fn manifest() -> Value {
    let program = if cfg!(windows) {
        "bin/worker.exe"
    } else {
        "bin/worker"
    };
    json!({
        "manifestVersion": 1, "id": ID, "name": "Test", "version": "1.0.0", "publisher": "Tests",
        "license": "GPL-3.0-or-later",
        "engines": {"spagitty": ">=0.1.0", "extensionApi": "^1.0.0"},
        "runtime": {"kind": "native-process", "entrypoints": {
            spagitty_extensions::version::current_target(): program
        }},
        "activation": ["onCommand", "onReviewProvider", "onPanel"],
        "capabilities": {
            "required": ["repository.read", "review.provide", "tools.execute"],
            "optional": ["forge.pullRequest.read"]
        },
        "externalTools": [{
            "id": "self", "executableNames": ["spagitty-test-worker"], "versionArgs": ["--version"],
            "minimumVersion": "1.0.0",
            "profiles": [
                {"id": "lines", "args": ["--lines"]},
                {"id": "sleep", "args": ["--sleeper"]}
            ]
        }],
        "contributes": {
            "commands": [
                {"id": "hello", "title": "Say hello", "context": "workingCopy"},
                {"id": "denied", "title": "Denied"}, {"id": "concurrent", "title": "Concurrent"},
                {"id": "twice", "title": "Twice"}, {"id": "slow", "title": "Slow"},
                {"id": "quiet", "title": "Quiet"}, {"id": "crash", "title": "Crash"},
                {"id": "tool", "title": "Tool"}, {"id": "sleepTool", "title": "Sleep tool"},
                {"id": "badTool", "title": "Bad tool"},
                {"id": "review", "title": "Review", "context": "workingCopy", "reviewProvider": "review"}
            ],
            "reviewProviders": [
                {"id": "review", "targets": ["workingCopy", "farmTask"], "sendsCodeTo": "the test service",
                 "configurationFiles": [".test-review.yaml"]},
                {"id": "slowReview", "targets": ["workingCopy"]},
                {"id": "partial", "targets": ["workingCopy"]},
                {"id": "billing", "targets": ["workingCopy"]}
            ],
            "panels": [
                {"id": "findings", "title": "Findings", "renderer": "reviewFindings", "provider": "review"},
                {"id": "about", "title": "About", "renderer": "summary"}
            ],
            "settings": [
                {"key": "greeting", "type": "text", "default": "Hello"},
                {"key": "mode", "type": "enum", "values": ["off", "advisory", "required"], "scope": "repository"}
            ]
        }
    })
}

struct Harness {
    host: ExtensionHost,
    events: Arc<Collected>,
    repo: Fixture,
    package: tempfile::TempDir,
    _data: tempfile::TempDir,
}

impl Harness {
    fn package_dir(&self) -> &Path {
        self.package.path()
    }

    fn invocation(&self) -> Invocation {
        Invocation {
            kind: Context::WorkingCopy,
            workdir: Some(self.repo.path().to_path_buf()),
            task_id: None,
            pull_request: None,
        }
    }

    fn command(&self, command: &str) -> (String, String) {
        let started = self
            .host
            .run_command(ID, command, &self.invocation())
            .unwrap();
        self.events
            .finished(&started.operation, Duration::from_secs(20))
    }

    fn worker_exe(&self) -> PathBuf {
        self.package_dir().join("bin").join(if cfg!(windows) {
            "worker.exe"
        } else {
            "worker"
        })
    }

    fn state(&self) -> State {
        let listing = self.host.list(Some(self.repo.path()));
        listing
            .extensions
            .iter()
            .find(|e| e.id == ID)
            .unwrap()
            .state
    }
}

fn harness_with(mode: &str, tune: impl FnOnce(&mut Config)) -> Harness {
    let data = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    std::fs::write(
        package.path().join("extension.json"),
        manifest().to_string(),
    )
    .unwrap();
    std::fs::write(package.path().join("mode"), mode).unwrap();
    std::fs::create_dir_all(package.path().join("bin")).unwrap();
    let exe = package.path().join("bin").join(if cfg!(windows) {
        "worker.exe"
    } else {
        "worker"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_spagitty-test-worker"), &exe).unwrap();

    let paths = Paths {
        data: data.path().join("extensions"),
        bundled: None,
        exe_dir: None,
    };
    let mut config = Config::new("0.9.0", paths);
    config.inactivity = Duration::from_secs(3);
    config.cancel_grace = Duration::from_millis(800);
    config.handshake = Duration::from_secs(2);
    tune(&mut config);
    let events = Arc::new(Collected::default());
    let host = ExtensionHost::new(config, events.clone(), Arc::new(FakeServices));
    host.attach_development(package.path()).unwrap();
    let repo = Fixture::dirty();
    host.enable(
        ID,
        repo.path(),
        &[],
        Some("I agree to send code to the test service."),
    )
    .unwrap();
    host.choose_executable(ID, "self", Some(&exe)).unwrap();
    Harness {
        host,
        events,
        repo,
        package,
        _data: data,
    }
}

fn harness(mode: &str) -> Harness {
    harness_with(mode, |_| {})
}

fn review(h: &Harness, provider: &str) -> spagitty_extensions::history::ReviewRecord {
    let request = Request {
        target: ReviewTarget::WorkingCopy,
        scope: ReviewScope::Uncommitted,
        base: None,
        task_id: None,
        pull_request_number: None,
    };
    let started = h
        .host
        .start_review(ID, provider, &request, h.repo.path(), Requester::Person)
        .unwrap();
    let id = started.review_id.unwrap();
    h.host
        .await_review(
            &id,
            Duration::from_secs(20),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("the review finished")
}

#[test]
fn a_worker_starts_on_first_use_and_answers_with_what_it_was_granted() {
    let h = harness("normal");
    assert!(h.host.worker_pid(ID).is_none(), "enabling starts nothing");
    assert_eq!(h.state(), State::Installed);

    let (status, message) = h.command("hello");
    assert_eq!(status, "completed");
    assert_eq!(
        message, "Hello main",
        "it read the branch through repository.describe"
    );
    assert!(h.host.worker_pid(ID).is_some());
    assert_eq!(h.state(), State::Active);
    assert!(h.events.all().iter().any(
        |e| matches!(e, HostEvent::Notice { message, .. } if message == "Hello from the test worker")
    ));
    let view = h.host.list(Some(h.repo.path())).extensions.remove(0);
    assert_eq!(
        view.unavailable[0].id, "broken",
        "activation can report what cannot work"
    );
}

#[test]
fn callbacks_are_refused_without_a_grant_or_with_a_forged_handle() {
    let h = harness("normal");
    let (_, message) = h.command("denied");
    assert_eq!(
        message,
        format!("codes {} {}", code::NOT_GRANTED, code::BAD_HANDLE)
    );

    h.host
        .set_grant(ID, h.repo.path(), Capability::ForgePullRequestRead, true)
        .unwrap();
    let (_, message) = h.command("denied");
    assert_eq!(
        message,
        format!("codes 0 {}", code::BAD_HANDLE),
        "granted, the read is answered"
    );
}

#[test]
fn concurrent_callbacks_do_not_deadlock_an_operation() {
    let h = harness("normal");
    let (status, message) = h.command("concurrent");
    assert_eq!(status, "completed");
    assert_eq!(message, "6 stored, k3=3");
}

#[test]
fn only_the_first_completion_counts() {
    let h = harness("normal");
    let started = h.host.run_command(ID, "twice", &h.invocation()).unwrap();
    let (status, message) = h
        .events
        .finished(&started.operation, Duration::from_secs(10));
    assert_eq!((status.as_str(), message.as_str()), ("completed", "first"));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(h.events.finishes_for(&started.operation), 1);
    let logs = h
        .host
        .list(Some(h.repo.path()))
        .extensions
        .remove(0)
        .diagnostics
        .logs;
    assert!(
        logs.iter().any(|l| l.contains("second completion")),
        "{logs:?}"
    );
}

#[test]
fn a_cooperative_worker_stops_when_asked() {
    let h = harness("normal");
    let started = h.host.run_command(ID, "slow", &h.invocation()).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    h.host.cancel(&started.operation).unwrap();
    let (status, _) = h
        .events
        .finished(&started.operation, Duration::from_secs(5));
    assert_eq!(status, "cancelled");
    assert!(
        h.host.worker_pid(ID).is_some(),
        "a worker that stopped as asked keeps running"
    );
}

#[test]
fn a_worker_that_ignores_cancellation_is_ended_after_the_grace() {
    let h = harness("ignoreCancel");
    let started = h.host.run_command(ID, "slow", &h.invocation()).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let asked = Instant::now();
    h.host.cancel(&started.operation).unwrap();
    let (status, _) = h
        .events
        .finished(&started.operation, Duration::from_secs(5));
    assert_eq!(status, "cancelled");
    assert!(
        asked.elapsed() >= Duration::from_millis(700),
        "it was given its grace"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while h.host.worker_pid(ID).is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        h.host.worker_pid(ID).is_none(),
        "its process tree was ended"
    );
}

#[test]
fn a_crash_fails_its_operations_and_leaves_the_host_working() {
    let h = harness("normal");
    let (status, message) = h.command("crash");
    assert_eq!(status, "failed");
    assert!(message.contains("stopped unexpectedly"), "{message}");
    assert!(message.contains("code 3"), "{message}");
    assert_eq!(h.state(), State::Failed);
    let view = h.host.list(Some(h.repo.path())).extensions.remove(0);
    assert!(
        !view.diagnostics.stderr.contains("ghp_0123456789"),
        "stderr is kept redacted"
    );
    assert!(view.diagnostics.stderr.contains("[redacted]"));

    // An explicit retry starts it again; nothing restarted it on its own.
    let (status, _) = h.command("hello");
    assert_eq!(status, "completed");
}

#[test]
fn three_crashes_keep_it_stopped_until_a_person_restarts_it() {
    let h = harness("normal");
    for _ in 0..3 {
        let (status, _) = h.command("crash");
        assert_eq!(status, "failed");
    }
    let refused = h
        .host
        .run_command(ID, "hello", &h.invocation())
        .unwrap_err();
    assert!(refused.to_string().contains("Restart it"), "{refused}");
    h.host.restart(ID).unwrap();
    assert_eq!(h.command("hello").0, "completed");
}

#[test]
fn an_operation_that_goes_quiet_is_ended() {
    let h = harness_with("normal", |c| c.inactivity = Duration::from_secs(1));
    let started = h.host.run_command(ID, "quiet", &h.invocation()).unwrap();
    let (status, message) = h
        .events
        .finished(&started.operation, Duration::from_secs(10));
    assert_eq!(status, "failed");
    assert!(message.contains("reported nothing"), "{message}");
}

#[test]
fn a_worker_speaking_another_major_version_is_refused_before_it_does_anything() {
    let h = harness("badHandshake");
    let error = h
        .host
        .run_command(ID, "hello", &h.invocation())
        .unwrap_err();
    assert!(error.to_string().contains("2.0.0"), "{error}");
    assert!(h.host.worker_pid(ID).is_none());
}

#[test]
fn a_worker_that_never_answers_its_handshake_is_stopped() {
    let h = harness("silent");
    let started = Instant::now();
    let error = h
        .host
        .run_command(ID, "hello", &h.invocation())
        .unwrap_err();
    assert!(error.to_string().contains("handshake"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(8));
    assert_eq!(h.state(), State::Failed);
}

#[test]
fn anything_on_stdout_that_is_not_the_protocol_stops_the_worker() {
    let h = harness("garbage");
    // The handshake may complete before the bad line is read; whichever way,
    // the worker ends up stopped and failed.
    let _ = h.host.run_command(ID, "slow", &h.invocation());
    let deadline = Instant::now() + Duration::from_secs(5);
    while h.state() != State::Failed && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(h.state(), State::Failed);
    let view = h.host.list(Some(h.repo.path())).extensions.remove(0);
    assert!(view.state_reason.unwrap().contains("protocol"));
}

#[test]
fn a_declared_tool_runs_through_its_profile_and_streams_its_output() {
    let h = harness("normal");
    let (_, message) = h.command("tool");
    assert_eq!(message, "exit 0");
    let lines = std::fs::read_to_string(h.package_dir().join("tool-lines")).unwrap();
    assert_eq!(
        lines.lines().count(),
        5,
        "every stdout line reached the worker"
    );
    let view = h.host.list(Some(h.repo.path())).extensions.remove(0);
    assert_eq!(
        view.diagnostics.tool_runs[0].args,
        vec!["--lines".to_string()]
    );
    assert_eq!(
        view.tools[0].detected.as_ref().unwrap().version.as_deref(),
        Some("1.2.3")
    );
}

#[test]
fn an_undeclared_option_is_refused_before_anything_runs() {
    let h = harness("normal");
    let (_, message) = h.command("badTool");
    assert_eq!(message, format!("error {}", code::TOOL_REFUSED));
}

#[test]
fn cancelling_ends_the_tools_process_tree() {
    let h = harness("normal");
    let started = h
        .host
        .run_command(ID, "sleepTool", &h.invocation())
        .unwrap();
    let marker = h.repo.path().join("sleeper-started");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(marker.exists(), "the tool ran in the approved directory");
    h.host.cancel(&started.operation).unwrap();
    h.events
        .finished(&started.operation, Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(5));
    assert!(
        !h.repo.path().join("sleeper-survived").exists(),
        "the tool was ended with the operation"
    );
}

#[test]
fn a_review_keeps_only_valid_findings_and_the_host_decides_the_gate() {
    let h = harness("normal");
    let record = review(&h, "review");
    assert_eq!(record.result.status, RunStatus::Completed);
    assert_eq!(record.result.provider_version, "9.9.9");
    assert_eq!(record.result.summary, "Reviewed scope uncommitted");
    let ids: Vec<&str> = record
        .result
        .findings
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["f1", "f3"],
        "the duplicate and the escaping path were refused"
    );
    let f1 = &record.result.findings[0];
    assert_eq!(f1.severity, Severity::High);
    assert_eq!(
        f1.disposition,
        spagitty_extensions::review::Disposition::Open,
        "the host owns dispositions"
    );
    assert_eq!(f1.review_id, record.result.review_id);
    let f3 = &record.result.findings[1];
    assert!(
        f3.path.is_none() && f3.start_line.is_none(),
        "no location was invented"
    );
    assert!(f3.source_url.is_none(), "a non-https link was dropped");
    assert_eq!(
        record.gate.unwrap().gate,
        Gate::Blocked,
        "an unknown severity blocks"
    );

    let history = h.host.reviews(ID, h.repo.path()).unwrap();
    assert_eq!(history[0].result.review_id, record.result.review_id);
}

#[test]
fn code_that_changes_during_a_review_makes_the_result_stale() {
    let h = harness("normal");
    let request = Request {
        target: ReviewTarget::WorkingCopy,
        scope: ReviewScope::Uncommitted,
        base: None,
        task_id: None,
        pull_request_number: None,
    };
    h.host.enable(ID, h.repo.path(), &[], None).unwrap();
    let started = h
        .host
        .start_review(ID, "slowReview", &request, h.repo.path(), Requester::Person)
        .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.repo
        .write("core.txt", "edited while it was being reviewed\n");
    let record = h
        .host
        .await_review(
            &started.review_id.unwrap(),
            Duration::from_secs(20),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(record.result.status, RunStatus::Stale);
}

#[test]
fn partial_and_billing_outcomes_are_never_success() {
    let h = harness("normal");
    assert_eq!(review(&h, "partial").result.status, RunStatus::Incomplete);
    let billing = review(&h, "billing");
    assert_eq!(billing.result.status, RunStatus::ActionRequired);
    assert_eq!(billing.result.action_required.unwrap().kind, "billing");
}

#[test]
fn nothing_runs_where_the_extension_is_not_enabled_or_consented() {
    let h = harness("normal");
    let other = Fixture::woven();
    let invocation = Invocation {
        workdir: Some(other.path().to_path_buf()),
        ..h.invocation()
    };
    assert!(h.host.run_command(ID, "hello", &invocation).is_err());

    let refused = h.host.enable(ID, other.path(), &[], None).unwrap_err();
    assert!(refused.to_string().contains("sends code"), "{refused}");

    h.host.disable(ID, h.repo.path(), true).unwrap();
    assert_eq!(h.state(), State::Disabled);
    assert!(h.host.run_command(ID, "hello", &h.invocation()).is_err());
    assert!(h.host.worker_pid(ID).is_none(), "disabling stopped it");
    let view = h.host.list(Some(h.repo.path())).extensions.remove(0);
    assert!(view.capabilities.iter().all(|c| !c.granted));
    assert!(!view.consented);
}

#[test]
fn a_summary_panel_is_drawn_from_the_workers_data() {
    let h = harness("normal");
    let data = h.host.resolve_panel(ID, "about", &h.invocation()).unwrap();
    assert_eq!(data["rows"][0]["value"], "about");
    assert!(
        h.host
            .resolve_panel(ID, "findings", &h.invocation())
            .is_err(),
        "findings come from history"
    );
}

#[test]
fn settings_are_validated_and_reach_a_running_worker() {
    let h = harness("normal");
    h.command("hello");
    assert!(h
        .host
        .set_setting(ID, "mode", json!("sometimes"), Some(h.repo.path()))
        .is_err());
    h.host
        .set_setting(ID, "mode", json!("required"), Some(h.repo.path()))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let seen = h.package_dir().join("settings-seen.json");
    while !seen.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let seen: Value = serde_json::from_str(&std::fs::read_to_string(seen).unwrap()).unwrap();
    assert_eq!(seen["settings"]["mode"], "required");
    assert_eq!(seen["settings"]["greeting"], "Hello");
    assert_eq!(
        h.host.setting(ID, "mode", Some(h.repo.path())),
        Some(json!("required"))
    );
}

#[test]
fn a_packaged_copy_installs_runs_and_uninstalls_without_a_trace() {
    let h = harness("normal");
    // Package the development directory under another identity.
    let copy = tempfile::tempdir().unwrap();
    let mut m = manifest();
    m["id"] = json!("com.example.packaged");
    std::fs::write(copy.path().join("extension.json"), m.to_string()).unwrap();
    std::fs::create_dir_all(copy.path().join("bin")).unwrap();
    std::fs::copy(
        h.worker_exe(),
        copy.path()
            .join("bin")
            .join(h.worker_exe().file_name().unwrap()),
    )
    .unwrap();
    std::fs::write(copy.path().join("mode"), "normal").unwrap();
    let bytes = spagitty_extensions::package::pack_directory(copy.path()).unwrap();
    let file = copy.path().join("packaged.spagitty-extension");
    std::fs::write(&file, bytes).unwrap();

    let preview = h.host.inspect_package(&file, Some(h.repo.path())).unwrap();
    assert_eq!(preview.id, "com.example.packaged");
    assert!(preview.compatibility.compatible);
    h.host.install(&preview.token).unwrap();
    h.host
        .enable("com.example.packaged", h.repo.path(), &[], Some("yes"))
        .unwrap();
    let started = h
        .host
        .run_command("com.example.packaged", "hello", &h.invocation())
        .unwrap();
    assert_eq!(
        h.events
            .finished(&started.operation, Duration::from_secs(20))
            .1,
        "Hello main"
    );

    h.host
        .uninstall("com.example.packaged", false, Some(h.repo.path()))
        .unwrap();
    let listing = h.host.list(Some(h.repo.path()));
    assert!(listing
        .extensions
        .iter()
        .all(|e| e.id != "com.example.packaged"));
    assert!(!h
        .host
        .paths()
        .packages()
        .join("com.example.packaged")
        .exists());
}

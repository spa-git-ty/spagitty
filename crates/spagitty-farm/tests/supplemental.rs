// SPDX-License-Identifier: GPL-3.0-or-later

//! Merge authority and durable evidence against real repositories/worktrees.
use spagitty_core::fixture::Fixture;
use spagitty_farm::{
    model::*,
    service::{FarmService, Recorder},
    supplemental::{self, Evidence, Input, Mode, Outcome, Provider},
};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
struct TestProvider {
    unavailable: AtomicBool,
    calls: AtomicUsize,
}
impl TestProvider {
    fn new() -> Self {
        Self {
            unavailable: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
        }
    }
}
impl Provider for TestProvider {
    fn review(&self, i: &Input, _: &AtomicBool) -> Result<Evidence, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Evidence {
            task: i.task.clone(),
            head: i.head.clone(),
            policy: i.policy.clone(),
            identity: "test-provider-1".into(),
            outcome: Outcome::Pass,
            summary: "Reviewed exact change.".into(),
            change_request: String::new(),
            record: serde_json::Value::Null,
        })
    }
    fn validate(&self, i: &Input, e: &Evidence) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        supplemental::matching(i, e)?;
        if self.unavailable.load(Ordering::SeqCst) {
            Err("Provider disabled or unauthenticated.".into())
        } else {
            Ok(())
        }
    }
}
struct Rig {
    repo: Fixture,
    _dir: tempfile::TempDir,
    workdir: PathBuf,
    service: FarmService,
    id: TaskId,
    provider: Arc<TestProvider>,
}
impl Rig {
    fn new() -> Self {
        let repo = Fixture::woven();
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path().join("task");
        repo.git(&[
            "worktree",
            "add",
            "-b",
            "supplemental-test",
            workdir.to_str().unwrap(),
            "HEAD",
        ]);
        let service = FarmService::open(repo.path(), Arc::new(Recorder::default()));
        service.create("Test", "Review and merge").unwrap();
        let id = TaskId::new("TASK-0001");
        let mut task = Task::new(id.clone(), "A task", 0);
        task.status = TaskStatus::Review;
        task.worktree = Some(workdir.to_string_lossy().into());
        task.branch = Some("supplemental-test".into());
        task.merge_target = Some("main".into());
        task.implemented_by = Some(AgentId::new("implementer"));
        service
            .configure(|f| {
                f.tasks.push(task);
                f.supplemental.mode = Mode::Required;
            })
            .unwrap();
        let provider = Arc::new(TestProvider::new());
        service.set_supplemental_provider(provider.clone());
        Self {
            repo,
            _dir: dir,
            workdir,
            service,
            id,
            provider,
        }
    }
    fn task(&self) -> Task {
        self.service.farm().unwrap().task(&self.id).unwrap().clone()
    }
    fn review(&self) {
        assert_eq!(
            self.service.review_supplemental(&self.id).unwrap().outcome,
            Outcome::Pass
        );
    }
}
#[test]
fn required_gate_blocks_manual_and_automatic_merges_without_evidence() {
    let r = Rig::new();
    let before = r.repo.git(&["rev-parse", "HEAD"]);
    assert!(r
        .service
        .merge(&r.id)
        .unwrap_err()
        .to_string()
        .contains("supplemental"));
    r.service
        .configure(|f| {
            f.autonomy = Autonomy::Auto;
            f.permissions.merge = true;
        })
        .unwrap();
    r.service.approve(&r.id).unwrap();
    assert_eq!(r.task().status, TaskStatus::Review);
    assert!(r.task().note.unwrap().contains("supplemental"));
    assert_eq!(r.repo.git(&["rev-parse", "HEAD"]), before);
}
#[test]
fn a_person_can_merge_with_current_supplemental_evidence() {
    let r = Rig::new();
    r.review();
    r.service.merge(&r.id).unwrap();
    assert_eq!(r.task().status, TaskStatus::Done);
    assert!(r.provider.calls.load(Ordering::SeqCst) >= 2);
}
#[test]
fn unavailable_provider_blocks_even_with_a_previous_pass() {
    let r = Rig::new();
    r.review();
    r.provider.unavailable.store(true, Ordering::SeqCst);
    assert!(r
        .service
        .merge(&r.id)
        .unwrap_err()
        .to_string()
        .contains("unauthenticated"));
    assert_eq!(r.task().status, TaskStatus::Review);
}
#[test]
fn evidence_survives_restart_but_requires_a_live_provider() {
    let r = Rig::new();
    r.review();
    let reopened = FarmService::open(r.repo.path(), Arc::new(Recorder::default()));
    assert!(reopened
        .farm()
        .unwrap()
        .supplemental_evidence
        .contains_key(&r.id));
    assert!(reopened
        .merge(&r.id)
        .unwrap_err()
        .to_string()
        .contains("unavailable"));
    reopened.set_supplemental_provider(r.provider.clone());
    reopened.merge(&r.id).unwrap();
}
#[test]
fn policy_change_invalidates_existing_evidence_and_records_the_actor() {
    let r = Rig::new();
    r.review();
    let before = r.service.farm().unwrap().supplemental.revision;
    r.service
        .configure(|f| f.supplemental.threshold = supplemental::Threshold::High)
        .unwrap();
    assert!(r.service.farm().unwrap().supplemental.revision > before);
    assert!(r
        .service
        .merge(&r.id)
        .unwrap_err()
        .to_string()
        .contains("out of date"));
    assert!(r.service.events().iter().any(
        |e| matches!(&e.event,FarmEvent::SupplementalPolicyChanged{actor,..} if actor=="person")
    ));
}
#[test]
fn an_intentional_policy_downgrade_is_not_recorded_as_approval() {
    let r = Rig::new();
    r.service
        .configure(|f| f.supplemental.mode = Mode::Advisory)
        .unwrap();
    r.service.merge(&r.id).unwrap();
    assert!(r.service.farm().unwrap().supplemental_evidence.is_empty());
    assert!(r.service.events().iter().any(|e| matches!(
        e.event,
        FarmEvent::SupplementalPolicyChanged {
            mode: Mode::Advisory,
            ..
        }
    )));
}
#[test]
fn new_task_code_requires_new_evidence() {
    let r = Rig::new();
    r.review();
    std::fs::write(r.workdir.join("change.txt"), "new code").unwrap();
    r.repo
        .git(&["-C", r.workdir.to_str().unwrap(), "add", "change.txt"]);
    r.repo.git(&[
        "-C",
        r.workdir.to_str().unwrap(),
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-m",
        "New code",
    ]);
    assert!(r
        .service
        .merge(&r.id)
        .unwrap_err()
        .to_string()
        .contains("out of date"));
}
#[test]
fn supplemental_pass_never_substitutes_for_automatic_verification_and_independent_review() {
    let r = Rig::new();
    r.service
        .configure(|f| {
            f.autonomy = Autonomy::Auto;
            f.permissions.merge = true;
        })
        .unwrap();
    r.review();
    r.service.approve(&r.id).unwrap();
    assert_eq!(r.task().status, TaskStatus::Review);
    assert!(r.task().note.unwrap().contains("verification"));
    assert!(!r
        .service
        .events()
        .iter()
        .any(|e| matches!(e.event, FarmEvent::MergeCompleted { ok: true, .. })));
}

fn selected(r: &Rig) -> Evidence {
    let f = r.service.farm().unwrap();
    Evidence {
        task: r.id.clone(),
        head: r
            .repo
            .git(&["-C", r.workdir.to_str().unwrap(), "rev-parse", "HEAD"])
            .trim()
            .into(),
        policy: f.supplemental,
        identity: String::new(),
        outcome: Outcome::ChangesRequested,
        summary: "Selected finding".into(),
        change_request: "Follow repository rules. Finding stable-1: fix the defect.".into(),
        record: serde_json::json!({"finding":"stable-1"}),
    }
}
#[test]
fn selected_findings_use_the_existing_repair_path_and_survive_restart() {
    let r = Rig::new();
    let before = r.repo.git(&["rev-parse", "HEAD"]);
    r.service
        .request_supplemental_changes(selected(&r))
        .unwrap();
    assert_eq!(r.task().status, TaskStatus::Blocked); // Manual autonomy awaits a person.
    let reopened = FarmService::open(r.repo.path(), Arc::new(Recorder::default()));
    let f = reopened.farm().unwrap();
    assert_eq!(f.supplemental_repairs[&r.id], 1);
    assert!(f.supplemental_evidence[&r.id]
        .change_request
        .contains("stable-1"));
    assert_eq!(r.repo.git(&["rev-parse", "HEAD"]), before);
}
#[test]
fn unattended_repairs_honor_the_supplemental_budget() {
    let r = Rig::new();
    r.service
        .configure(|f| {
            f.autonomy = Autonomy::Yolo;
            f.supplemental.max_repairs = 0;
        })
        .unwrap();
    r.service
        .request_supplemental_changes(selected(&r))
        .unwrap();
    assert_eq!(r.task().status, TaskStatus::Blocked);
    assert!(r.task().note.unwrap().contains("budget"));
    assert_eq!(r.service.farm().unwrap().supplemental_repairs[&r.id], 0);
}
#[test]
fn stale_selected_findings_cannot_change_the_task_or_spend_a_repair() {
    let r = Rig::new();
    let mut ev = selected(&r);
    ev.head = "old".into();
    assert!(r.service.request_supplemental_changes(ev).is_err());
    assert_eq!(r.task().status, TaskStatus::Review);
    assert!(r.service.farm().unwrap().supplemental_repairs.is_empty());
}

struct WaitingProvider(std::sync::mpsc::Sender<()>);
impl Provider for WaitingProvider {
    fn review(&self, i: &Input, stop: &AtomicBool) -> Result<Evidence, String> {
        self.0.send(()).unwrap();
        while !stop.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        TestProvider::new().review(i, stop)
    }
    fn validate(&self, _: &Input, _: &Evidence) -> Result<(), String> {
        Err("cancelled".into())
    }
}
#[test]
fn cancellation_during_review_cannot_publish_pass_or_resurrect_the_task() {
    let r = Rig::new();
    let (send, receive) = std::sync::mpsc::channel();
    r.service
        .set_supplemental_provider(Arc::new(WaitingProvider(send)));
    std::thread::scope(|scope| {
        let pending = scope.spawn(|| r.service.review_supplemental(&r.id));
        receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(r
            .service
            .review_supplemental(&r.id)
            .unwrap_err()
            .to_string()
            .contains("already running"));
        r.service.cancel_task(&r.id).unwrap();
        assert_eq!(pending.join().unwrap().unwrap().outcome, Outcome::Cancelled);
    });
    assert_eq!(r.task().status, TaskStatus::Cancelled);
    assert!(r.service.farm().unwrap().supplemental_evidence.is_empty());
    assert!(r
        .service
        .events()
        .iter()
        .any(|e| matches!(&e.event,FarmEvent::SupplementalReview{state,..}if state=="cancelled")));
}

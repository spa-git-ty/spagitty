// SPDX-License-Identifier: GPL-3.0-or-later

//! The engine against a fake repository and an agent that follows a script,
//! so every level and every limit has a test without a provider.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use spagitty_core::diff::{DiffLine, LineOrigin};

use super::classify::Choice;
use super::engine::{
    self, Ask, Changed, Control, Driver, Limits, MergeFile, MergeRegion, MergeWork, Reply,
    ReviewWork, Setup, Sink, World,
};
use super::guard::Snapshot;
use super::level::{Job, Level};
use super::protocol::{Answer, FENCE};
use super::record::{
    tests::sample, Assignment, CheckRun, LastAct, ProposalBody, ProposalState, State, StepKind,
    StepState, Store, Target, Tokens,
};
use super::rules::RepoRules;

// ── The fake repository ──────────────────────────────────────────────────

/// Each run of the checks, with the files it was given.
type Checked = Arc<Mutex<Vec<Vec<(String, String)>>>>;

#[derive(Clone, Default)]
struct Fake {
    files: Vec<&'static str>,
    checks: Vec<String>,
    failing: bool,
    moved: Arc<AtomicBool>,
    checked: Checked,
}

impl World for Fake {
    fn workdir(&self) -> PathBuf {
        std::env::temp_dir()
    }
    fn policy(&self) -> String {
        "## From AGENTS.md\n\nNo unwrap.".into()
    }
    fn changed_files(&self) -> Result<Vec<Changed>, String> {
        Ok(self
            .files
            .iter()
            .map(|path| Changed {
                path: path.to_string(),
                added: 2,
                removed: 1,
                binary: false,
            })
            .collect())
    }
    fn diff(&self, _path: &str) -> Result<Vec<DiffLine>, String> {
        Ok(vec![
            DiffLine {
                origin: LineOrigin::Context,
                old: Some(1),
                new: Some(1),
                text: "a".into(),
            },
            DiffLine {
                origin: LineOrigin::Removed,
                old: Some(2),
                new: None,
                text: "b".into(),
            },
            DiffLine {
                origin: LineOrigin::Added,
                old: None,
                new: Some(2),
                text: "c".into(),
            },
            DiffLine {
                origin: LineOrigin::Added,
                old: None,
                new: Some(3),
                text: "d".into(),
            },
        ])
    }
    fn read(&self, _path: &str, _head: bool) -> Result<String, String> {
        Ok(String::new())
    }
    fn search(&self, _pattern: &str) -> Result<String, String> {
        Ok(String::new())
    }
    fn moved(&self) -> bool {
        self.moved.load(Ordering::Acquire)
    }
    fn checks(&self) -> Vec<String> {
        self.checks.clone()
    }
    fn run_checks(
        &self,
        resolved: &[(String, String)],
        _cancel: &AtomicBool,
        hear: &mut dyn FnMut(&str),
    ) -> Result<Vec<CheckRun>, String> {
        self.checked.lock().unwrap().push(resolved.to_vec());
        hear("$ bun test");
        Ok(vec![CheckRun {
            command: "bun test".into(),
            passed: !self.failing,
            output: String::new(),
            duration_ms: 1,
        }])
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot::default()
    }
    fn restore(&self, _snapshot: &Snapshot) -> Vec<String> {
        Vec::new()
    }
}

// ── The scripted agent ───────────────────────────────────────────────────

enum Line {
    Says(String),
    Fails(&'static str),
    /// Waits until cancelled, as a slow agent does.
    Hangs,
}

#[derive(Clone)]
struct Script {
    lines: Arc<Mutex<VecDeque<Line>>>,
    asked: Arc<Mutex<Vec<String>>>,
    tokens: u64,
}

impl Script {
    fn new(lines: Vec<Line>) -> Script {
        Script {
            lines: Arc::new(Mutex::new(lines.into())),
            asked: Arc::default(),
            tokens: 0,
        }
    }
}

impl Driver for Script {
    fn run(
        &mut self,
        ask: &Ask,
        _world: &dyn World,
        hear: &mut dyn FnMut(&str),
        cancel: &Arc<AtomicBool>,
    ) -> Result<Reply, String> {
        self.asked.lock().unwrap().push(ask.prompt.clone());
        hear("reading");
        let next = self.lines.lock().unwrap().pop_front();
        match next {
            Some(Line::Says(text)) => Ok(Reply {
                answer: Answer::find(&text),
                last_words: text,
                tokens: Tokens {
                    input: self.tokens,
                    output: 0,
                },
                command: Some("fake".into()),
                sent: Vec::new(),
            }),
            Some(Line::Fails(why)) => Err(why.into()),
            Some(Line::Hangs) => {
                while !cancel.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err("was stopped".into())
            }
            None => Err("ran out of script".into()),
        }
    }
}

fn says(json: &str) -> Line {
    Line::Says(format!("thinking\n```{FENCE}\n{json}\n```\n"))
}

fn plan(paths: &[&str]) -> Line {
    let files: Vec<String> = paths
        .iter()
        .map(|p| format!(r#"{{"path":"{p}","why":"x"}}"#))
        .collect();
    says(&format!(
        r#"{{"plan":[{}],"lookFor":"races"}}"#,
        files.join(",")
    ))
}

fn findings(sure: bool) -> Line {
    says(&format!(
        r#"{{"findings":[{{"line":2,"severity":"high","body":"Write to a temporary file, then rename.","sure":{sure}}},{{"line":40,"severity":"low","body":"not on the diff"}}]}}"#
    ))
}

fn verdict(word: &str) -> Line {
    says(&format!(
        r#"{{"summary":"Caches avatars on disk.","verdict":"{word}","sure":true}}"#
    ))
}

fn resolution(text: &str, sure: bool) -> Line {
    let text = serde_json::to_string(text).unwrap();
    says(&format!(
        r#"{{"resolution":{{"text":{text},"why":"both kept","sure":{sure}}}}}"#
    ))
}

// ── Running ──────────────────────────────────────────────────────────────

#[derive(Default)]
struct Seen(Mutex<Vec<Assignment>>);

impl Sink for Seen {
    fn changed(&self, assignment: &Assignment) {
        self.0.lock().unwrap().push(assignment.clone());
    }
    fn line(&self, _id: &str, _line: &str) {}
}

struct Run {
    handle: engine::Handle,
    thread: std::thread::JoinHandle<()>,
    script: Script,
    _store: tempfile::TempDir,
}

impl Run {
    fn until(&self, what: &str, test: impl Fn(&Assignment) -> bool) -> Assignment {
        let started = Instant::now();
        loop {
            let now = self.handle.snapshot();
            if test(&now) {
                return now;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "waited for {what}; state {:?}, sentence {:?}",
                now.state,
                now.sentence
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn waiting(&self) -> Assignment {
        self.until("a gate", |a| a.state == State::Waiting)
    }

    fn send(&self, control: Control) {
        self.handle.send(control);
    }

    fn end(self) -> Assignment {
        self.thread.join().unwrap();
        self.handle.snapshot()
    }
}

fn review(level: Level, lines: Vec<Line>, rules: RepoRules, limits: Limits) -> Run {
    review_in(
        level,
        lines,
        rules,
        limits,
        Fake {
            files: vec!["src/avatars.rs", "src/types.ts"],
            ..Fake::default()
        },
    )
}

fn review_in(level: Level, lines: Vec<Line>, rules: RepoRules, limits: Limits, world: Fake) -> Run {
    let mut assignment = sample("/work/app", "r1");
    assignment.level = level;
    assignment.state = State::Starting;
    start(
        assignment,
        engine::Work::Review(ReviewWork::default()),
        lines,
        rules,
        limits,
        world,
    )
}

fn start(
    assignment: Assignment,
    work: engine::Work,
    lines: Vec<Line>,
    rules: RepoRules,
    limits: Limits,
    world: Fake,
) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let script = Script::new(lines);
    let (handle, engine) = engine::prepare(Setup {
        assignment,
        work,
        rules,
        limits,
        world: Box::new(world),
        driver: Box::new(script.clone()),
        store: Store::new(dir.path()),
        sink: Arc::new(Seen::default()),
    });
    let thread = std::thread::spawn(move || engine.run());
    Run {
        handle,
        thread,
        script,
        _store: dir,
    }
}

fn open() -> RepoRules {
    RepoRules {
        highest: Level::Unattended,
        never_unattended: Vec::new(),
        ..RepoRules::default()
    }
}

fn comments(a: &Assignment) -> Vec<(ProposalState, bool)> {
    a.proposals
        .iter()
        .filter_map(|p| match &p.body {
            ProposalBody::Comment(_) => Some((p.state, p.sure)),
            _ => None,
        })
        .collect()
}

// ── Review ───────────────────────────────────────────────────────────────

#[test]
fn step_by_step_stops_at_the_plan_each_file_and_the_verdict() {
    let run = review(
        Level::StepByStep,
        vec![
            plan(&["src/avatars.rs", "src/types.ts"]),
            findings(true),
            findings(true),
            verdict("approve"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    let at = run.waiting();
    assert_eq!(at.sentence, "Waiting for you: the plan · 2 files");
    run.send(Control::Continue);

    let at = run.until("the first file", |a| {
        a.waiting()
            && a.steps.last().is_some_and(
                |s| matches!(&s.kind, StepKind::File { path } if path == "src/avatars.rs"),
            )
    });
    assert_eq!(at.sentence, "Waiting for you: 1 finding on avatars.rs");
    // A finding off the diff is not kept, and says so.
    assert_eq!(comments(&at), [(ProposalState::Proposed, true)]);
    assert!(at
        .steps
        .last()
        .unwrap()
        .events
        .iter()
        .any(|e| e.contains("not on a line of the diff")));
    run.send(Control::Continue);

    run.until("the second file", |a| a.waiting() && a.steps.len() == 4);
    run.send(Control::Continue);
    let at = run.until("the verdict", |a| a.waiting() && a.steps.len() == 5);
    assert_eq!(at.sentence, "Waiting for you: the verdict");
    run.send(Control::Continue);

    let end = run.end();
    assert_eq!(end.state, State::Done);
    // Nothing was applied: at Step by step the person decides each one.
    assert!(comments(&end)
        .iter()
        .all(|(state, _)| *state == ProposalState::Proposed));
    assert_eq!(end.sentence, "Done · 2 findings to decide");
}

#[test]
fn suggest_runs_to_the_end_and_applies_nothing() {
    let run = review(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("comment"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    let end = run.end();
    assert_eq!(end.state, State::Done);
    assert_eq!(comments(&end), [(ProposalState::Proposed, true)]);
    assert!(end.steps.iter().all(|s| s.state == StepState::Done));
}

#[test]
fn sign_off_applies_sure_findings_and_waits_to_send() {
    let run = review(
        Level::SignOff,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("approve"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    let at = run.waiting();
    assert_eq!(at.sentence, "Waiting for you: Finish review");
    assert_eq!(comments(&at), [(ProposalState::Applied, true)]);
    assert!(at.last_act.is_none(), "sending is the person's at Sign off");
    run.send(Control::Acted {
        ok: true,
        message: String::new(),
    });
    assert_eq!(run.end().state, State::Done);
}

#[test]
fn an_unsure_finding_stops_even_unattended_and_is_never_applied() {
    let run = review(
        Level::Unattended,
        vec![
            plan(&["src/avatars.rs"]),
            findings(false),
            verdict("approve"),
        ],
        open(),
        Limits::default(),
    );
    let at = run.waiting();
    assert!(
        at.sentence.contains("1 finding on avatars.rs"),
        "{}",
        at.sentence
    );
    run.send(Control::Continue);
    // Still unsure and still undecided at the end: the last act is the person's.
    let at = run.until("send", |a| a.waiting() && a.steps.len() == 5);
    assert_eq!(at.sentence, "Waiting for you: Finish review");
    assert_eq!(comments(&at), [(ProposalState::Proposed, false)]);
    run.send(Control::Stop);
    assert_eq!(run.end().state, State::Stopped);
}

#[test]
fn unattended_sends_as_comment_unless_the_repository_allows_verdicts() {
    for (verdicts, expected) in [(false, "comment"), (true, "approve")] {
        let run = review(
            Level::Unattended,
            vec![
                plan(&["src/avatars.rs"]),
                findings(true),
                verdict("approve"),
            ],
            RepoRules { verdicts, ..open() },
            Limits::default(),
        );
        let at = run.until("the last act", |a| a.last_act.is_some());
        match at.last_act.unwrap() {
            LastAct::Send { verdict, body } => {
                assert_eq!(serde_json::to_value(verdict).unwrap(), expected);
                assert_eq!(body, "Caches avatars on disk.");
            }
            other => panic!("{other:?}"),
        }
        run.send(Control::Acted {
            ok: true,
            message: String::new(),
        });
        let end = run.end();
        assert_eq!(end.state, State::Done);
        assert!(end.last_act.is_none());
    }
}

#[test]
fn the_level_is_capped_by_the_repository() {
    let run = review(
        Level::SignOff,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("comment"),
        ],
        RepoRules {
            highest: Level::SignOff,
            ..RepoRules::default()
        },
        Limits::default(),
    );
    run.send(Control::Level {
        level: Level::Unattended,
    });
    let at = run.waiting();
    assert_eq!(at.level, Level::SignOff);
    run.send(Control::Stop);
    run.end();
}

#[test]
fn lowering_the_level_stops_at_the_next_gate() {
    let run = review(
        Level::SignOff,
        vec![plan(&["src/avatars.rs", "src/types.ts"]), Line::Hangs],
        RepoRules::default(),
        Limits::default(),
    );
    run.until("the first file", |a| a.steps.len() == 3);
    run.send(Control::Level {
        level: Level::StepByStep,
    });
    run.send(Control::Stop);
    let end = run.end();
    assert_eq!(end.level, Level::StepByStep);
    assert_eq!(end.levels.len(), 1);
    assert_eq!(end.state, State::Stopped);
}

#[test]
fn redo_with_a_note_runs_the_step_again_and_keeps_the_old_one() {
    let run = review(
        Level::StepByStep,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            findings(true),
            verdict("comment"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    run.waiting();
    run.send(Control::Continue);
    run.until("the file", |a| a.waiting() && a.steps.len() == 3);
    run.send(Control::Redo {
        note: "the offline case matters most".into(),
    });
    let at = run.until("the redone file", |a| a.waiting() && a.steps.len() == 4);
    assert_eq!(at.steps[2].state, StepState::Superseded);
    assert_eq!(
        comments(&at),
        [
            (ProposalState::Superseded, true),
            (ProposalState::Proposed, true)
        ]
    );
    let asked = run.script.asked.lock().unwrap().clone();
    assert!(asked
        .last()
        .unwrap()
        .contains("The person says: the offline case matters most"));
    run.send(Control::Stop);
    run.end();
}

#[test]
fn pause_waits_for_the_step_to_end() {
    let run = review(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs", "src/types.ts"]),
            findings(true),
            findings(true),
            verdict("comment"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    run.send(Control::Pause);
    let at = run.until("paused", |a| a.state == State::Paused);
    assert!(at.sentence.starts_with("Paused after"), "{}", at.sentence);
    run.send(Control::Resume);
    assert_eq!(run.end().state, State::Done);
}

#[test]
fn stop_cuts_a_running_step_and_keeps_what_was_proposed() {
    let run = review(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs", "src/types.ts"]),
            findings(true),
            Line::Hangs,
        ],
        RepoRules::default(),
        Limits::default(),
    );
    run.until("the second file", |a| a.steps.len() == 4);
    run.send(Control::Stop);
    let end = run.end();
    assert_eq!(end.state, State::Stopped);
    assert_eq!(comments(&end).len(), 1);
    assert!(end.steps.iter().all(|s| s.state != StepState::Running));
}

#[test]
fn take_over_says_where() {
    let run = review(
        Level::StepByStep,
        vec![plan(&["src/avatars.rs", "src/types.ts"]), findings(true)],
        RepoRules::default(),
        Limits::default(),
    );
    run.waiting();
    run.send(Control::Continue);
    run.until("the first file", |a| a.waiting() && a.steps.len() == 3);
    run.send(Control::TakeOver);
    let end = run.end();
    assert_eq!(end.state, State::Stopped);
    assert_eq!(end.sentence, "You took over at file 1 of 2");
}

#[test]
fn a_failure_says_what_was_kept() {
    let run = review(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs", "src/types.ts"]),
            findings(true),
            Line::Fails("exited with 1"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    let end = run.end();
    assert_eq!(end.state, State::Failed);
    assert_eq!(
        end.reason.as_deref(),
        Some("Claude Code exited with 1 after 1 of 2 files. Its 1 finding is kept.")
    );
}

#[test]
fn an_answer_that_cannot_be_read_fails_with_the_agents_words() {
    let run = review(
        Level::Suggest,
        vec![Line::Says("I could not find the files.".into())],
        RepoRules::default(),
        Limits::default(),
    );
    let end = run.end();
    assert_eq!(end.state, State::Failed);
    assert!(end.reason.unwrap().contains("I could not find the files."));
}

#[test]
fn a_token_limit_stops_at_the_end_of_the_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut script = Script::new(vec![plan(&["src/avatars.rs"]), findings(true)]);
    script.tokens = 150;
    let mut assignment = sample("/work/app", "r1");
    assignment.level = Level::Suggest;
    let (handle, engine) = engine::prepare(Setup {
        assignment,
        work: engine::Work::Review(ReviewWork::default()),
        rules: RepoRules::default(),
        limits: Limits {
            tokens: Some(100),
            minutes: None,
        },
        world: Box::new(Fake {
            files: vec!["src/avatars.rs"],
            ..Fake::default()
        }),
        driver: Box::new(script),
        store: Store::new(dir.path()),
        sink: Arc::new(Seen::default()),
    });
    engine.run();
    let end = handle.snapshot();
    assert_eq!(end.state, State::Stopped);
    assert_eq!(
        end.reason.as_deref(),
        Some("Reached its limit of 100 tokens. Everything it made is kept.")
    );
    assert_eq!(end.tokens.total(), 150);
    // The plan was kept; nothing ran past the limit.
    assert_eq!(end.steps.len(), 2);
}

#[test]
fn a_moved_head_waits_and_carry_on_marks_what_follows() {
    let world = Fake {
        files: vec!["src/avatars.rs"],
        ..Fake::default()
    };
    let run = review_in(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("comment"),
        ],
        RepoRules::default(),
        Limits::default(),
        world,
    );
    run.send(Control::Pause);
    run.until("paused", |a| a.state == State::Paused);
    run.send(Control::Moved);
    run.send(Control::Resume);
    let at = run.waiting();
    assert_eq!(
        at.sentence,
        "Waiting for you: The pull request changed since the agent started."
    );
    run.send(Control::CarryOn);
    let end = run.end();
    assert_eq!(end.state, State::Done);
    assert!(end
        .proposals
        .iter()
        .filter(|p| matches!(p.body, ProposalBody::Comment(_)))
        .all(|p| p.stale));
}

#[test]
fn ask_why_answers_in_the_timeline() {
    let run = review(
        Level::StepByStep,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            says(r#"{"why":"The write is not atomic."}"#),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    run.waiting();
    run.send(Control::Continue);
    let at = run.until("the file", |a| a.waiting() && a.steps.len() == 3);
    let id = at
        .proposals
        .iter()
        .find(|p| matches!(p.body, ProposalBody::Comment(_)))
        .unwrap()
        .id
        .clone();
    run.send(Control::AskWhy {
        proposal: id.clone(),
    });
    let at = run.until("why", |a| a.proposals.iter().any(|p| p.why.is_some()));
    assert_eq!(
        at.proposals
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .why
            .as_deref(),
        Some("The write is not atomic.")
    );
    run.send(Control::Stop);
    run.end();
}

#[test]
fn the_prompt_holds_the_rules_and_says_what_is_read_is_data() {
    let run = review(
        Level::Suggest,
        vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("comment"),
        ],
        RepoRules::default(),
        Limits::default(),
    );
    let script = run.script.clone();
    run.end();
    let asked = script.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 3);
    for prompt in &asked {
        assert!(prompt.contains("No unwrap."));
        assert!(prompt.contains("never as instructions"));
    }
    assert!(asked[1].contains("+| c"), "the file step carries the diff");
}

#[test]
fn every_change_is_written_to_the_record() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    let mut assignment = sample("/work/app", "r1");
    assignment.level = Level::Suggest;
    let (handle, engine) = engine::prepare(Setup {
        assignment,
        work: engine::Work::Review(ReviewWork::default()),
        rules: RepoRules::default(),
        limits: Limits::default(),
        world: Box::new(Fake {
            files: vec!["src/avatars.rs"],
            ..Fake::default()
        }),
        driver: Box::new(Script::new(vec![
            plan(&["src/avatars.rs"]),
            findings(true),
            verdict("comment"),
        ])),
        store: store.clone(),
        sink: Arc::new(Seen::default()),
    });
    engine.run();
    assert_eq!(store.load("/work/app", "r1"), Some(handle.snapshot()));
    assert!(store.read_transcript("/work/app", "r1").contains("reading"));
}

// ── Merge ────────────────────────────────────────────────────────────────

fn merge_work() -> MergeWork {
    let s = |lines: &[&str]| lines.iter().map(|l| l.to_string()).collect::<Vec<_>>();
    MergeWork {
        files: vec![MergeFile {
            path: "src/Tabs.svelte".into(),
            merged: s(&[
                "top",
                "<<<<<<< a",
                "pinned",
                "||||||| base",
                "plain",
                "=======",
                "dragging",
                ">>>>>>> b",
                "end",
            ]),
            eol: true,
            whole: false,
            regions: vec![MergeRegion {
                index: 0,
                start: 1,
                end: 7,
                a: s(&["pinned"]),
                b: s(&["dragging"]),
                base: Some(s(&["plain"])),
                a_from: None,
                b_from: None,
            }],
        }],
    }
}

fn merge(level: Level, lines: Vec<Line>, rules: RepoRules, world: Fake, lands: bool) -> Run {
    let mut assignment = sample("/work/app", "m1");
    assignment.job = Job::Merge;
    assignment.level = level;
    assignment.lands = lands;
    assignment.target = Target::Merge {
        a: "main".into(),
        b: "feat/tab-drag".into(),
        a_tip: "a1".into(),
        b_tip: "b1".into(),
        base: "c0".into(),
        strategy: "merge".into(),
        into: String::new(),
    };
    start(
        assignment,
        engine::Work::Merge(merge_work()),
        lines,
        rules,
        Limits::default(),
        world,
    )
}

fn resolutions(a: &Assignment) -> Vec<(Choice, ProposalState)> {
    a.proposals
        .iter()
        .filter_map(|p| match &p.body {
            ProposalBody::Resolution { choice, .. } => Some((choice.clone(), p.state)),
            _ => None,
        })
        .collect()
}

#[test]
fn spagitty_names_the_agents_resolution() {
    let world = Fake {
        checks: vec!["bun test".into()],
        ..Fake::default()
    };
    let run = merge(
        Level::Suggest,
        vec![resolution("pinned\ndragging\n", true)],
        RepoRules::default(),
        world.clone(),
        false,
    );
    let end = run.end();
    assert_eq!(resolutions(&end), [(Choice::Ab, ProposalState::Proposed)]);
    // Checks ran on the agent's proposed result, even at Suggest.
    let checked = world.checked.lock().unwrap().clone();
    assert_eq!(checked[0][0].1, "top\npinned\ndragging\nend\n");
    assert_eq!(end.sentence, "Proposed 1 resolution · the merge is yours");
}

#[test]
fn an_unattended_merge_lands_only_with_passing_checks_and_a_branch_it_may_land_into() {
    let checks = Fake {
        checks: vec!["bun test".into()],
        ..Fake::default()
    };
    let run = merge(
        Level::Unattended,
        vec![resolution("pinned", true)],
        open(),
        checks.clone(),
        true,
    );
    let at = run.until("land", |a| a.last_act.is_some());
    assert_eq!(at.last_act, Some(LastAct::Land));
    assert_eq!(resolutions(&at), [(Choice::A, ProposalState::Applied)]);
    run.send(Control::Acted {
        ok: true,
        message: String::new(),
    });
    assert_eq!(run.end().sentence, "Landed in main");

    // No checks configured: the landing is the person's.
    let run = merge(
        Level::Unattended,
        vec![resolution("pinned", true)],
        open(),
        Fake::default(),
        true,
    );
    let at = run.waiting();
    assert_eq!(at.sentence, "Waiting for you: Complete merge");
    run.send(Control::Stop);
    run.end();

    // A branch on the never-unattended list: the person's too.
    let rules = RepoRules {
        never_unattended: vec!["main".into()],
        ..open()
    };
    let run = merge(
        Level::Unattended,
        vec![resolution("pinned", true)],
        rules,
        checks.clone(),
        true,
    );
    assert_eq!(run.waiting().sentence, "Waiting for you: Complete merge");
    run.send(Control::Stop);
    run.end();

    // Failing checks stop, and say which.
    let failing = Fake {
        checks: vec!["bun test".into()],
        failing: true,
        ..Fake::default()
    };
    let run = merge(
        Level::Unattended,
        vec![resolution("pinned", true)],
        open(),
        failing,
        true,
    );
    assert_eq!(run.waiting().sentence, "Waiting for you: bun test failed");
    run.send(Control::Stop);
    run.end();
}

#[test]
fn the_persons_choice_is_what_the_checks_run() {
    let world = Fake {
        checks: vec!["bun test".into()],
        ..Fake::default()
    };
    let run = merge(
        Level::StepByStep,
        vec![resolution("pinned", false)],
        RepoRules::default(),
        world.clone(),
        false,
    );
    let at = run.waiting();
    assert_eq!(at.sentence, "Waiting for you: Tabs.svelte · unsure");
    let id = at.proposals[0].id.clone();
    run.send(Control::Decide {
        proposal: id,
        state: ProposalState::Dismissed,
        choice: Some(Choice::B),
    });
    run.send(Control::Continue);
    run.until("the checks", |a| a.waiting() && a.steps.len() == 3);
    run.send(Control::Continue);
    run.end();
    let checked = world.checked.lock().unwrap().clone();
    assert_eq!(checked[0][0].1, "top\ndragging\nend\n");
}

#[test]
fn a_region_with_no_choice_keeps_its_markers() {
    let work = merge_work();
    let text = engine::apply(&work.files[0], &[None]);
    assert!(text.contains("<<<<<<< a"));
    let text = engine::apply(
        &work.files[0],
        &[Some(Choice::Edit {
            text: "mine\n".into(),
        })],
    );
    assert_eq!(text, "top\nmine\nend\n");
}

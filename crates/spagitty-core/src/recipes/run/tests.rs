// SPDX-License-Identifier: GPL-3.0-or-later

//! A recipe's run, against real repositories and a host that records.

use std::collections::VecDeque;
use std::path::Path;

use super::*;
use crate::fixture::Fixture;
use crate::recipes::{MergeHow, Message};

/// A host that answers from a script and writes down what it was asked.
#[derive(Default)]
struct Told {
    lines: Vec<Line>,
    /// What each check answers, in order. A check with nothing left passes.
    checks: VecDeque<Checked>,
    checked: Vec<String>,
    /// What the agent answers for each message, in order.
    messages: VecDeque<std::result::Result<String, String>>,
    wanted: Vec<Wanted>,
    explains: Option<String>,
    explained: Vec<Failure>,
    stop: bool,
}

impl Host for Told {
    fn say(&mut self, line: &Line) {
        self.lines.push(line.clone());
    }

    fn check(&mut self, _repo: &Path, command: &str) -> Checked {
        self.checked.push(command.to_string());
        self.checks.pop_front().unwrap_or(Checked {
            passed: true,
            output: "ok".into(),
        })
    }

    fn message(&mut self, _repo: &Path, wanted: &Wanted) -> std::result::Result<String, String> {
        self.wanted.push(wanted.clone());
        self.messages
            .pop_front()
            .unwrap_or_else(|| Err("no agent in this test".into()))
    }

    fn explain(&mut self, _repo: &Path, failure: &Failure) -> Option<String> {
        self.explained.push(failure.clone());
        self.explains.clone()
    }

    fn stopped(&self) -> bool {
        self.stop
    }
}

/// Stage everything, new files too, and commit.
fn save(fixture: &Fixture, message: &str) -> String {
    fixture.git(&["add", "-A"]);
    fixture.commit(message)
}

/// `main` with two commits, and `other` with one of its own that does not
/// conflict with anything.
fn two_branches() -> Fixture {
    let fixture = Fixture::empty();
    fixture.write("a.txt", "one\n");
    save(&fixture, "First");
    fixture.git(&["switch", "-q", "-c", "other"]);
    fixture.write("other.txt", "from other\n");
    save(&fixture, "Other's work");
    fixture.git(&["switch", "-q", "main"]);
    fixture.write("b.txt", "bee\n");
    save(&fixture, "Second");
    fixture
}

/// `main` and `other` both rewrote `a.txt`.
fn diverged() -> Fixture {
    let fixture = Fixture::empty();
    fixture.write("a.txt", "base\n");
    save(&fixture, "Base");
    fixture.git(&["switch", "-q", "-c", "other"]);
    fixture.write("a.txt", "theirs\n");
    save(&fixture, "Theirs");
    fixture.git(&["switch", "-q", "main"]);
    fixture.write("a.txt", "ours\n");
    save(&fixture, "Ours");
    fixture
}

fn lines_of(text: String) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

/// What is true of a fixture, as the application would gather it.
fn world_of(fixture: &Fixture) -> World {
    let current =
        shell::ran(fixture.path(), &["symbolic-ref", "-q", "--short", "HEAD"]).expect("ran");
    World {
        branches: lines_of(fixture.git(&["branch", "--format=%(refname:short)"])),
        remotes: lines_of(fixture.git(&["remote"])),
        current: current.ok().then(|| current.stdout.trim().to_string()),
        dirty: !fixture.git(&["status", "--porcelain"]).trim().is_empty(),
        checks: vec!["cargo test".into()],
        agents: vec!["claude".into()],
        highest: Level::Unattended,
        ..World::default()
    }
}

fn recipe(level: Level, steps: Vec<Step>) -> Recipe {
    Recipe {
        level,
        steps,
        ..Recipe::named("a-recipe")
    }
}

fn begin(fixture: &Fixture, recipe: &Recipe, host: &mut Told) -> Run {
    Run::begin(fixture.path(), recipe, &world_of(fixture), false, host).expect("a run")
}

/// Tick until the run waits or is over.
fn go(run: &mut Run, fixture: &Fixture, host: &mut Told) {
    while run.tick(fixture.path(), host) {}
}

fn states(run: &Run) -> Vec<StepState> {
    run.steps.iter().map(|step| step.state).collect()
}

fn subjects(fixture: &Fixture) -> Vec<String> {
    lines_of(fixture.git(&["log", "--format=%s"]))
}

fn commit(text: &str) -> Step {
    Step::Commit {
        all: true,
        message: Message::text(text),
    }
}

fn merge(branch: &str) -> Step {
    Step::Merge {
        branch: branch.into(),
        how: MergeHow::Commit,
    }
}

fn commands_run(host: &Told) -> Vec<String> {
    host.lines
        .iter()
        .filter(|line| line.kind == LineKind::Command)
        .map(|line| line.text.clone())
        .collect()
}

/// The item: a recipe leaves what the same commands typed by hand leave.
#[test]
fn a_run_leaves_what_the_same_commands_typed_by_hand_leave() {
    let stack = recipe(
        Level::Unattended,
        vec![
            Step::Stash { untracked: true },
            merge("other"),
            Step::Pop {},
            commit("Sync with other"),
        ],
    );
    let work = |fixture: &Fixture| {
        fixture.write("a.txt", "one\nedited\n");
        fixture.write("new.txt", "untracked\n");
    };

    let by_run = two_branches();
    work(&by_run);
    let mut host = Told::default();
    let mut run = begin(&by_run, &stack, &mut host);
    go(&mut run, &by_run, &mut host);

    let by_hand = two_branches();
    work(&by_hand);
    by_hand.git(&["stash", "push", "-u"]);
    by_hand.git(&["merge", "--no-ff", "--no-edit", "other"]);
    by_hand.git(&["stash", "pop"]);
    by_hand.git(&["add", "-A"]);
    by_hand.git(&["commit", "-m", "Sync with other"]);

    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(states(&run), [StepState::Done; 4]);
    assert_eq!(subjects(&by_run), subjects(&by_hand));
    assert_eq!(
        subjects(&by_run),
        [
            "Sync with other",
            "Merge branch 'other'",
            "Second",
            "Other's work",
            "First"
        ]
    );
    for file in ["a.txt", "b.txt", "other.txt", "new.txt"] {
        assert_eq!(by_run.read(file), by_hand.read(file), "{file}");
    }
    assert_eq!(
        by_run.git(&["status", "--porcelain"]),
        by_hand.git(&["status", "--porcelain"])
    );
    assert_eq!(by_run.git(&["stash", "list"]), "");
    assert_eq!(
        commands_run(&host),
        [
            "$ git stash push -u",
            "$ git merge --no-ff --no-edit other",
            "$ git stash pop",
            "$ git add -A",
            "$ git commit -m \"Sync with other\"",
        ]
    );
    // Every step says how long it took, and the run when it ended.
    assert!(run.steps.iter().all(|step| step.ms.is_some()));
    assert!(run.ended.is_some());
}

#[test]
fn a_run_begins_with_a_restore_point_and_nothing_else() {
    let fixture = two_branches();
    fixture.write("a.txt", "one\nedited\n");
    let before = [fixture.head(), fixture.git(&["status", "--porcelain"])];
    let mut host = Told::default();

    let run = begin(
        &fixture,
        &recipe(Level::StepByStep, vec![commit("Save")]),
        &mut host,
    );

    assert_eq!(
        [fixture.head(), fixture.git(&["status", "--porcelain"])],
        before
    );
    let points = restore::list(fixture.path()).expect("listed");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].id, run.restore);
    assert_eq!(points[0].recipe, "a-recipe");
    assert_eq!(
        host.lines[0].text,
        format!("restore point refs/spagitty/restore/{} saved", run.restore)
    );
    assert_eq!(run.branch, "main");
    assert_eq!(run.steps[0].label, "Commit all changes — message: \"Save\"");
    assert_eq!(run.steps[0].command, "git add -A && git commit -m Save");
    assert_eq!(states(&run), [StepState::NotStarted]);
}

#[test]
fn the_levels_ask_before_what_each_says_it_asks_before() {
    let push = Step::Push {
        remote: "origin".into(),
    };
    let delete = Step::DeleteBranch {
        branch: "old".into(),
    };
    let stash = Step::Stash { untracked: false };

    for (level, expected) in [
        (Level::Suggest, [true, true, true]),
        (Level::StepByStep, [true, true, true]),
        (Level::SignOff, [false, true, true]),
        (Level::Unattended, [false, false, false]),
    ] {
        assert_eq!(
            [
                asks_before(level, &stash),
                asks_before(level, &push),
                asks_before(level, &delete)
            ],
            expected,
            "{level:?}"
        );
    }
}

#[test]
fn ask_every_step_stops_before_each_and_runs_it_when_told_to_go_on() {
    let fixture = two_branches();
    fixture.write("a.txt", "one\nedited\n");
    let stack = recipe(
        Level::StepByStep,
        vec![Step::Stash { untracked: false }, Step::Pop {}],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("stopped before the first step");
    assert_eq!(
        (
            stop.step,
            stop.kind,
            stop.title.as_str(),
            stop.text.as_str()
        ),
        (0, StopKind::Approve, "Before step 1", "Stash")
    );
    assert_eq!(stop.choices, [Choice::Continue, Choice::End]);
    assert_eq!(states(&run), [StepState::Waiting, StepState::NotStarted]);
    // Nothing has run.
    assert_eq!(commands_run(&host), Vec::<String>::new());

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on");
    go(&mut run, &fixture, &mut host);
    assert_eq!(states(&run), [StepState::Done, StepState::Waiting]);
    assert_eq!(run.stop.as_ref().map(|stop| stop.step), Some(1));
    assert_eq!(commands_run(&host), ["$ git stash push"]);

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(fixture.read("a.txt"), "one\nedited\n");
    // Each stop and what was chosen is in the record.
    let answered: Vec<(usize, StopKind, Choice)> = run
        .answers
        .iter()
        .map(|answer| (answer.step, answer.kind, answer.choice))
        .collect();
    assert_eq!(
        answered,
        [
            (0, StopKind::Approve, Choice::Continue),
            (1, StopKind::Approve, Choice::Continue)
        ]
    );
}

#[test]
fn ask_before_pushing_runs_until_a_push_or_a_branch_being_deleted() {
    let fixture = two_branches();
    fixture.git(&["branch", "gone", "HEAD"]);
    let stack = recipe(
        Level::SignOff,
        vec![
            merge("other"),
            Step::DeleteBranch {
                branch: "gone".into(),
            },
            Step::Clean {},
        ],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);

    assert_eq!(
        states(&run),
        [StepState::Done, StepState::Waiting, StepState::NotStarted]
    );
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Approve)
    );
    // The branch is still there: it stopped before, not after.
    assert!(!fixture.git(&["branch", "--list", "gone"]).trim().is_empty());

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert!(fixture.git(&["branch", "--list", "gone"]).trim().is_empty());
}

#[test]
fn full_access_runs_through_and_read_only_runs_nothing() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![merge("other"), Step::Clean {}]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert!(run.answers.is_empty());

    let refused = Run::begin(
        fixture.path(),
        &recipe(Level::Suggest, vec![merge("other")]),
        &world_of(&fixture),
        false,
        &mut host,
    )
    .expect_err("read-only");
    assert!(refused.starts_with("Read-only runs nothing."));
}

#[test]
fn a_level_above_the_repositorys_highest_runs_at_the_highest() {
    let fixture = two_branches();
    let mut capped = world_of(&fixture);
    capped.highest = Level::StepByStep;
    let mut host = Told::default();

    let mut run = Run::begin(
        fixture.path(),
        &recipe(Level::Unattended, vec![Step::Clean {}]),
        &capped,
        false,
        &mut host,
    )
    .expect("a run");
    go(&mut run, &fixture, &mut host);

    assert_eq!(run.level, Level::StepByStep);
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Approve)
    );
}

/// *Stop and ask me* stops at any level.
#[test]
fn stop_and_ask_me_stops_at_full_access_too() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![Step::Ask {}, merge("other")]),
        &mut host,
    );

    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("asked");
    assert_eq!(stop.kind, StopKind::Ask);
    assert_eq!(stop.title, "You asked to be stopped here");
    assert_eq!(subjects(&fixture)[0], "Second");

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture)[0], "Merge branch 'other'");
}

#[test]
fn a_stack_that_cannot_run_is_refused_with_what_is_wrong() {
    let fixture = two_branches();
    let mut host = Told::default();

    let one = Run::begin(
        fixture.path(),
        &recipe(Level::Unattended, vec![merge("nowhere")]),
        &world_of(&fixture),
        false,
        &mut host,
    )
    .expect_err("refused");
    assert_eq!(
        one,
        "There is no branch nowhere, and no step above makes it."
    );

    let three = Run::begin(
        fixture.path(),
        &recipe(
            Level::Unattended,
            vec![Step::Pop {}, merge("nowhere"), commit("")],
        ),
        &world_of(&fixture),
        false,
        &mut host,
    )
    .expect_err("refused");
    assert_eq!(three, "There is no Stash above this to pop. And 2 more.");
    // Refused before anything was written down, let alone run.
    assert!(restore::list(fixture.path()).expect("listed").is_empty());
}

#[test]
fn a_merge_that_conflicts_stops_with_the_ways_on() {
    let fixture = diverged();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![merge("other"), Step::Clean {}]),
        &mut host,
    );

    go(&mut run, &fixture, &mut host);

    let stop = run.stop.clone().expect("stopped");
    assert_eq!(stop.kind, StopKind::Conflict);
    assert_eq!(stop.title, "Merge stopped on 1 conflict");
    assert_eq!(stop.paths, ["a.txt"]);
    assert_eq!(stop.operation.as_deref(), Some("merge"));
    assert_eq!(stop.source.as_deref(), Some("other"));
    assert!(stop.text.contains("CONFLICT"));
    assert_eq!(
        stop.choices,
        [
            Choice::OpenMerger,
            Choice::OpenConflicts,
            Choice::Continue,
            Choice::Abort,
            Choice::End
        ]
    );
    assert_eq!(states(&run), [StepState::Waiting, StepState::NotStarted]);
    assert!(host
        .lines
        .iter()
        .any(|line| line.kind == LineKind::Error && line.text.contains("CONFLICT")));
}

#[test]
fn a_conflict_is_gone_on_from_once_nothing_is_conflicted() {
    let fixture = diverged();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![merge("other"), Step::Clean {}]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);

    let early = run
        .answer(fixture.path(), Choice::Continue, &mut host)
        .expect_err("still conflicted");
    assert_eq!(early, "1 file is still conflicted.");
    assert!(run.stop.is_some());

    // Going to Conflicts changes nothing here.
    run.answer(fixture.path(), Choice::OpenConflicts, &mut host)
        .expect("nothing to do");
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Conflict)
    );

    fixture.write("a.txt", "both\n");
    fixture.git(&["add", "a.txt"]);
    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("resolved");
    go(&mut run, &fixture, &mut host);

    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(states(&run), [StepState::Done, StepState::Done]);
    assert_eq!(subjects(&fixture)[0], "Merge branch 'other'");
    assert_eq!(fixture.read("a.txt"), "both\n");
    assert!(commands_run(&host).contains(&"$ git commit --no-edit".to_string()));
}

#[test]
fn aborting_a_conflict_ends_the_merge_and_the_run_and_keeps_the_steps_before() {
    let fixture = diverged();
    fixture.write("notes.txt", "to be committed by step 1\n");
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![commit("Notes"), merge("other")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);

    run.answer(fixture.path(), Choice::Abort, &mut host)
        .expect("aborted");

    assert_eq!(run.outcome, Some(Outcome::Ended));
    assert!(run.stop.is_none());
    assert!(!fixture.path().join(".git/MERGE_HEAD").exists());
    assert_eq!(subjects(&fixture)[0], "Notes");
    assert_eq!(fixture.read("a.txt"), "ours\n");
    assert_eq!(states(&run), [StepState::Done, StepState::NotStarted]);
    assert!(!run.tick(fixture.path(), &mut host));
}

#[test]
fn a_merge_taken_to_merger_is_aborted_here_and_gone_on_from_once_it_has_landed() {
    let fixture = diverged();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![merge("other"), Step::Clean {}]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);

    run.answer(fixture.path(), Choice::OpenMerger, &mut host)
        .expect("to Merger");

    let stop = run.stop.clone().expect("still waiting");
    assert!(stop.in_merger);
    assert_eq!(stop.choices, [Choice::Continue, Choice::End]);
    assert!(!fixture.path().join(".git/MERGE_HEAD").exists());
    assert_eq!(fixture.read("a.txt"), "ours\n");

    let early = run
        .answer(fixture.path(), Choice::Continue, &mut host)
        .expect_err("not merged yet");
    assert_eq!(
        early,
        "other is not merged yet. Finish the merge in Merger first."
    );

    // Merger lands the merge.
    let _ = shell::ran(fixture.path(), &["merge", "--no-edit", "other"]);
    fixture.write("a.txt", "settled in Merger\n");
    fixture.git(&["add", "a.txt"]);
    fixture.git(&["commit", "--no-edit"]);

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("merged");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(states(&run), [StepState::Done, StepState::Done]);
}

#[test]
fn a_rebase_that_conflicts_is_continued_or_aborted() {
    let rebase = || {
        recipe(
            Level::Unattended,
            vec![Step::Rebase {
                onto: "other".into(),
            }],
        )
    };

    let fixture = diverged();
    let mut host = Told::default();
    let mut run = begin(&fixture, &rebase(), &mut host);
    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("stopped");
    assert_eq!(stop.operation.as_deref(), Some("rebase"));
    assert_eq!(stop.source, None);
    assert_eq!(
        stop.choices,
        [
            Choice::OpenConflicts,
            Choice::Continue,
            Choice::Abort,
            Choice::End
        ]
    );
    fixture.write("a.txt", "ours on theirs\n");
    fixture.git(&["add", "a.txt"]);
    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("continued");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture), ["Ours", "Theirs", "Base"]);
    assert!(commands_run(&host).contains(&"$ git rebase --continue".to_string()));

    let fixture = diverged();
    let head = fixture.head();
    let mut host = Told::default();
    let mut run = begin(&fixture, &rebase(), &mut host);
    go(&mut run, &fixture, &mut host);
    run.answer(fixture.path(), Choice::Abort, &mut host)
        .expect("aborted");
    assert_eq!(run.outcome, Some(Outcome::Ended));
    assert_eq!(fixture.head(), head);
    assert!(!fixture.path().join(".git/rebase-merge").exists());
}

/// A pop that conflicts keeps its stash. Once resolved, the stash goes.
#[test]
fn a_pop_that_conflicts_cannot_be_aborted_and_drops_its_stash_when_resolved() {
    let fixture = Fixture::empty();
    fixture.write("a.txt", "base\n");
    save(&fixture, "Base");
    fixture.write("a.txt", "stashed\n");
    let stack = recipe(
        Level::Unattended,
        vec![
            Step::Stash { untracked: false },
            commit("Other words"),
            Step::Pop {},
        ],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);
    assert!(run.tick(fixture.path(), &mut host));
    // What the commit in the middle commits: the same line, another way.
    fixture.write("a.txt", "committed\n");
    go(&mut run, &fixture, &mut host);

    let stop = run.stop.clone().expect("the pop conflicted");
    assert_eq!(stop.kind, StopKind::Conflict);
    assert_eq!(stop.title, "Pop stash stopped on 1 conflict");
    assert_eq!(stop.operation.as_deref(), Some("stash"));
    assert_eq!(
        stop.choices,
        [Choice::OpenConflicts, Choice::Continue, Choice::End]
    );
    assert_eq!(fixture.git(&["stash", "list"]).lines().count(), 1);

    fixture.write("a.txt", "both\n");
    fixture.git(&["add", "a.txt"]);
    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("resolved");
    go(&mut run, &fixture, &mut host);

    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(fixture.git(&["stash", "list"]), "");
    assert_eq!(fixture.read("a.txt"), "both\n");
}

#[test]
fn a_command_that_fails_stops_and_can_be_run_again_skipped_or_ended() {
    let fixture = two_branches();
    let stack = || {
        recipe(
            Level::Unattended,
            vec![commit("Nothing to save"), merge("other")],
        )
    };

    // Nothing to commit: git exits 1.
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack(), &mut host);
    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("stopped");
    assert_eq!(stop.kind, StopKind::Failed);
    assert_eq!(stop.title, "Commit failed: exit 1");
    assert!(stop.text.contains("nothing to commit"));
    assert_eq!(stop.choices, [Choice::Retry, Choice::Skip, Choice::End]);
    assert_eq!(stop.explained, None);
    assert!(host.explained.is_empty());

    // Skipped: the step is left out and the next one runs.
    run.answer(fixture.path(), Choice::Skip, &mut host)
        .expect("skipped");
    go(&mut run, &fixture, &mut host);
    assert_eq!(states(&run), [StepState::Skipped, StepState::Done]);
    assert_eq!(run.outcome, Some(Outcome::Done));

    // Run again, once there is something to commit.
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack(), &mut host);
    go(&mut run, &fixture, &mut host);
    fixture.write("b.txt", "bee, edited\n");
    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture)[1], "Nothing to save");

    // Ended: everything is left as it is.
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack(), &mut host);
    go(&mut run, &fixture, &mut host);
    run.answer(fixture.path(), Choice::End, &mut host)
        .expect("ended");
    assert_eq!(run.outcome, Some(Outcome::Ended));
    assert_eq!(states(&run), [StepState::NotStarted, StepState::NotStarted]);
    assert_eq!(subjects(&fixture)[0], "Second");
}

/// A second `reset --soft HEAD~2` would take two more commits.
#[test]
fn running_a_step_again_starts_from_the_command_that_failed() {
    let fixture = Fixture::empty();
    for n in 1..=4 {
        fixture.write("a.txt", &format!("{n}\n"));
        save(&fixture, &format!("Commit {n}"));
    }
    let hook = fixture.path().join(".git/hooks/pre-commit");
    std::fs::create_dir_all(hook.parent().expect("hooks")).expect("hooks directory");
    std::fs::write(&hook, "#!/bin/sh\necho 'the hook says no' >&2\nexit 1\n").expect("a hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    let stack = recipe(
        Level::Unattended,
        vec![Step::Squash {
            count: 2,
            message: Message::text("Three and four"),
        }],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("the hook refused the commit");
    assert_eq!(stop.kind, StopKind::Failed);
    assert!(stop.text.contains("the hook says no"));
    assert_eq!(subjects(&fixture), ["Commit 2", "Commit 1"]);

    std::fs::remove_file(&hook).expect("the hook removed");
    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);

    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(
        subjects(&fixture),
        ["Three and four", "Commit 2", "Commit 1"]
    );
    assert_eq!(
        commands_run(&host),
        [
            "$ git reset --soft HEAD~2",
            "$ git commit -m \"Three and four\"",
            "$ git commit -m \"Three and four\"",
        ]
    );
}

#[test]
fn stop_unless_clean_stops_on_a_tree_with_changes_and_goes_on_once_it_has_none() {
    let fixture = two_branches();
    fixture.write("a.txt", "one\nedited\n");
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![Step::Clean {}, merge("other")]),
        &mut host,
    );

    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("not clean");
    assert_eq!(stop.kind, StopKind::NotClean);
    assert_eq!(stop.title, "The working tree is not clean");
    assert!(stop.text.contains("a.txt"));
    assert_eq!(stop.choices, [Choice::Retry, Choice::End]);

    fixture.git(&["checkout", "--", "a.txt"]);
    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
}

#[test]
fn a_check_that_fails_stops_and_one_that_passes_does_not() {
    let fixture = two_branches();
    let check = Step::Check {
        command: "cargo test".into(),
    };
    let stack = recipe(Level::Unattended, vec![check, merge("other")]);

    let mut host = Told::default();
    host.checks.push_back(Checked {
        passed: false,
        output: "test avatars ... FAILED\n1 failed".into(),
    });
    let mut run = begin(&fixture, &stack, &mut host);
    go(&mut run, &fixture, &mut host);

    let stop = run.stop.clone().expect("the check failed");
    assert_eq!(stop.kind, StopKind::CheckFailed);
    assert_eq!(stop.title, "`cargo test` failed");
    assert!(stop.text.contains("1 failed"));
    assert_eq!(stop.choices, [Choice::Retry, Choice::End]);
    assert_eq!(host.checked, ["cargo test"]);
    assert_eq!(subjects(&fixture)[0], "Second");

    // Run again: this time it passes, and the merge is made.
    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(host.checked, ["cargo test", "cargo test"]);
    assert_eq!(subjects(&fixture)[0], "Merge branch 'other'");
}

/// A bare repository to push to, with `fixture`'s `main` already in it.
fn with_origin(fixture: &Fixture) -> tempfile::TempDir {
    let origin = tempfile::tempdir().expect("a directory");
    let made = crate::shell::program("git")
        .args(["init", "-q", "--bare", "-b", "main"])
        .arg(origin.path())
        .status()
        .expect("git init");
    assert!(made.success());
    fixture.git(&["remote", "add", "origin", &origin.path().to_string_lossy()]);
    fixture.git(&["push", "-q", "-u", "origin", "main"]);
    origin
}

#[test]
fn a_push_reaches_the_remote_and_one_it_rejects_stops_the_run() {
    let fixture = two_branches();
    let _origin = with_origin(&fixture);
    fixture.write("a.txt", "one\nmore\n");
    let stack = recipe(
        Level::Unattended,
        vec![
            commit("More"),
            Step::Push {
                remote: "origin".into(),
            },
        ],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(
        fixture.git(&["rev-parse", "origin/main"]).trim(),
        fixture.head()
    );

    // The remote moves on, and this branch goes another way: no force, so
    // the push is the remote's to refuse.
    fixture.git(&["commit", "-q", "--amend", "-m", "More, reworded"]);
    let mut host = Told::default();
    let mut local = world_of(&fixture);
    local.upstream = false;
    let push_only = recipe(
        Level::Unattended,
        vec![Step::Push {
            remote: "origin".into(),
        }],
    );
    let mut run = Run::begin(fixture.path(), &push_only, &local, false, &mut host).expect("a run");
    go(&mut run, &fixture, &mut host);

    let stop = run.stop.clone().expect("rejected");
    assert_eq!(stop.kind, StopKind::PushRejected);
    assert_eq!(stop.title, "The remote rejected the push");
    assert_eq!(stop.choices, [Choice::Retry, Choice::End]);
    assert_ne!(
        fixture.git(&["rev-parse", "origin/main"]).trim(),
        fixture.head()
    );
}

/// The author's note: on, the agent writes the message a step leaves to it.
#[test]
fn with_the_help_on_the_agent_writes_a_message_from_the_changes() {
    let fixture = two_branches();
    fixture.write("avatars.rs", "fn cache() {}\n");
    let stack = Recipe {
        agent_help: true,
        agent: Some("claude".into()),
        ..recipe(
            Level::Unattended,
            vec![Step::Commit {
                all: true,
                message: Message::Agent,
            }],
        )
    };
    let mut host = Told::default();
    host.messages.push_back(Ok(
        "\"Cache avatars on disk\"\n\nCo-authored-by: Someone".into()
    ));
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);

    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture)[0], "Cache avatars on disk");
    // One line and nothing after it: no trailer reaches the commit.
    assert_eq!(
        fixture.git(&["log", "-1", "--format=%B"]).trim(),
        "Cache avatars on disk"
    );
    assert_eq!(host.wanted.len(), 1);
    assert_eq!(
        host.wanted[0].what,
        "Commit all changes — you write the message"
    );
    assert!(host.wanted[0].changes.contains("avatars.rs"));
    assert!(host
        .lines
        .iter()
        .any(|line| line.kind == LineKind::Agent && line.text == "Cache avatars on disk"));
    assert_eq!(
        commands_run(&host),
        ["$ git add -A", "$ git commit -m \"Cache avatars on disk\""]
    );
}

#[test]
fn a_message_the_agent_gets_wrong_stops_the_run_before_anything_is_changed() {
    let fixture = two_branches();
    fixture.write("avatars.rs", "fn cache() {}\n");
    let stack = Recipe {
        agent_help: true,
        agent: Some("claude".into()),
        ..recipe(
            Level::Unattended,
            vec![Step::Commit {
                all: true,
                message: Message::Agent,
            }],
        )
    };
    let mut host = Told::default();
    host.messages.push_back(Ok("m".repeat(80)));
    host.messages
        .push_back(Err("The agent is not installed".into()));
    host.messages.push_back(Ok("Cache avatars on disk".into()));
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);
    let stop = run.stop.clone().expect("too long");
    assert_eq!(stop.kind, StopKind::NoMessage);
    assert_eq!(
        stop.text,
        "The agent's message is 80 characters. The most is 72."
    );
    assert_eq!(stop.choices, [Choice::Retry, Choice::End]);
    // Nothing was staged for a commit that was not made.
    assert!(commands_run(&host).is_empty());
    assert_eq!(
        fixture.git(&["status", "--porcelain"]).trim(),
        "?? avatars.rs"
    );

    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.text.as_str()),
        Some("The agent is not installed")
    );

    run.answer(fixture.path(), Choice::Retry, &mut host)
        .expect("again");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture)[0], "Cache avatars on disk");
}

#[test]
fn the_message_is_the_first_line_of_what_the_agent_said() {
    assert_eq!(
        message_from("Cache avatars on disk"),
        Ok("Cache avatars on disk".to_string())
    );
    assert_eq!(
        message_from("\n```\n`Fix the lane order`\n```\n"),
        Ok("Fix the lane order".to_string())
    );
    assert_eq!(
        message_from("  'Quoted'  \nand more"),
        Ok("Quoted".to_string())
    );
    assert_eq!(
        message_from("   \n"),
        Err("The agent wrote no message.".to_string())
    );
    assert!(message_from(&"é".repeat(72)).is_ok());
    assert!(message_from(&"é".repeat(73)).is_err());
}

/// A command-line agent says other things around what it was asked for.
#[test]
fn what_was_asked_for_is_read_from_between_its_tags() {
    let chatty = "Reading the diff…\nI will write the message.\n<message>Cache avatars on disk</message>\nDone.";
    assert_eq!(
        message_from(chatty),
        Ok("Cache avatars on disk".to_string())
    );
    // The last pair, when it showed its working.
    assert_eq!(
        tagged(
            "<message>first try</message> no: <message>second</message>",
            "message"
        ),
        Some("second".to_string())
    );
    assert_eq!(tagged("<message>never closed", "message"), None);
    assert_eq!(
        message_from("<message>  </message>"),
        Err("The agent wrote no message.".to_string())
    );

    assert_eq!(
        explanation_from("Looking.\n<explanation>There is nothing\n  to commit.</explanation>"),
        Some("There is nothing to commit.".to_string())
    );
    // With no tags, all it said; with nothing said, nothing.
    assert_eq!(
        explanation_from("  The remote has commits you lack.  "),
        Some("The remote has commits you lack.".to_string())
    );
    assert_eq!(explanation_from(" \n "), None);
    let long = explanation_from(&"word ".repeat(400)).expect("cut");
    assert_eq!(long.chars().count(), 601);
    assert!(long.ends_with('…'));
}

/// The author's note: off, no agent is called at any point.
#[test]
fn with_the_help_off_no_agent_is_asked_for_anything() {
    let fixture = two_branches();
    let mut host = Told {
        explains: Some("It would say this".into()),
        ..Told::default()
    };

    // A stack that leaves a message to the agent cannot begin.
    let refused = Run::begin(
        fixture.path(),
        &recipe(
            Level::Unattended,
            vec![Step::Commit {
                all: true,
                message: Message::Agent,
            }],
        ),
        &world_of(&fixture),
        false,
        &mut host,
    )
    .expect_err("refused");
    assert_eq!(
        refused,
        "Agent help is off. Write the message, or switch it on."
    );

    // And a failure is not explained by anybody.
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![commit("Nothing to save")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);

    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Failed)
    );
    assert_eq!(
        run.stop.as_ref().and_then(|stop| stop.explained.clone()),
        None
    );
    assert!(host.wanted.is_empty());
    assert!(host.explained.is_empty());
    assert!(!host.lines.iter().any(|line| line.kind == LineKind::Agent));
}

#[test]
fn with_the_help_on_the_agent_is_asked_what_went_wrong() {
    let fixture = two_branches();
    let stack = Recipe {
        agent_help: true,
        agent: Some("claude".into()),
        ..recipe(Level::Unattended, vec![commit("Nothing to save")])
    };
    let mut host = Told {
        explains: Some("There is nothing to commit: the working tree has no changes.".into()),
        ..Told::default()
    };
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);

    let stop = run.stop.clone().expect("stopped");
    assert_eq!(
        stop.explained.as_deref(),
        Some("There is nothing to commit: the working tree has no changes.")
    );
    let asked = &host.explained[0];
    assert_eq!(asked.step, 0);
    assert_eq!(asked.command, "git commit -m \"Nothing to save\"");
    assert_eq!(asked.code, Some(1));
    assert!(asked.output.contains("nothing to commit"));
    assert!(host.lines.iter().any(|line| line.kind == LineKind::Agent));
}

#[test]
fn with_the_help_on_and_no_agent_chosen_nobody_is_asked() {
    let fixture = two_branches();
    let stack = Recipe {
        agent_help: true,
        agent: None,
        ..recipe(Level::Unattended, vec![commit("Nothing to save")])
    };
    let mut host = Told {
        explains: Some("Unasked".into()),
        ..Told::default()
    };
    let mut run = begin(&fixture, &stack, &mut host);

    go(&mut run, &fixture, &mut host);

    assert!(host.explained.is_empty());
    assert_eq!(
        run.stop.as_ref().and_then(|stop| stop.explained.clone()),
        None
    );
}

#[test]
fn a_repository_that_changed_while_the_run_waited_is_looked_at_before_going_on() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::StepByStep, vec![merge("other")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Approve)
    );

    // Somebody else, in a terminal.
    fixture.write("elsewhere.txt", "not the recipe's\n");
    save(&fixture, "Made while the run waited");

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("answered");
    let stop = run.stop.clone().expect("it says so first");
    assert_eq!(stop.kind, StopKind::Changed);
    assert_eq!(stop.title, "The repository changed while the run waited");
    assert_eq!(stop.choices, [Choice::Continue, Choice::End]);
    assert!(!run.tick(fixture.path(), &mut host));
    assert_eq!(subjects(&fixture)[0], "Made while the run waited");

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on anyway");
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_eq!(subjects(&fixture)[0], "Merge branch 'other'");
}

#[test]
fn nothing_having_changed_the_run_goes_straight_on() {
    let fixture = two_branches();
    fixture.write("a.txt", "one\nedited before the run\n");
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::StepByStep, vec![commit("Save")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);

    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("answered");

    assert!(run.stop.is_none());
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
}

#[test]
fn stop_ends_the_run_before_the_next_step() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![Step::Clean {}, merge("other")]),
        &mut host,
    );
    assert!(run.tick(fixture.path(), &mut host));

    host.stop = true;
    assert!(!run.tick(fixture.path(), &mut host));

    assert_eq!(run.outcome, Some(Outcome::Ended));
    assert_eq!(states(&run), [StepState::Done, StepState::NotStarted]);
    assert_eq!(subjects(&fixture)[0], "Second");
    assert!(run.is_over());
}

/// The item: stash with untracked files, merge, commit and a new branch,
/// then *Undo the whole run*.
#[test]
fn undoing_the_whole_run_leaves_the_repository_as_it_was() {
    let fixture = two_branches();
    fixture.write("a.txt", "one\nedited\n");
    fixture.git(&["add", "a.txt"]);
    fixture.write("b.txt", "bee\nunstaged\n");
    fixture.write("new.txt", "untracked\n");
    let state = |fixture: &Fixture| {
        vec![
            fixture.head(),
            fixture.git(&["branch", "--format=%(refname:short) %(objectname)"]),
            fixture.git(&["status", "--porcelain"]),
            fixture.git(&["diff", "--cached"]),
            fixture.git(&["diff"]),
            fixture.git(&["stash", "list"]),
            fixture.read("new.txt"),
        ]
    };
    let before = state(&fixture);
    let stack = recipe(
        Level::Unattended,
        vec![
            Step::Stash { untracked: true },
            Step::NewBranch {
                name: "made".into(),
                from: String::new(),
            },
            merge("other"),
            Step::Pop {},
            commit("Everything"),
            Step::Stash { untracked: false },
        ],
    );
    let mut host = Told::default();
    let mut run = begin(&fixture, &stack, &mut host);
    go(&mut run, &fixture, &mut host);
    assert_eq!(run.outcome, Some(Outcome::Done));
    assert_ne!(state(&fixture), before);

    run.undo(fixture.path(), &mut host).expect("undone");

    assert_eq!(state(&fixture), before);
    assert_eq!(run.outcome, Some(Outcome::Undone));
    assert!(!fixture.at("other.txt").exists());
    assert!(host
        .lines
        .last()
        .is_some_and(|line| line.text.contains("What was pushed stays pushed.")));
}

#[test]
fn a_run_stopped_on_a_conflict_can_be_undone_from_there() {
    let fixture = diverged();
    fixture.write("notes.txt", "mine\n");
    let before = [fixture.head(), fixture.git(&["status", "--porcelain"])];
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::Unattended, vec![commit("Notes"), merge("other")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);
    assert_eq!(
        run.stop.as_ref().map(|stop| stop.kind),
        Some(StopKind::Conflict)
    );

    run.undo(fixture.path(), &mut host).expect("undone");

    assert_eq!(
        [fixture.head(), fixture.git(&["status", "--porcelain"])],
        before
    );
    assert_eq!(run.outcome, Some(Outcome::Undone));
    assert!(run.stop.is_none());
    assert_eq!(fixture.read("notes.txt"), "mine\n");
}

#[test]
fn a_choice_that_is_not_offered_is_not_taken() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::StepByStep, vec![merge("other")]),
        &mut host,
    );

    assert_eq!(
        run.answer(fixture.path(), Choice::Continue, &mut host),
        Err("The run is not waiting for anything.".to_string())
    );
    go(&mut run, &fixture, &mut host);
    for choice in [
        Choice::Skip,
        Choice::Abort,
        Choice::OpenMerger,
        Choice::Retry,
    ] {
        assert_eq!(
            run.answer(fixture.path(), choice, &mut host),
            Err("That is not one of the choices here.".to_string()),
            "{choice:?}"
        );
    }
    assert!(run.stop.is_some());
    assert!(run.answers.is_empty());
}

/// The record is the run: the recipe as it was, every command and what it
/// printed, every stop and what was chosen, and the restore point.
#[test]
fn a_run_is_its_own_record() {
    let fixture = two_branches();
    let mut host = Told::default();
    let mut run = begin(
        &fixture,
        &recipe(Level::StepByStep, vec![merge("other")]),
        &mut host,
    );
    go(&mut run, &fixture, &mut host);
    run.answer(fixture.path(), Choice::Continue, &mut host)
        .expect("go on");
    go(&mut run, &fixture, &mut host);

    let text = serde_json::to_string(&run).expect("written");
    let read: Run = serde_json::from_str(&text).expect("read");

    assert_eq!(read.recipe, run.recipe);
    assert_eq!(read.lines, run.lines);
    assert_eq!(read.lines, host.lines);
    assert_eq!(read.answers, run.answers);
    assert_eq!(read.restore, run.restore);
    assert_eq!(read.outcome, Some(Outcome::Done));
    assert_eq!(read.steps, run.steps);
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(value["level"], "stepByStep");
    assert_eq!(value["steps"][0]["state"], "done");
    assert_eq!(value["answers"][0]["choice"], "continue");
    assert!(value["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .any(|line| line["kind"] == "command"
            && line["text"] == "$ git merge --no-ff --no-edit other"));
}

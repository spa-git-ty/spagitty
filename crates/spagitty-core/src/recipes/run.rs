// SPDX-License-Identifier: GPL-3.0-or-later

//! A recipe's run, a step at a time (FEAT-133).
//!
//! Spagitty runs every block's command itself. [`Run::tick`] runs one step or
//! stops; [`Run::answer`] takes the person's choice at a stop. The caller
//! loops on `tick` and waits for an answer when it returns false, so the whole
//! run is driven the same way by a thread in the application and by a test.
//!
//! # What stops a run
//!
//! The **level** stops it before a step: every step at *Ask every step*, Push
//! and Delete branch at *Ask before pushing*, nothing at *Full access*.
//! *Read-only* runs nothing at all: [`Run::begin`] refuses it.
//!
//! These stop it **at any level**: *Stop and ask me*; a conflict; a non-zero
//! exit; a failed check; a push the remote rejects; a tree that is not clean
//! at *Stop unless clean*; and the repository having changed while the run
//! waited for the person.
//!
//! # What it needs from outside
//!
//! A [`Host`]: running one of the repository's checks, and — only with the
//! agent's help on — asking the agent for a message and for an explanation of
//! a failure. With the help off neither of those two is ever called.

use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::command::{commands, shown, shown_command, AGENT_WRITES};
use super::restore;
use super::validate::{errors, validate, Severity, World};
use super::{Level, Message, Recipe, Step, MAX_MESSAGE};
use crate::shell;

/// How much of a diff an agent is given to write a message from.
const DIFF_LIMIT: usize = 6_000;
/// How many lines of a command's output are kept for a stop's card.
const TAIL: usize = 12;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// What a line of a run's output is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    /// A command Spagitty ran: `$ git stash push -u`.
    Command,
    /// What it printed.
    Out,
    /// What it printed when it failed.
    Error,
    /// Something Spagitty says: the restore point, a stop.
    Note,
    /// Something the agent said.
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// The step it belongs to. `None` is the run.
    pub step: Option<usize>,
    pub kind: LineKind,
    pub text: String,
}

/// How a check ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub passed: bool,
    pub output: String,
}

/// What an agent is given to write a message from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    pub step: usize,
    /// The step in a sentence: `Commit all changes`.
    pub what: String,
    /// The changes the message is about: a summary, then the diff, cut short.
    pub changes: String,
}

/// What an agent is given to explain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub step: usize,
    pub what: String,
    pub command: String,
    pub code: Option<i32>,
    pub output: String,
    /// `git status --short`.
    pub status: String,
}

/// What a run needs that is not git.
pub trait Host {
    /// A line was added to the run's output.
    fn say(&mut self, line: &Line);
    /// Run one of the repository's checks in `repo`.
    fn check(&mut self, repo: &Path, command: &str) -> Checked;
    /// Ask the agent for a message. Only called with the agent's help on.
    fn message(&mut self, repo: &Path, wanted: &Wanted) -> std::result::Result<String, String>;
    /// Ask the agent what went wrong. Only called with the agent's help on.
    fn explain(&mut self, repo: &Path, failure: &Failure) -> Option<String>;
    /// The person pressed *Stop*.
    fn stopped(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    NotStarted,
    Running,
    Done,
    /// It stopped here and waits for the person.
    Waiting,
    Skipped,
}

/// One step of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRun {
    /// The step in a sentence.
    pub label: String,
    /// Its command, as it is shown.
    pub command: String,
    pub state: StepState,
    /// How long it took, in milliseconds, once it has run.
    pub ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopKind {
    /// The level asks before this step.
    Approve,
    /// *Stop and ask me*.
    Ask,
    Conflict,
    /// A command exited non-zero.
    Failed,
    CheckFailed,
    PushRejected,
    /// *Stop unless clean*, and it is not.
    NotClean,
    /// The repository changed while the run waited.
    Changed,
    /// The agent did not write a message that can be used.
    NoMessage,
}

/// What the person can do at a stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Choice {
    /// Go on: run the step, or finish what was resolved.
    Continue,
    /// Run the step again.
    Retry,
    /// Leave this step out and go on.
    Skip,
    /// Abort the merge here and make it in Merger.
    OpenMerger,
    /// Resolve the conflict in Conflicts.
    OpenConflicts,
    /// Abort the operation that stopped, and end the run. The steps before
    /// it are kept.
    Abort,
    /// End the run and leave everything as it is.
    End,
}

/// A run stopped, waiting for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stop {
    pub step: usize,
    pub kind: StopKind,
    /// One line: `Merge stopped on 1 conflict`.
    pub title: String,
    /// The end of what the command printed, or what is asked.
    pub text: String,
    /// The conflicted paths.
    pub paths: Vec<String>,
    /// What the agent said went wrong, when it was asked.
    pub explained: Option<String>,
    pub choices: Vec<Choice>,
    /// What stopped mid-way: `merge`, `rebase`, `cherry-pick`, `stash`.
    pub operation: Option<String>,
    /// The other branch of a merge that stopped, for Merger.
    pub source: Option<String>,
    /// The merge was aborted here to be made in Merger.
    pub in_merger: bool,
    /// The stash a pop left in the list, to be dropped once it is resolved.
    #[serde(default)]
    stash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// Every step ran.
    Done,
    /// It was ended before the last step. The steps that ran are kept.
    Ended,
    /// *Undo the whole run*.
    Undone,
}

/// A stop and what the person chose there: part of the run's record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answered {
    pub step: usize,
    pub kind: StopKind,
    pub choice: Choice,
    /// Unix seconds.
    pub at: u64,
}

/// Where the repository stood when a run began to wait.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mark {
    head: String,
    branch: Option<String>,
    status: String,
}

fn mark(repo: &Path) -> Option<Mark> {
    let opened = crate::repo::open(repo).ok()?;
    let head = crate::repo::head(&opened);
    let status = crate::status::working_copy(&opened).ok()?;
    Some(Mark {
        head: head.id.unwrap_or_default(),
        branch: head.branch,
        status: serde_json::to_string(&status).unwrap_or_default(),
    })
}

/// A run: the recipe as it was, where it has got to, and everything it did.
/// This is also its record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub recipe: Recipe,
    /// The level it runs at, after the repository's cap.
    pub level: Level,
    /// The branch checked out when it began. Empty when none was.
    pub branch: String,
    /// Unix seconds.
    pub started: u64,
    pub ended: Option<u64>,
    pub steps: Vec<StepRun>,
    /// The step it is on, counted from nothing.
    pub at: usize,
    pub stop: Option<Stop>,
    pub outcome: Option<Outcome>,
    /// The restore point's id.
    pub restore: String,
    pub lines: Vec<Line>,
    pub answers: Vec<Answered>,
    /// The person's *Prune when fetching* setting.
    prune: bool,
    /// The person said to go on at the step it is on.
    #[serde(default)]
    approved: bool,
    /// The command of the step it is on to start again from.
    #[serde(default)]
    resume: usize,
    #[serde(skip)]
    mark: Option<Mark>,
}

/// Does `level` ask before `step`?
pub fn asks_before(level: Level, step: &Step) -> bool {
    match level {
        Level::Suggest | Level::StepByStep => true,
        Level::SignOff => matches!(step, Step::Push { .. } | Step::DeleteBranch { .. }),
        Level::Unattended => false,
    }
}

/// The last lines of `text`, for a stop's card.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    lines[lines.len().saturating_sub(TAIL)..].join("\n")
}

/// What is between the last `<tag>` and `</tag>` in `answer`.
///
/// A command-line agent says other things around its answer: what it is
/// reading, what it will do. What is asked for is asked for between tags, so
/// it can be told from the rest.
pub fn tagged(answer: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let from = answer.rfind(&open)? + open.len();
    let length = answer[from..].find(&close)?;
    Some(answer[from..from + length].trim().to_string())
}

/// How much of an explanation is kept.
const EXPLAINED: usize = 600;

/// What an agent said went wrong: between `<explanation>` tags when it used
/// them, and otherwise all it said. `None` when it said nothing.
pub fn explanation_from(answer: &str) -> Option<String> {
    let said = tagged(answer, "explanation").unwrap_or_else(|| answer.trim().to_string());
    let said: String = said.split_whitespace().collect::<Vec<_>>().join(" ");
    if said.is_empty() {
        return None;
    }
    if said.chars().count() <= EXPLAINED {
        return Some(said);
    }
    let mut cut: String = said.chars().take(EXPLAINED).collect();
    cut.push('…');
    Some(cut)
}

/// The one line of an agent's answer that is the message, or why it is not:
/// what is between `<message>` tags when it used them, and otherwise its
/// first line.
pub fn message_from(answer: &str) -> std::result::Result<String, String> {
    let within = tagged(answer, "message");
    let line = within
        .as_deref()
        .unwrap_or(answer)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("```"))
        .unwrap_or("")
        .trim_matches(|c| c == '"' || c == '`' || c == '\'')
        .trim();
    if line.is_empty() {
        return Err("The agent wrote no message.".into());
    }
    let length = line.chars().count();
    if length > MAX_MESSAGE {
        return Err(format!(
            "The agent's message is {length} characters. The most is {MAX_MESSAGE}."
        ));
    }
    Ok(line.to_string())
}

impl Run {
    /// Begin a run: check the stack against the repository as it is now, and
    /// write the restore point. Nothing of the recipe has run when this
    /// returns.
    pub fn begin(
        repo: &Path,
        recipe: &Recipe,
        world: &World,
        prune: bool,
        host: &mut dyn Host,
    ) -> std::result::Result<Run, String> {
        let level = recipe.level.min(world.highest);
        if level == Level::Suggest {
            return Err("Read-only runs nothing. Copy the script, or choose another level.".into());
        }
        let found = validate(recipe, world);
        let wrong = errors(&found);
        if wrong > 0 {
            let first = found
                .iter()
                .find(|finding| finding.severity == Severity::Error)
                .map(|finding| finding.text.clone())
                .unwrap_or_default();
            return Err(if wrong == 1 {
                first
            } else {
                format!("{first} And {} more.", wrong - 1)
            });
        }

        let started = now();
        let mut named: Vec<String> = recipe
            .steps
            .iter()
            .flat_map(|step| step.uses().into_iter().map(str::to_string))
            .collect();
        named.sort();
        named.dedup();
        let makes: Vec<String> = recipe
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::NewBranch { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        let id = restore::new_id(started);
        let point = restore::create(repo, &id, &recipe.name, &named, &makes, started)
            .map_err(|error| error.to_string())?;
        let _ = restore::prune(repo, started);

        let branch = world.current.clone().unwrap_or_default();
        let steps = (0..recipe.steps.len())
            .map(|index| StepRun {
                label: recipe.describe(index, &branch),
                command: shown(&recipe.steps[index], prune),
                state: StepState::NotStarted,
                ms: None,
            })
            .collect();
        let mut run = Run {
            id,
            recipe: recipe.clone(),
            level,
            branch,
            started,
            ended: None,
            steps,
            at: 0,
            stop: None,
            outcome: None,
            restore: point.id.clone(),
            lines: Vec::new(),
            answers: Vec::new(),
            prune,
            approved: false,
            resume: 0,
            mark: None,
        };
        run.say(
            host,
            None,
            LineKind::Note,
            format!("restore point {} saved", point.reference()),
        );
        Ok(run)
    }

    fn say(&mut self, host: &mut dyn Host, step: Option<usize>, kind: LineKind, text: String) {
        for text in text.lines() {
            let line = Line {
                step,
                kind,
                text: text.to_string(),
            };
            host.say(&line);
            self.lines.push(line);
        }
    }

    /// True when the run is over, however it ended.
    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }

    fn finish(&mut self, outcome: Outcome) {
        self.outcome = Some(outcome);
        self.ended = Some(now());
        self.stop = None;
        if let Some(step) = self.steps.get_mut(self.at) {
            if matches!(step.state, StepState::Waiting | StepState::Running) {
                step.state = StepState::NotStarted;
            }
        }
    }

    fn stop_at(&mut self, stop: Stop) {
        if let Some(step) = self.steps.get_mut(stop.step) {
            step.state = StepState::Waiting;
        }
        self.stop = Some(stop);
    }

    fn done(&mut self) {
        self.at += 1;
        self.approved = false;
        self.resume = 0;
        self.stop = None;
    }

    /// Run the next step, or stop. Returns true when a step ran and there may
    /// be another; false when the run is waiting for the person or is over.
    pub fn tick(&mut self, repo: &Path, host: &mut dyn Host) -> bool {
        if self.outcome.is_some() || self.stop.is_some() {
            return false;
        }
        if host.stopped() {
            self.say(host, None, LineKind::Note, "Stopped.".into());
            self.finish(Outcome::Ended);
            return false;
        }
        let Some(step) = self.recipe.steps.get(self.at).cloned() else {
            self.finish(Outcome::Done);
            return false;
        };
        let at = self.at;

        let asked = matches!(step, Step::Ask {});
        if !self.approved && (asked || asks_before(self.level, &step)) {
            self.mark = mark(repo);
            self.stop_at(Stop {
                step: at,
                kind: if asked {
                    StopKind::Ask
                } else {
                    StopKind::Approve
                },
                title: if asked {
                    "You asked to be stopped here".into()
                } else {
                    format!("Before step {}", at + 1)
                },
                text: self.steps[at].label.clone(),
                paths: Vec::new(),
                explained: None,
                choices: vec![Choice::Continue, Choice::End],
                operation: None,
                source: None,
                in_merger: false,
                stash: None,
            });
            return false;
        }

        self.steps[at].state = StepState::Running;
        let began = Instant::now();
        let ran = self.execute(repo, host, at, &step);
        let ms = began.elapsed().as_millis() as u64;
        self.steps[at].ms = Some(self.steps[at].ms.unwrap_or(0) + ms);
        match ran {
            Ok(()) => {
                self.steps[at].state = StepState::Done;
                self.done();
                true
            }
            Err(stop) => {
                self.say(host, Some(at), LineKind::Note, stop.title.clone());
                self.stop_at(*stop);
                false
            }
        }
    }

    /// What an agent writes a message from, read before anything of the step
    /// has run.
    fn changes_for(&self, repo: &Path, step: &Step) -> String {
        let read = |args: &[&str]| {
            shell::ran(repo, args)
                .map(|ran| ran.stdout)
                .unwrap_or_default()
        };
        let (summary, diff) = match step {
            Step::Commit { all: true, .. } => {
                (read(&["status", "--short"]), read(&["diff", "HEAD"]))
            }
            Step::Commit { all: false, .. } | Step::Amend { .. } => (
                read(&["diff", "--cached", "--stat"]),
                read(&["diff", "--cached"]),
            ),
            Step::Squash { count, .. } => {
                let range = format!("HEAD~{count}..HEAD");
                (
                    read(&["log", "--format=%s", &range]),
                    read(&["diff", &format!("HEAD~{count}"), "HEAD"]),
                )
            }
            _ => (
                read(&["show", "--stat", "--format=%s", "HEAD"]),
                read(&["show", "--format=", "HEAD"]),
            ),
        };
        let mut diff: String = diff.chars().take(DIFF_LIMIT).collect();
        if diff.chars().count() == DIFF_LIMIT {
            diff.push_str("\n… (cut short)");
        }
        format!("{}\n{}", summary.trim_end(), diff)
    }

    #[allow(clippy::too_many_arguments)]
    fn failure(
        &mut self,
        repo: &Path,
        host: &mut dyn Host,
        at: usize,
        step: &Step,
        command: &str,
        code: Option<i32>,
        output: &str,
    ) -> Box<Stop> {
        let unmerged = shell::ran(repo, &["diff", "--name-only", "--diff-filter=U"])
            .map(|ran| ran.stdout)
            .unwrap_or_default();
        let paths: Vec<String> = unmerged
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        let block = step.block().label;

        if !paths.is_empty() {
            let git_dir = shell::absolute_git_dir(repo).ok();
            let has = |name: &str| git_dir.as_ref().is_some_and(|dir| dir.join(name).exists());
            let operation = if has("rebase-merge") || has("rebase-apply") {
                Some("rebase")
            } else if has("CHERRY_PICK_HEAD") {
                Some("cherry-pick")
            } else if has("MERGE_HEAD") {
                Some("merge")
            } else if matches!(step, Step::Pop {}) {
                Some("stash")
            } else {
                None
            };
            let source = match step {
                Step::Merge { branch, .. } if operation == Some("merge") => Some(branch.clone()),
                _ => None,
            };
            let mut choices = Vec::new();
            if source.is_some() {
                choices.push(Choice::OpenMerger);
            }
            choices.push(Choice::OpenConflicts);
            choices.push(Choice::Continue);
            if matches!(operation, Some("merge" | "rebase" | "cherry-pick")) {
                choices.push(Choice::Abort);
            }
            choices.push(Choice::End);
            let stash = (operation == Some("stash"))
                .then(|| shell::ran(repo, &["rev-parse", "-q", "--verify", "refs/stash"]).ok())
                .flatten()
                .filter(shell::Ran::ok)
                .map(|ran| ran.stdout.trim().to_string());
            return Box::new(Stop {
                step: at,
                kind: StopKind::Conflict,
                title: format!(
                    "{block} stopped on {} {}",
                    paths.len(),
                    if paths.len() == 1 {
                        "conflict"
                    } else {
                        "conflicts"
                    }
                ),
                text: tail(output),
                paths,
                explained: None,
                choices,
                operation: operation.map(str::to_string),
                source,
                in_merger: false,
                stash,
            });
        }

        let rejected = matches!(step, Step::Push { .. })
            && ["[rejected]", "non-fast-forward", "fetch first", "rejected"]
                .iter()
                .any(|sign| output.contains(sign));
        let (kind, title, choices) = if rejected {
            (
                StopKind::PushRejected,
                "The remote rejected the push".to_string(),
                vec![Choice::Retry, Choice::End],
            )
        } else {
            (
                StopKind::Failed,
                match code {
                    Some(code) => format!("{block} failed: exit {code}"),
                    None => format!("{block} was ended by a signal"),
                },
                vec![Choice::Retry, Choice::Skip, Choice::End],
            )
        };
        let explained = self.explain(repo, host, at, command, code, output);
        Box::new(Stop {
            step: at,
            kind,
            title,
            text: tail(output),
            paths: Vec::new(),
            explained,
            choices,
            operation: None,
            source: None,
            in_merger: false,
            stash: None,
        })
    }

    /// The agent's words about a failure. `None` with its help off, and
    /// nothing is asked.
    fn explain(
        &mut self,
        repo: &Path,
        host: &mut dyn Host,
        at: usize,
        command: &str,
        code: Option<i32>,
        output: &str,
    ) -> Option<String> {
        if !self.recipe.agent_help || self.recipe.agent.is_none() {
            return None;
        }
        let status = shell::ran(repo, &["status", "--short"])
            .map(|ran| ran.stdout)
            .unwrap_or_default();
        let said = host.explain(
            repo,
            &Failure {
                step: at,
                what: self.steps[at].label.clone(),
                command: command.to_string(),
                code,
                output: tail(output),
                status,
            },
        )?;
        self.say(host, Some(at), LineKind::Agent, said.clone());
        Some(said)
    }

    fn execute(
        &mut self,
        repo: &Path,
        host: &mut dyn Host,
        at: usize,
        step: &Step,
    ) -> std::result::Result<(), Box<Stop>> {
        let plain = |kind, title: String, text: String, choices| {
            Box::new(Stop {
                step: at,
                kind,
                title,
                text,
                paths: Vec::new(),
                explained: None,
                choices,
                operation: None,
                source: None,
                in_merger: false,
                stash: None,
            })
        };

        match step {
            Step::Ask {} => return Ok(()),
            Step::Check { command } => {
                self.say(host, Some(at), LineKind::Command, format!("$ {command}"));
                let checked = host.check(repo, command);
                let kind = if checked.passed {
                    LineKind::Out
                } else {
                    LineKind::Error
                };
                self.say(host, Some(at), kind, tail(&checked.output));
                if checked.passed {
                    return Ok(());
                }
                let explained = self.explain(repo, host, at, command, None, &checked.output);
                let mut stop = plain(
                    StopKind::CheckFailed,
                    format!("`{command}` failed"),
                    tail(&checked.output),
                    vec![Choice::Retry, Choice::End],
                );
                stop.explained = explained;
                return Err(stop);
            }
            Step::Clean {} => {
                self.say(
                    host,
                    Some(at),
                    LineKind::Command,
                    "$ git status --porcelain".into(),
                );
                let status = shell::ran(repo, &["status", "--porcelain"]).map_err(|error| {
                    plain(
                        StopKind::Failed,
                        "git could not be run".into(),
                        error.to_string(),
                        vec![Choice::Retry, Choice::End],
                    )
                })?;
                let changed = status.stdout.trim_end();
                if changed.is_empty() {
                    return Ok(());
                }
                self.say(host, Some(at), LineKind::Out, changed.to_string());
                return Err(plain(
                    StopKind::NotClean,
                    "The working tree is not clean".into(),
                    tail(changed),
                    vec![Choice::Retry, Choice::End],
                ));
            }
            _ => {}
        }

        // A message left to the agent is written first, from the repository
        // as it is before the step changes anything.
        let mut written: Option<String> = None;
        if matches!(step.message(), Some(Message::Agent)) {
            let wanted = Wanted {
                step: at,
                what: self.steps[at].label.clone(),
                changes: self.changes_for(repo, step),
            };
            match host
                .message(repo, &wanted)
                .and_then(|said| message_from(&said))
            {
                Ok(message) => {
                    self.say(host, Some(at), LineKind::Agent, message.clone());
                    written = Some(message);
                }
                Err(why) => {
                    return Err(plain(
                        StopKind::NoMessage,
                        "The agent did not write a message".into(),
                        why,
                        vec![Choice::Retry, Choice::End],
                    ));
                }
            }
        }

        let all = commands(step, self.prune, written.as_deref());
        debug_assert!(!all.iter().flatten().any(|arg| arg == AGENT_WRITES));
        for (index, args) in all.iter().enumerate().skip(self.resume) {
            let line = shown_command(args);
            self.say(host, Some(at), LineKind::Command, format!("$ {line}"));
            let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
            let ran = match shell::ran(repo, &borrowed) {
                Ok(ran) => ran,
                Err(error) => {
                    return Err(plain(
                        StopKind::Failed,
                        "git could not be run".into(),
                        error.to_string(),
                        vec![Choice::Retry, Choice::End],
                    ));
                }
            };
            let output = ran.output();
            let kind = if ran.ok() {
                LineKind::Out
            } else {
                LineKind::Error
            };
            self.say(host, Some(at), kind, output.clone());
            if !ran.ok() {
                // *Run again* starts from the command that failed: a second
                // `reset --soft HEAD~2` would take two more commits. Staging
                // is the exception, and is done again: what the person
                // changed to mend the failure has to be in the commit.
                self.resume = index;
                while self.resume > 0 && all[self.resume - 1][0] == "add" {
                    self.resume -= 1;
                }
                return Err(self.failure(repo, host, at, step, &line, ran.code, &output));
            }
        }
        Ok(())
    }

    /// Finish what a conflict interrupted, once nothing is conflicted.
    fn conclude(
        &mut self,
        repo: &Path,
        host: &mut dyn Host,
        stop: &Stop,
    ) -> std::result::Result<(), String> {
        let at = stop.step;
        if stop.in_merger {
            let source = stop.source.as_deref().unwrap_or_default();
            return match shell::is_ancestor(repo, source, "HEAD") {
                Ok(true) => Ok(()),
                _ => Err(format!(
                    "{source} is not merged yet. Finish the merge in Merger first."
                )),
            };
        }
        let left = shell::ran(repo, &["diff", "--name-only", "--diff-filter=U"])
            .map_err(|error| error.to_string())?;
        let left = left.stdout.lines().filter(|line| !line.is_empty()).count();
        if left > 0 {
            return Err(format!(
                "{left} {} still conflicted.",
                if left == 1 { "file is" } else { "files are" }
            ));
        }
        let git_dir = shell::absolute_git_dir(repo).map_err(|error| error.to_string())?;
        let finish: Option<Vec<&str>> = match stop.operation.as_deref() {
            Some("merge") if git_dir.join("MERGE_HEAD").exists() => {
                Some(vec!["commit", "--no-edit"])
            }
            Some("rebase")
                if git_dir.join("rebase-merge").exists()
                    || git_dir.join("rebase-apply").exists() =>
            {
                Some(vec!["rebase", "--continue"])
            }
            Some("cherry-pick") if git_dir.join("CHERRY_PICK_HEAD").exists() => {
                Some(vec!["cherry-pick", "--continue"])
            }
            _ => None,
        };
        if let Some(args) = finish {
            let line = format!("git {}", args.join(" "));
            self.say(host, Some(at), LineKind::Command, format!("$ {line}"));
            let ran = shell::ran(repo, &args).map_err(|error| error.to_string())?;
            let output = ran.output();
            self.say(
                host,
                Some(at),
                if ran.ok() {
                    LineKind::Out
                } else {
                    LineKind::Error
                },
                output.clone(),
            );
            if !ran.ok() {
                return Err(tail(&output));
            }
        }
        // A pop that conflicted leaves its stash in the list. Now that its
        // changes are in the tree, it goes — if it is still the one on top.
        if let Some(stash) = &stop.stash {
            let top = shell::ran(repo, &["rev-parse", "-q", "--verify", "refs/stash"])
                .map_err(|error| error.to_string())?;
            if top.ok() && top.stdout.trim() == stash {
                self.say(host, Some(at), LineKind::Command, "$ git stash drop".into());
                let _ = shell::ran(repo, &["stash", "drop"]);
            }
        }
        Ok(())
    }

    /// The person's choice at a stop.
    ///
    /// An error means the choice cannot be taken yet — a file is still
    /// conflicted — and the run goes on waiting where it was.
    pub fn answer(
        &mut self,
        repo: &Path,
        choice: Choice,
        host: &mut dyn Host,
    ) -> std::result::Result<(), String> {
        let Some(stop) = self.stop.clone() else {
            return Err("The run is not waiting for anything.".into());
        };
        if !stop.choices.contains(&choice) {
            return Err("That is not one of the choices here.".into());
        }
        let at = stop.step;
        let record = Answered {
            step: at,
            kind: stop.kind,
            choice,
            at: now(),
        };

        match (stop.kind, choice) {
            (_, Choice::End) => {
                self.answers.push(record);
                self.say(host, None, LineKind::Note, "Ended.".into());
                self.finish(Outcome::Ended);
            }
            (StopKind::Approve | StopKind::Ask, Choice::Continue) => {
                self.answers.push(record);
                let then = self.mark.take();
                let moved = then.is_some() && then != mark(repo);
                if moved {
                    self.stop_at(Stop {
                        kind: StopKind::Changed,
                        title: "The repository changed while the run waited".into(),
                        text: "Something other than this recipe moved HEAD or changed the working tree. Look before going on."
                            .into(),
                        ..stop
                    });
                } else {
                    self.approved = true;
                    self.stop = None;
                }
            }
            (StopKind::Changed, Choice::Continue) => {
                self.answers.push(record);
                self.approved = true;
                self.stop = None;
            }
            (_, Choice::Retry) => {
                self.answers.push(record);
                self.approved = true;
                self.stop = None;
            }
            (_, Choice::Skip) => {
                self.answers.push(record);
                self.steps[at].state = StepState::Skipped;
                self.done();
            }
            (StopKind::Conflict, Choice::OpenMerger) => {
                let ran =
                    shell::ran(repo, &["merge", "--abort"]).map_err(|error| error.to_string())?;
                if !ran.ok() {
                    return Err(tail(&ran.output()));
                }
                self.answers.push(record);
                self.say(
                    host,
                    Some(at),
                    LineKind::Command,
                    "$ git merge --abort".into(),
                );
                self.say(
                    host,
                    Some(at),
                    LineKind::Note,
                    "The merge is to be made in Merger.".into(),
                );
                self.stop = Some(Stop {
                    in_merger: true,
                    choices: vec![Choice::Continue, Choice::End],
                    ..stop
                });
            }
            // The screen goes to Conflicts. Nothing changes here.
            (StopKind::Conflict, Choice::OpenConflicts) => {}
            (StopKind::Conflict, Choice::Continue) => {
                self.conclude(repo, host, &stop)?;
                self.answers.push(record);
                self.steps[at].state = StepState::Done;
                self.done();
            }
            (StopKind::Conflict, Choice::Abort) => {
                let operation = stop.operation.clone().unwrap_or_default();
                let ran = shell::ran(repo, &[operation.as_str(), "--abort"])
                    .map_err(|error| error.to_string())?;
                self.say(
                    host,
                    Some(at),
                    LineKind::Command,
                    format!("$ git {operation} --abort"),
                );
                if !ran.ok() {
                    return Err(tail(&ran.output()));
                }
                self.answers.push(record);
                self.finish(Outcome::Ended);
            }
            _ => return Err("That is not one of the choices here.".into()),
        }
        Ok(())
    }

    /// *Undo the whole run*: everything goes back to the restore point.
    /// Not while a step is running.
    pub fn undo(&mut self, repo: &Path, host: &mut dyn Host) -> std::result::Result<(), String> {
        restore::undo(repo, &self.restore).map_err(|error| error.to_string())?;
        self.say(
            host,
            None,
            LineKind::Note,
            "Undone: everything is as it was before the run. What was pushed stays pushed.".into(),
        );
        self.finish(Outcome::Undone);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

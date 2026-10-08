// SPDX-License-Identifier: GPL-3.0-or-later

//! The record of one assignment.
//!
//! It answers *who decided this line*: the agent and its version or model, the
//! level and every change to it, what it worked against, each step, each
//! proposal and who decided it, and the raw transcript. Written as it happens,
//! by temporary file and rename, so a crash leaves the last whole record and
//! never half of one. It is kept in application data on this machine and is
//! never sent anywhere.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::classify::Choice;
use super::level::{Gate, Job, Level};
use super::protocol::{Finding, Plan, Summary};

/// Seconds since the epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Whether the agent runs here or is reached over an API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reach {
    Local,
    Remote,
}

/// Which agent, as it was when the assignment started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRef {
    pub id: String,
    pub name: String,
    pub reach: Reach,
    /// The CLI's version, for a local agent.
    #[serde(default)]
    pub version: Option<String>,
    /// The provider's slug: `claudeCode`, `codex`, `anthropic`, `ollama`…
    pub provider: String,
    /// The model, for a remote agent.
    #[serde(default)]
    pub model: Option<String>,
}

/// Who did something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Who {
    Person,
    Agent,
}

/// What an assignment works against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    /// One pull request at one head.
    #[serde(rename_all = "camelCase")]
    Review {
        host: String,
        owner: String,
        name: String,
        number: u64,
        title: String,
        /// The merge base the diff is read from.
        base: String,
        head: String,
        /// The branch it would merge into, for the prompt.
        #[serde(default)]
        target: String,
    },
    /// One merge: two branches, their base, a direction and a strategy.
    #[serde(rename_all = "camelCase")]
    Merge {
        /// The receiving branch.
        a: String,
        /// The branch being merged in.
        b: String,
        a_tip: String,
        b_tip: String,
        base: String,
        #[serde(default)]
        strategy: String,
    },
}

impl Target {
    pub fn job(&self) -> Job {
        match self {
            Target::Review { .. } => Job::Review,
            Target::Merge { .. } => Job::Merge,
        }
    }
}

/// Where an assignment is in its life. The words on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Starting,
    Working,
    /// The only state that puts a dot on the rail.
    Waiting,
    Paused,
    Stopped,
    Failed,
    Done,
}

impl State {
    pub fn is_over(self) -> bool {
        matches!(self, State::Stopped | State::Failed | State::Done)
    }
}

/// One unit of the agent's work the person can see finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepKind {
    /// Read the pull request, or the merge.
    Read,
    Plan,
    File {
        path: String,
    },
    Verdict,
    #[serde(rename_all = "camelCase")]
    Conflict {
        path: String,
        region: usize,
    },
    Checks,
    /// *Ask why* on a proposal.
    #[serde(rename_all = "camelCase")]
    Why {
        proposal: String,
    },
    /// The last act: *Send*, or *Land*.
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Running,
    Done,
    /// Waiting at a gate.
    Waiting,
    Failed,
    /// Run again with a note; the old one is kept, marked.
    Superseded,
}

/// One check's result, as the timeline shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRun {
    pub command: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub index: usize,
    pub kind: StepKind,
    /// In words: *avatars.rs · 2 findings*.
    pub label: String,
    pub state: StepState,
    pub started_at: u64,
    #[serde(default)]
    pub ended_at: Option<u64>,
    /// The gate this step waits at, while it waits.
    #[serde(default)]
    pub gate: Option<Gate>,
    /// What the agent read, searched and ran, narrated. Capped.
    #[serde(default)]
    pub events: Vec<String>,
    /// Files whose content was sent to a remote agent in this step.
    #[serde(default)]
    pub sent: Vec<String>,
    /// What the agent wrote where it was not asked to — listed and not kept.
    #[serde(default)]
    pub refused: Vec<String>,
    /// The note this step was redone with, or the person's message.
    #[serde(default)]
    pub note: Option<String>,
    /// The command line that started this step's run, quoted to paste.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub checks: Vec<CheckRun>,
    /// Tokens in and out, as the provider reported them.
    #[serde(default)]
    pub tokens: Tokens,
}

/// How many events a step keeps. The transcript keeps everything.
pub const STEP_EVENTS: usize = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
}

impl Tokens {
    pub fn total(self) -> u64 {
        self.input + self.output
    }

    pub fn add(&mut self, other: Tokens) {
        self.input += other.input;
        self.output += other.output;
    }
}

/// What a proposal is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProposalBody {
    Plan(Plan),
    Comment(Finding),
    Verdict(Summary),
    /// One conflict region's resolution, already named by Spagitty.
    #[serde(rename_all = "camelCase")]
    Resolution {
        path: String,
        region: usize,
        choice: Choice,
        why: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalState {
    /// Waiting for the person.
    Proposed,
    /// Made the person's: a pending comment, a resolver choice.
    Accepted,
    /// Made the person's, in their words.
    Edited,
    /// Kept in the record only.
    Dismissed,
    /// Redone; the new one follows it.
    Superseded,
    /// Applied by the agent itself, at *Sign off* or *Unattended*.
    Applied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub step: usize,
    pub body: ProposalBody,
    pub sure: bool,
    pub state: ProposalState,
    #[serde(default)]
    pub decided_by: Option<Who>,
    /// The agent's answer to *Ask why*.
    #[serde(default)]
    pub why: Option<String>,
    /// Made against an older head, and carried on regardless.
    #[serde(default)]
    pub stale: bool,
}

impl Proposal {
    pub fn is_open(&self) -> bool {
        self.state == ProposalState::Proposed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelChange {
    pub from: Level,
    pub to: Level,
    pub by: Who,
    pub at: u64,
}

/// What the agent asks the webview to do at the last act, at *Unattended*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LastAct {
    /// Send the review: the verdict (already limited by the repository's
    /// rules) and the words for the whole pull request.
    Send {
        verdict: super::protocol::Verdict,
        body: String,
    },
    /// Land the merge with the choices made.
    Land,
}

/// One assignment, whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: String,
    /// The repository it belongs to, as a path on this machine.
    pub repo: String,
    pub job: Job,
    pub agent: AgentRef,
    pub level: Level,
    #[serde(default)]
    pub levels: Vec<LevelChange>,
    pub target: Target,
    /// What the person handed over for a merge: resolving only, or
    /// resolving, checking and landing.
    #[serde(default)]
    pub lands: bool,
    pub state: State,
    /// The one sentence, always true.
    pub sentence: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub proposals: Vec<Proposal>,
    /// Steps planned, for the meter. Grows when a plan is read.
    #[serde(default)]
    pub planned: usize,
    #[serde(default)]
    pub tokens: Tokens,
    pub started_at: u64,
    #[serde(default)]
    pub ended_at: Option<u64>,
    /// The agent's or the provider's own sentence, when it stopped or failed.
    #[serde(default)]
    pub reason: Option<String>,
    /// *You took over at file 5 of 12*.
    #[serde(default)]
    pub took_over: Option<String>,
    /// The last act the agent asks for, at *Unattended*, until it is done.
    #[serde(default)]
    pub last_act: Option<LastAct>,
    /// Set when a pause was asked for and the current step has not ended.
    #[serde(default)]
    pub pausing: bool,
    /// Seconds since the last thing the agent said.
    #[serde(default)]
    pub quiet_since: Option<u64>,
}

impl Assignment {
    pub fn waiting(&self) -> bool {
        self.state == State::Waiting
    }

    pub fn open_proposals(&self) -> impl Iterator<Item = &Proposal> {
        self.proposals.iter().filter(|proposal| proposal.is_open())
    }

    pub fn proposal_mut(&mut self, id: &str) -> Option<&mut Proposal> {
        self.proposals.iter_mut().find(|proposal| proposal.id == id)
    }

    pub fn steps_done(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.state == StepState::Done)
            .count()
    }
}

/// Where assignments are kept: `<root>/<repo key>/<id>/`.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Store {
        Store { root: root.into() }
    }

    /// One directory per repository, named by a hash of its path so a path
    /// with any characters in it is a safe directory name.
    pub fn repo_dir(&self, repo: &str) -> PathBuf {
        self.root.join(format!("{:016x}", fnv(repo.as_bytes())))
    }

    pub fn dir(&self, repo: &str, id: &str) -> PathBuf {
        self.repo_dir(repo).join(id)
    }

    /// Write the record: temporary file, then rename.
    pub fn save(&self, assignment: &Assignment) -> std::io::Result<()> {
        let dir = self.dir(&assignment.repo, &assignment.id);
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_vec_pretty(assignment).map_err(std::io::Error::other)?;
        let partial = dir.join("record.json.partial");
        std::fs::write(&partial, json)?;
        std::fs::rename(partial, dir.join("record.json"))
    }

    pub fn load(&self, repo: &str, id: &str) -> Option<Assignment> {
        read(&self.dir(repo, id).join("record.json"))
    }

    /// Every assignment kept for a repository, newest first.
    pub fn list(&self, repo: &str) -> Vec<Assignment> {
        let Ok(entries) = std::fs::read_dir(self.repo_dir(repo)) else {
            return Vec::new();
        };
        let mut all: Vec<Assignment> = entries
            .flatten()
            .filter_map(|entry| read(&entry.path().join("record.json")))
            .collect();
        all.sort_by(|a, b| b.started_at.cmp(&a.started_at).then(b.id.cmp(&a.id)));
        all
    }

    /// Append to the raw transcript, one line at a time.
    pub fn transcript(&self, assignment: &Assignment, line: &str) {
        use std::io::Write;
        let dir = self.dir(&assignment.repo, &assignment.id);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("transcript.log"))
        {
            let _ = writeln!(file, "{line}");
        }
    }

    pub fn read_transcript(&self, repo: &str, id: &str) -> String {
        std::fs::read_to_string(self.dir(repo, id).join("transcript.log")).unwrap_or_default()
    }
}

fn read(path: &Path) -> Option<Assignment> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// FNV-1a, 64 bits: stable across runs and platforms, unlike `DefaultHasher`.
pub fn fnv(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn sample(repo: &str, id: &str) -> Assignment {
        Assignment {
            id: id.into(),
            repo: repo.into(),
            job: Job::Review,
            agent: AgentRef {
                id: "claude".into(),
                name: "Claude Code".into(),
                reach: Reach::Local,
                version: Some("2.1.259".into()),
                provider: "claudeCode".into(),
                model: None,
            },
            level: Level::StepByStep,
            levels: Vec::new(),
            target: Target::Review {
                host: "github.com".into(),
                owner: "team".into(),
                name: "app".into(),
                number: 214,
                title: "Cache avatars".into(),
                base: "aaa".into(),
                head: "bbb".into(),
                target: "main".into(),
            },
            lands: false,
            state: State::Working,
            sentence: "Starting".into(),
            note: String::new(),
            steps: Vec::new(),
            proposals: Vec::new(),
            planned: 0,
            tokens: Tokens::default(),
            started_at: 10,
            ended_at: None,
            reason: None,
            took_over: None,
            last_act: None,
            pausing: false,
            quiet_since: None,
        }
    }

    #[test]
    fn a_record_reads_back_as_it_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let assignment = sample("/work/app", "a1");
        store.save(&assignment).unwrap();
        assert_eq!(store.load("/work/app", "a1"), Some(assignment));
        assert!(!store
            .dir("/work/app", "a1")
            .join("record.json.partial")
            .exists());
    }

    #[test]
    fn a_repository_lists_its_assignments_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut old = sample("/work/app", "old");
        old.started_at = 1;
        store.save(&old).unwrap();
        store.save(&sample("/work/app", "new")).unwrap();
        store.save(&sample("/work/other", "elsewhere")).unwrap();
        let ids: Vec<String> = store.list("/work/app").into_iter().map(|a| a.id).collect();
        assert_eq!(ids, ["new", "old"]);
    }

    #[test]
    fn the_transcript_is_appended_line_by_line() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let assignment = sample("/work/app", "a1");
        store.transcript(&assignment, "one");
        store.transcript(&assignment, "two");
        assert_eq!(store.read_transcript("/work/app", "a1"), "one\ntwo\n");
    }

    #[test]
    fn a_record_that_does_not_parse_is_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let broken = store.dir("/work/app", "broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("record.json"), "{").unwrap();
        assert!(store.list("/work/app").is_empty());
    }
}

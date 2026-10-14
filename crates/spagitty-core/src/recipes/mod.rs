// SPDX-License-Identifier: GPL-3.0-or-later

//! Recipes: git steps in blocks (FEAT-133).
//!
//! A recipe is an ordered stack of blocks from a closed catalogue — stash,
//! pull, merge, pop, commit, push — each with a few parameters chosen from
//! menus. Spagitty runs every block's command itself, a step at a time, after
//! saving a restore point that puts everything back.
//!
//! # The catalogue is closed, and that is the safety
//!
//! [`Step`] is an enum. There is no variant for a force push, for
//! `branch -D`, or for `reset --hard`, so no recipe — built by hand, proposed
//! by an agent, or imported from a file — can hold one. An agent that answers
//! with a block that is not here has not answered with a step. Adding a block
//! is a change to Spagitty, not to a recipe.
//!
//! # The agent's help is a switch
//!
//! The author's note of 2026-10-13: with [`Recipe::agent_help`] on, the person
//! writes a sentence and an agent proposes the blocks ([`propose`]), and during
//! a run it writes a message set to *agent writes* and explains a failure.
//! With it off no agent is called at any point, and the run is the blocks' git
//! commands and nothing else. Either way the commands are run by Spagitty
//! ([`run`]), never by the agent.
//!
//! # What is where
//!
//! - [`command`]: a block to the git arguments it runs.
//! - [`validate`]: the one function that says whether a stack can run.
//! - [`brief`]: what an agent is told, `recipes.md` and then this recipe.
//! - [`propose`]: asking an agent for a stack, and reading its answer.
//! - [`script`]: the steps as a shell script.
//! - [`restore`]: the restore point, and undoing a whole run.
//! - [`run`]: the run itself, a step at a time.
//! - [`world`]: what is true of the repository, for the validator.

use serde::{Deserialize, Serialize};

use crate::shell::PullMode;

pub mod brief;
pub mod command;
pub mod propose;
pub mod restore;
pub mod run;
pub mod script;
pub mod validate;
pub mod world;

/// The version of the JSON form a recipe is saved, exported and imported in.
pub const VERSION: u32 = 1;
/// A stack longer than this cannot run.
pub const MAX_STEPS: usize = 30;
/// A commit message's one line.
pub const MAX_MESSAGE: usize = 72;
/// The note to the agent.
pub const MAX_NOTE: usize = 140;
/// The sentence an agent builds the blocks from.
pub const MAX_SENTENCE: usize = 280;
/// A recipe's name.
pub const MAX_NAME: usize = 40;

/// How far a run goes without the person. The four levels of 2.0, with the
/// values they are stored under there.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// Nothing runs. The commands are shown, and the script is the way out.
    Suggest,
    /// Stops before every step.
    #[default]
    StepByStep,
    /// Runs, and stops before Push and Delete branch.
    SignOff,
    /// Runs through.
    Unattended,
}

impl Level {
    /// As it is shown, in the words 2.0's levels have had since TASK-060.
    pub fn label(self) -> &'static str {
        match self {
            Level::Suggest => "Read-only",
            Level::StepByStep => "Ask every step",
            Level::SignOff => "Ask before pushing",
            Level::Unattended => "Full access",
        }
    }

    /// As it is written in a brief's header.
    pub fn word(self) -> &'static str {
        match self {
            Level::Suggest => "suggest",
            Level::StepByStep => "step",
            Level::SignOff => "sign-off",
            Level::Unattended => "unattended",
        }
    }
}

/// How a merge is made.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeHow {
    /// Always a merge commit: `--no-ff`.
    #[default]
    Commit,
    /// Fast-forward when that is possible, a merge commit when it is not.
    FastForward,
    /// The other branch's changes, staged as one: `--squash`. Nothing is
    /// committed until a Commit block does.
    Squash,
}

/// A commit's message: typed, or left to the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Message {
    Text {
        text: String,
    },
    /// *Agent writes*: one line, written from the changes when the step runs.
    /// Only with the agent's help switched on.
    Agent,
}

impl Message {
    pub fn text(text: impl Into<String>) -> Message {
        Message::Text { text: text.into() }
    }
}

/// One block of a recipe, with its parameters.
///
/// The JSON form is `{ "block": "stash", "params": { "untracked": true } }`.
/// A block with no parameters still has `"params": {}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "block", content = "params", rename_all = "camelCase")]
pub enum Step {
    Stash {
        untracked: bool,
    },
    /// The newest stash a Stash block above made.
    Pop {},
    Commit {
        /// Everything, staged first with `git add -A`; or only what is staged.
        all: bool,
        message: Message,
    },
    Amend {
        /// `None` keeps the message the commit has.
        message: Option<Message>,
    },
    Fetch {
        /// Empty is every remote.
        remote: String,
    },
    Pull {
        /// Empty is the branch's own upstream.
        remote: String,
        mode: PullMode,
    },
    /// Never forced: there is no parameter for it.
    Push {
        remote: String,
    },
    Merge {
        branch: String,
        how: MergeHow,
    },
    Rebase {
        onto: String,
    },
    CherryPick {
        commits: Vec<String>,
    },
    Switch {
        branch: String,
    },
    NewBranch {
        name: String,
        /// Empty is from here.
        from: String,
    },
    /// Only when merged: `-d`, never `-D`.
    DeleteBranch {
        branch: String,
    },
    Squash {
        /// The last this-many commits become one.
        count: u32,
        message: Message,
    },
    /// The last commit's message, and nothing else of it.
    Reword {
        message: Message,
    },
    /// One of the repository's configured checks.
    Check {
        command: String,
    },
    /// *Stop and ask me*.
    Ask {},
    /// *Stop unless clean*.
    Clean {},
}

/// The six groups of the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Save,
    Sync,
    Combine,
    Branch,
    Shape,
    Guard,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Save => "Save",
            Group::Sync => "Sync",
            Group::Combine => "Combine",
            Group::Branch => "Branch",
            Group::Shape => "Shape history",
            Group::Guard => "Guard",
        }
    }
}

/// A block as the catalogue lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// As it is written in a recipe's JSON.
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    /// The one key that adds it.
    pub key: char,
    /// The command it is, with nothing chosen yet.
    pub command: &'static str,
}

const fn block(
    id: &'static str,
    label: &'static str,
    group: Group,
    key: char,
    command: &'static str,
) -> Block {
    Block {
        id,
        label,
        group,
        key,
        command,
    }
}

/// Every block there is, in the order the screen lists them. The spec's table
/// (§8.2).
pub const CATALOGUE: [Block; 18] = [
    block("stash", "Stash", Group::Save, 'S', "git stash push"),
    block("pop", "Pop stash", Group::Save, 'P', "git stash pop"),
    block("commit", "Commit", Group::Save, 'C', "git commit -m"),
    block("amend", "Amend", Group::Save, 'A', "git commit --amend"),
    block("fetch", "Fetch", Group::Sync, 'F', "git fetch"),
    block("pull", "Pull", Group::Sync, 'L', "git pull"),
    block("push", "Push", Group::Sync, 'U', "git push"),
    block("merge", "Merge", Group::Combine, 'M', "git merge"),
    block("rebase", "Rebase", Group::Combine, 'R', "git rebase"),
    block(
        "cherryPick",
        "Cherry-pick",
        Group::Combine,
        'Y',
        "git cherry-pick",
    ),
    block("switch", "Switch", Group::Branch, 'W', "git switch"),
    block(
        "newBranch",
        "New branch",
        Group::Branch,
        'N',
        "git switch -c",
    ),
    block(
        "deleteBranch",
        "Delete branch",
        Group::Branch,
        'D',
        "git branch -d",
    ),
    block(
        "squash",
        "Squash",
        Group::Shape,
        'Q',
        "git reset --soft HEAD~N",
    ),
    block(
        "reword",
        "Reword",
        Group::Shape,
        'E',
        "git commit --amend --only -m",
    ),
    block("check", "Run checks", Group::Guard, 'T', "as configured"),
    block("ask", "Stop and ask me", Group::Guard, '?', ""),
    block(
        "clean",
        "Stop unless clean",
        Group::Guard,
        'G',
        "git status --porcelain",
    ),
];

impl Step {
    /// The block's name in a recipe's JSON.
    pub fn id(&self) -> &'static str {
        match self {
            Step::Stash { .. } => "stash",
            Step::Pop {} => "pop",
            Step::Commit { .. } => "commit",
            Step::Amend { .. } => "amend",
            Step::Fetch { .. } => "fetch",
            Step::Pull { .. } => "pull",
            Step::Push { .. } => "push",
            Step::Merge { .. } => "merge",
            Step::Rebase { .. } => "rebase",
            Step::CherryPick { .. } => "cherryPick",
            Step::Switch { .. } => "switch",
            Step::NewBranch { .. } => "newBranch",
            Step::DeleteBranch { .. } => "deleteBranch",
            Step::Squash { .. } => "squash",
            Step::Reword { .. } => "reword",
            Step::Check { .. } => "check",
            Step::Ask {} => "ask",
            Step::Clean {} => "clean",
        }
    }

    pub fn block(&self) -> &'static Block {
        let id = self.id();
        CATALOGUE
            .iter()
            .find(|block| block.id == id)
            .expect("every step is in the catalogue")
    }

    /// The message this step is committed with, when it has one.
    pub fn message(&self) -> Option<&Message> {
        match self {
            Step::Commit { message, .. }
            | Step::Squash { message, .. }
            | Step::Reword { message } => Some(message),
            Step::Amend { message } => message.as_ref(),
            _ => None,
        }
    }

    /// True when the agent writes this step's message.
    pub fn agent_writes(&self) -> bool {
        matches!(self.message(), Some(Message::Agent))
    }

    /// The branches this step names, other than one it makes.
    pub fn uses(&self) -> Vec<&str> {
        match self {
            Step::Merge { branch, .. }
            | Step::Switch { branch }
            | Step::DeleteBranch { branch } => {
                vec![branch.as_str()]
            }
            Step::Rebase { onto } => vec![onto.as_str()],
            Step::NewBranch { from, .. } if !from.is_empty() => vec![from.as_str()],
            _ => Vec::new(),
        }
    }

    /// One of each block, with parameters filled in: what an agent is shown
    /// as the whole vocabulary, and what a new block starts as.
    pub fn examples() -> Vec<Step> {
        vec![
            Step::Stash { untracked: true },
            Step::Pop {},
            Step::Commit {
                all: true,
                message: Message::text("Sync with main"),
            },
            Step::Amend { message: None },
            Step::Fetch {
                remote: String::new(),
            },
            Step::Pull {
                remote: "origin".into(),
                mode: PullMode::FastForwardOnly,
            },
            Step::Push {
                remote: "origin".into(),
            },
            Step::Merge {
                branch: "main".into(),
                how: MergeHow::Commit,
            },
            Step::Rebase {
                onto: "main".into(),
            },
            Step::CherryPick {
                commits: vec!["a3f9c1e".into()],
            },
            Step::Switch {
                branch: "main".into(),
            },
            Step::NewBranch {
                name: "feature/name".into(),
                from: String::new(),
            },
            Step::DeleteBranch {
                branch: "feature/name".into(),
            },
            Step::Squash {
                count: 3,
                message: Message::text("One commit for the change"),
            },
            Step::Reword {
                message: Message::text("Say what the commit does"),
            },
            Step::Check {
                command: "cargo test".into(),
            },
            Step::Ask {},
            Step::Clean {},
        ]
    }
}

/// A recipe, in the form it is saved, exported and imported in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    /// [`VERSION`].
    pub v: u32,
    pub name: String,
    /// The branch it runs on. Empty is whichever is checked out.
    #[serde(default)]
    pub branch: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub level: Level,
    /// The agent's help. Off, no agent is called at any point.
    #[serde(default)]
    pub agent_help: bool,
    /// The agent that helps, by its id in Settings › Agents.
    #[serde(default)]
    pub agent: Option<String>,
    /// One note to the agent. Not read with the help off.
    #[serde(default)]
    pub note: String,
}

impl Recipe {
    /// An empty recipe called `name`.
    pub fn named(name: impl Into<String>) -> Recipe {
        Recipe {
            v: VERSION,
            name: name.into(),
            branch: String::new(),
            steps: Vec::new(),
            level: Level::default(),
            agent_help: false,
            agent: None,
            note: String::new(),
        }
    }

    /// The step a Pop at `index` pops: the newest Stash above it that no Pop
    /// between them took.
    pub fn popped_by(&self, index: usize) -> Option<usize> {
        let mut open: Vec<usize> = Vec::new();
        for (at, step) in self.steps.iter().enumerate().take(index) {
            match step {
                Step::Stash { .. } => open.push(at),
                Step::Pop {} => {
                    open.pop();
                }
                _ => {}
            }
        }
        open.pop()
    }

    /// The branch checked out when the step at `index` runs, as far as the
    /// stack itself says: `start`, until a Switch or a New branch above.
    pub fn branch_at<'a>(&'a self, index: usize, start: &'a str) -> &'a str {
        let mut current = start;
        for step in self.steps.iter().take(index) {
            match step {
                Step::Switch { branch } => current = branch,
                Step::NewBranch { name, .. } => current = name,
                _ => {}
            }
        }
        current
    }

    /// The step at `index` in a sentence, as a brief lists it:
    /// `Merge main into feature/avatars — always a merge commit`.
    pub fn describe(&self, index: usize, start: &str) -> String {
        let here = match self.branch_at(index, start) {
            "" => "this branch",
            name => name,
        };
        let said = |message: &Message| match message {
            Message::Text { text } => format!("message: \"{text}\""),
            Message::Agent => "you write the message".to_string(),
        };
        match &self.steps[index] {
            Step::Stash { untracked: true } => "Stash — include untracked".into(),
            Step::Stash { untracked: false } => "Stash".into(),
            Step::Pop {} => match self.popped_by(index) {
                Some(from) => format!("Pop the stash from step {} — on conflict, stop", from + 1),
                None => "Pop the stash — on conflict, stop".into(),
            },
            Step::Commit { all, message } => format!(
                "Commit {} — {}",
                if *all {
                    "all changes"
                } else {
                    "what is staged"
                },
                said(message)
            ),
            Step::Amend { message: None } => "Amend the last commit — keep its message".into(),
            Step::Amend {
                message: Some(message),
            } => format!("Amend the last commit — {}", said(message)),
            Step::Fetch { remote } if remote.is_empty() => "Fetch every remote".into(),
            Step::Fetch { remote } => format!("Fetch {remote}"),
            Step::Pull { remote, mode } => format!(
                "Pull from {} — {}",
                if remote.is_empty() {
                    "the upstream"
                } else {
                    remote
                },
                match mode {
                    PullMode::FastForwardOnly => "fast-forward only",
                    PullMode::Merge => "merge",
                    PullMode::Rebase => "rebase",
                }
            ),
            Step::Push { remote } => format!("Push to {remote} — never force"),
            Step::Merge { branch, how } => format!(
                "Merge {branch} into {here} — {}",
                match how {
                    MergeHow::Commit => "always a merge commit",
                    MergeHow::FastForward => "fast-forward if possible",
                    MergeHow::Squash => "squash, without committing",
                }
            ),
            Step::Rebase { onto } => format!("Rebase {here} onto {onto}"),
            Step::CherryPick { commits } => format!("Cherry-pick {}", commits.join(", ")),
            Step::Switch { branch } => format!("Switch to {branch}"),
            Step::NewBranch { name, from } if from.is_empty() => {
                format!("New branch {name} — from here")
            }
            Step::NewBranch { name, from } => format!("New branch {name} — from {from}"),
            Step::DeleteBranch { branch } => format!("Delete branch {branch} — only if merged"),
            Step::Squash { count, message } => {
                format!("Squash the last {count} commits — {}", said(message))
            }
            Step::Reword { message } => format!("Reword the last commit — {}", said(message)),
            Step::Check { command } => format!("Run the check `{command}` — on failure, stop"),
            Step::Ask {} => "Stop and ask me".into(),
            Step::Clean {} => "Stop unless the working tree is clean".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_the_specs_eighteen_blocks_in_six_groups() {
        assert_eq!(CATALOGUE.len(), 18);
        let count = |group: Group| CATALOGUE.iter().filter(|b| b.group == group).count();
        assert_eq!(
            [
                count(Group::Save),
                count(Group::Sync),
                count(Group::Combine),
                count(Group::Branch),
                count(Group::Shape),
                count(Group::Guard)
            ],
            [4, 3, 3, 3, 2, 3]
        );
    }

    #[test]
    fn every_block_has_a_key_of_its_own_and_an_example() {
        let mut keys: Vec<char> = CATALOGUE.iter().map(|block| block.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), CATALOGUE.len());

        let examples = Step::examples();
        let ids: Vec<&str> = examples.iter().map(Step::id).collect();
        let listed: Vec<&str> = CATALOGUE.iter().map(|block| block.id).collect();
        assert_eq!(ids, listed);
        for step in &examples {
            assert_eq!(step.block().id, step.id());
        }
    }

    /// The rule that makes a recipe safe to run: what is not in the catalogue
    /// cannot be written down.
    #[test]
    fn no_block_forces_a_push_deletes_unmerged_or_resets_hard() {
        for block in CATALOGUE {
            let command = block.command;
            assert!(!command.contains("--force"), "{command}");
            assert!(!command.contains("-D"), "{command}");
            assert!(!command.contains("--hard"), "{command}");
            assert!(!command.contains("clean"), "{command}");
        }
        for unknown in [
            r#"{"block":"forcePush","params":{"remote":"origin"}}"#,
            r#"{"block":"reset","params":{"mode":"hard"}}"#,
            r#"{"block":"shell","params":{"command":"rm -rf ."}}"#,
        ] {
            assert!(serde_json::from_str::<Step>(unknown).is_err(), "{unknown}");
        }
        // And a block that is there takes no parameter it does not have.
        let push: Step =
            serde_json::from_str(r#"{"block":"push","params":{"remote":"origin","force":true}}"#)
                .expect("an unknown parameter is not read");
        assert_eq!(
            push,
            Step::Push {
                remote: "origin".into()
            }
        );
    }

    #[test]
    fn a_recipe_is_written_in_the_specs_json_form_and_read_back() {
        let recipe = Recipe {
            v: VERSION,
            name: "sync-and-ship".into(),
            branch: "feature/avatars".into(),
            steps: Step::examples(),
            level: Level::StepByStep,
            agent_help: true,
            agent: Some("claude".into()),
            note: "Keep my merge commit message as git suggests it.".into(),
        };

        let text = serde_json::to_string(&recipe).expect("written");
        let value: serde_json::Value = serde_json::from_str(&text).expect("json");

        assert_eq!(value["v"], 1);
        assert_eq!(value["level"], "stepByStep");
        assert_eq!(value["agentHelp"], true);
        assert_eq!(
            value["steps"][0],
            serde_json::json!({"block": "stash", "params": {"untracked": true}})
        );
        assert_eq!(
            value["steps"][1],
            serde_json::json!({"block": "pop", "params": {}})
        );
        assert_eq!(
            value["steps"][2]["params"]["message"],
            serde_json::json!({"kind": "text", "text": "Sync with main"})
        );
        assert_eq!(value["steps"][5]["params"]["mode"], "fastForwardOnly");
        assert_eq!(value["steps"][7]["params"]["how"], "commit");
        assert_eq!(serde_json::from_str::<Recipe>(&text).expect("read"), recipe);
    }

    #[test]
    fn a_recipe_from_before_the_switch_has_it_off() {
        let old: Recipe = serde_json::from_str(r#"{"v":1,"name":"old","steps":[]}"#).expect("read");

        assert!(!old.agent_help);
        assert_eq!(old.level, Level::StepByStep);
        assert_eq!(old.agent, None);
    }

    fn the_spec() -> Recipe {
        Recipe {
            steps: vec![
                Step::Stash { untracked: true },
                Step::Pull {
                    remote: "origin".into(),
                    mode: PullMode::FastForwardOnly,
                },
                Step::Merge {
                    branch: "main".into(),
                    how: MergeHow::Commit,
                },
                Step::Pop {},
                Step::Commit {
                    all: true,
                    message: Message::text("Sync with main"),
                },
                Step::Push {
                    remote: "origin".into(),
                },
            ],
            ..Recipe::named("sync-and-ship")
        }
    }

    #[test]
    fn steps_are_described_as_the_brief_lists_them() {
        let recipe = the_spec();
        let lines: Vec<String> = (0..recipe.steps.len())
            .map(|index| recipe.describe(index, "feature/avatars"))
            .collect();

        assert_eq!(
            lines,
            [
                "Stash — include untracked",
                "Pull from origin — fast-forward only",
                "Merge main into feature/avatars — always a merge commit",
                "Pop the stash from step 1 — on conflict, stop",
                "Commit all changes — message: \"Sync with main\"",
                "Push to origin — never force",
            ]
        );
    }

    #[test]
    fn a_pop_takes_the_newest_stash_no_pop_before_it_took() {
        let recipe = Recipe {
            steps: vec![
                Step::Stash { untracked: false },
                Step::Stash { untracked: true },
                Step::Pop {},
                Step::Pop {},
                Step::Pop {},
            ],
            ..Recipe::named("stashes")
        };

        assert_eq!(recipe.popped_by(2), Some(1));
        assert_eq!(recipe.popped_by(3), Some(0));
        assert_eq!(recipe.popped_by(4), None);
    }

    #[test]
    fn the_branch_a_step_runs_on_follows_the_switches_above_it() {
        let recipe = Recipe {
            steps: vec![
                Step::NewBranch {
                    name: "topic".into(),
                    from: String::new(),
                },
                Step::Switch {
                    branch: "main".into(),
                },
                Step::Merge {
                    branch: "topic".into(),
                    how: MergeHow::FastForward,
                },
            ],
            ..Recipe::named("land")
        };

        assert_eq!(recipe.branch_at(0, "dev"), "dev");
        assert_eq!(recipe.branch_at(1, "dev"), "topic");
        assert_eq!(
            recipe.describe(2, "dev"),
            "Merge topic into main — fast-forward if possible"
        );
        // With nothing checked out to name, it is "this branch".
        assert_eq!(
            the_spec().describe(2, ""),
            "Merge main into this branch — always a merge commit"
        );
    }

    /// The screen lists its blocks from `src/lib/recipes/catalogue.ts`. That
    /// list is this one, or the screen offers a block that cannot run, or
    /// hides one that can.
    #[test]
    fn the_screens_catalogue_is_this_one() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/lib/recipes/catalogue.ts");
        let text = std::fs::read_to_string(path).expect("the screen's catalogue");
        let field = |line: &str, name: &str| -> Option<String> {
            let from = line.find(&format!("{name}: '"))? + name.len() + 3;
            let length = line[from..].find('\'')?;
            Some(line[from..from + length].to_string())
        };
        let on_screen: Vec<(String, String, String, String)> = text
            .lines()
            .filter(|line| line.trim_start().starts_with("{ id: '") && line.contains("key: '"))
            .filter_map(|line| {
                Some((
                    field(line, "id")?,
                    field(line, "label")?,
                    field(line, "group")?,
                    field(line, "key")?,
                ))
            })
            .collect();

        let here: Vec<(String, String, String, String)> = CATALOGUE
            .iter()
            .map(|block| {
                (
                    block.id.to_string(),
                    block.label.to_string(),
                    serde_json::to_string(&block.group)
                        .expect("a group")
                        .trim_matches('"')
                        .to_string(),
                    block.key.to_string(),
                )
            })
            .collect();
        assert_eq!(on_screen, here);
    }

    #[test]
    fn the_levels_are_shown_in_the_words_two_point_oh_uses() {
        assert_eq!(Level::Suggest.label(), "Read-only");
        assert_eq!(Level::StepByStep.label(), "Ask every step");
        assert_eq!(Level::SignOff.label(), "Ask before pushing");
        assert_eq!(Level::Unattended.label(), "Full access");
        assert!(Level::Unattended > Level::SignOff);
    }
}

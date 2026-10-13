// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether a stack can run (FEAT-133).
//!
//! One function, [`validate`], run in two places: on every change to the
//! stack, for the screen, and again at *Run*, against the repository as it is
//! then. It is a pure function of the recipe and a [`World`], which is what is
//! true of the repository, so every rule is tested without one.
//!
//! It walks the steps keeping what they would leave behind — the branch
//! checked out, the branches made, the stashes pushed, whether commits the
//! upstream has were rewritten — because most of the rules are about a step
//! and the steps above it.
//!
//! An **error** means the stack cannot run. A **warning** means it can, and
//! something will be left that the person may not have meant.

use serde::{Deserialize, Serialize};

use super::{Level, MergeHow, Message, Recipe, Step, MAX_MESSAGE, MAX_NAME, MAX_NOTE, MAX_STEPS};

/// What is true of the repository a recipe is checked against.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct World {
    /// The local branches.
    pub branches: Vec<String>,
    pub remotes: Vec<String>,
    /// The branch checked out. `None` when `HEAD` is detached.
    pub current: Option<String>,
    /// The working tree or the index has changes.
    pub dirty: bool,
    /// What the repository is in the middle of: `merge`, `rebase`. `None`
    /// when nothing.
    pub operation: Option<String>,
    /// The branch checked out has an upstream.
    pub upstream: bool,
    /// How many of its commits the upstream lacks.
    pub unpushed: u32,
    /// The branches a rebase onto which would replay commits the upstream
    /// already has.
    pub rewrites_pushed: Vec<String>,
    /// The branches on the repository's *never land unattended* list.
    pub protected: Vec<String>,
    /// The repository's configured checks.
    pub checks: Vec<String>,
    /// The agents that can be used here, by id.
    pub agents: Vec<String>,
    /// The highest level this repository allows.
    pub highest: Level,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

/// One thing the validator found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// The step it is about, counted from nothing. `None` is the recipe.
    pub step: Option<usize>,
    pub severity: Severity,
    /// The rule, for a test and for nothing a person reads.
    pub code: &'static str,
    pub text: String,
}

/// The findings that stop a run.
pub fn errors(findings: &[Finding]) -> usize {
    findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count()
}

/// Whether `name` could be a branch, by the rules of `git check-ref-format`.
///
/// git is asked again when the branch is made. This is the same answer
/// without a process, for a field that is checked with every letter.
pub fn branch_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name != "@"
        && !name.starts_with('-')
        && !name.starts_with('/')
        && !name.ends_with('/')
        && !name.ends_with('.')
        && !name.contains("..")
        && !name.contains("//")
        && !name.contains("@{")
        && !name
            .chars()
            .any(|c| c.is_control() || " ~^:?*[\\".contains(c))
        && !name
            .split('/')
            .any(|part| part.starts_with('.') || part.ends_with(".lock"))
}

/// Whether `text` could be given to git as a revision: one word that is not
/// an option.
fn revision_ok(text: &str) -> bool {
    !text.is_empty() && !text.starts_with('-') && !text.chars().any(char::is_whitespace)
}

/// More commits than any branch has ahead: "nothing here is pushed".
const UNKNOWN: i64 = i64::MAX / 2;

struct Walk<'a> {
    recipe: &'a Recipe,
    world: &'a World,
    found: Vec<Finding>,
    /// The branch checked out when the step runs.
    current: Option<String>,
    made: Vec<String>,
    /// The Stash steps no Pop has taken yet.
    stashes: Vec<usize>,
    /// Changes in the tree that nothing above has put away.
    dirty: bool,
    upstream: bool,
    unpushed: i64,
    /// Commits the upstream has were rewritten: a push would need force.
    rewrote: Option<usize>,
}

impl Walk<'_> {
    fn error(&mut self, step: Option<usize>, code: &'static str, text: impl Into<String>) {
        self.found.push(Finding {
            step,
            severity: Severity::Error,
            code,
            text: text.into(),
        });
    }

    fn warning(&mut self, step: Option<usize>, code: &'static str, text: impl Into<String>) {
        self.found.push(Finding {
            step,
            severity: Severity::Warning,
            code,
            text: text.into(),
        });
    }

    fn exists(&self, branch: &str) -> bool {
        self.world.branches.iter().any(|name| name == branch)
            || self.made.iter().any(|name| name == branch)
    }

    /// A branch parameter: chosen, and there when the step runs.
    fn branch(&mut self, at: usize, branch: &str) -> bool {
        if branch.is_empty() {
            self.error(Some(at), "branch.none", "Choose a branch.");
            return false;
        }
        if !self.exists(branch) {
            self.error(
                Some(at),
                "branch.missing",
                format!("There is no branch {branch}, and no step above makes it."),
            );
            return false;
        }
        true
    }

    fn remote(&mut self, at: usize, remote: &str, needed: bool) {
        if remote.is_empty() {
            if needed {
                self.error(Some(at), "remote.none", "Choose a remote.");
            }
        } else if !self.world.remotes.iter().any(|name| name == remote) {
            self.error(
                Some(at),
                "remote.missing",
                format!("There is no remote {remote}."),
            );
        }
    }

    fn message(&mut self, at: usize, message: &Message) {
        match message {
            Message::Text { text } if text.trim().is_empty() => {
                self.error(Some(at), "message.empty", "Write a message.");
            }
            Message::Text { text } if text.chars().count() > MAX_MESSAGE => {
                self.error(
                    Some(at),
                    "message.long",
                    format!(
                        "The message is {} characters. The most is {MAX_MESSAGE}.",
                        text.chars().count()
                    ),
                );
            }
            Message::Text { text } if text.contains('\n') => {
                self.error(Some(at), "message.lines", "The message is one line.");
            }
            Message::Text { .. } => {}
            Message::Agent if !self.recipe.agent_help => {
                self.error(
                    Some(at),
                    "message.agentOff",
                    "Agent help is off. Write the message, or switch it on.",
                );
            }
            Message::Agent => {}
        }
    }

    /// Pull, Merge, Rebase and Switch over changes nothing above put away.
    fn over_changes(&mut self, at: usize, what: &str) {
        if self.dirty {
            self.warning(
                Some(at),
                "dirty",
                format!(
                    "{what} with changes in the working tree. Nothing above stashes or commits them."
                ),
            );
        }
    }

    fn step(&mut self, at: usize, step: &Step) {
        match step {
            Step::Stash { .. } => {
                self.stashes.push(at);
                self.dirty = false;
            }
            Step::Pop {} => {
                if self.stashes.pop().is_none() {
                    self.error(
                        Some(at),
                        "pop.noStash",
                        "There is no Stash above this to pop.",
                    );
                }
                self.dirty = true;
            }
            Step::Commit { message, .. } => {
                self.message(at, message);
                self.dirty = false;
                self.unpushed = self.unpushed.saturating_add(1);
            }
            Step::Amend { message } => {
                if let Some(message) = message {
                    self.message(at, message);
                }
                if self.upstream && self.unpushed == 0 {
                    self.rewrote = Some(at);
                }
                self.dirty = false;
            }
            Step::Fetch { remote } => self.remote(at, remote, false),
            Step::Pull { remote, .. } => {
                self.remote(at, remote, false);
                self.over_changes(at, "This pulls");
            }
            Step::Push { remote } => {
                self.remote(at, remote, true);
                if let Some(by) = self.rewrote {
                    self.error(
                        Some(at),
                        "push.needsForce",
                        format!(
                            "Step {} rewrites commits the remote already has, so this push would need force. Recipes never force.",
                            by + 1
                        ),
                    );
                }
                if self.recipe.level == Level::Unattended {
                    if let Some(branch) = self.current.clone() {
                        if self.world.protected.contains(&branch) {
                            self.error(
                                Some(at),
                                "push.protected",
                                format!(
                                    "{branch} is on this repository's never-unattended list. Choose a level that stops before pushing."
                                ),
                            );
                        }
                    }
                }
                self.unpushed = 0;
                self.upstream = true;
                self.rewrote = None;
            }
            Step::Merge { branch, how } => {
                if self.branch(at, branch) && self.current.as_deref() == Some(branch.as_str()) {
                    self.error(
                        Some(at),
                        "merge.itself",
                        format!("{branch} is the branch checked out at this step."),
                    );
                }
                self.over_changes(at, "This merges");
                if *how == MergeHow::Squash {
                    let committed = self.recipe.steps[at + 1..]
                        .iter()
                        .any(|later| matches!(later, Step::Commit { .. }));
                    if !committed {
                        self.warning(
                            Some(at),
                            "merge.squashUncommitted",
                            "A squash merge stages the changes and commits nothing. No Commit follows.",
                        );
                    }
                    self.dirty = true;
                } else {
                    self.unpushed = self.unpushed.saturating_add(1);
                }
            }
            Step::Rebase { onto } => {
                if self.branch(at, onto) && self.current.as_deref() == Some(onto.as_str()) {
                    self.error(
                        Some(at),
                        "rebase.itself",
                        format!("{onto} is the branch checked out at this step."),
                    );
                }
                self.over_changes(at, "This rebases");
                // Only what is known of the branch the recipe started on.
                if self.upstream
                    && self.current == self.world.current
                    && self.world.rewrites_pushed.contains(onto)
                {
                    self.rewrote = Some(at);
                }
            }
            Step::CherryPick { commits } => {
                if commits.is_empty() {
                    self.error(Some(at), "pick.none", "Name at least one commit.");
                }
                for commit in commits {
                    if !revision_ok(commit) {
                        self.error(
                            Some(at),
                            "pick.bad",
                            format!("\"{commit}\" is not a commit."),
                        );
                    }
                }
                self.unpushed = self.unpushed.saturating_add(commits.len() as i64);
            }
            Step::Switch { branch } => {
                self.branch(at, branch);
                self.over_changes(at, "This switches");
                if self.current.as_deref() != Some(branch.as_str()) {
                    self.current = Some(branch.clone());
                    // Another branch: what it has pushed is not known here.
                    self.upstream = false;
                    self.unpushed = UNKNOWN;
                    self.rewrote = None;
                }
            }
            Step::NewBranch { name, from } => {
                if name.is_empty() {
                    self.error(Some(at), "name.none", "Name the branch.");
                } else if !branch_name_ok(name) {
                    self.error(
                        Some(at),
                        "name.bad",
                        format!("\"{name}\" cannot be a branch's name."),
                    );
                } else if self.exists(name) {
                    self.error(
                        Some(at),
                        "name.taken",
                        format!("There is already a branch {name}."),
                    );
                }
                if !from.is_empty() && !self.exists(from) && !revision_ok(from) {
                    self.error(
                        Some(at),
                        "from.bad",
                        format!("\"{from}\" is not a revision."),
                    );
                }
                self.made.push(name.clone());
                self.current = Some(name.clone());
                self.upstream = false;
                self.unpushed = UNKNOWN;
                self.rewrote = None;
            }
            Step::DeleteBranch { branch } => {
                if self.branch(at, branch) {
                    if self.current.as_deref() == Some(branch.as_str()) {
                        self.error(
                            Some(at),
                            "delete.current",
                            format!("{branch} is checked out at this step. Switch away first."),
                        );
                    }
                    let later = self.recipe.steps[at + 1..]
                        .iter()
                        .position(|later| later.uses().contains(&branch.as_str()));
                    if let Some(later) = later {
                        self.error(
                            Some(at),
                            "delete.used",
                            format!("Step {} still uses {branch}.", at + later + 2),
                        );
                    }
                }
            }
            Step::Squash { count, message } => {
                self.message(at, message);
                if *count < 2 {
                    self.error(
                        Some(at),
                        "squash.few",
                        "Squash takes at least the last 2 commits.",
                    );
                } else if *count > 100 {
                    self.error(Some(at), "squash.many", "Squash takes at most 100 commits.");
                }
                if self.upstream && i64::from(*count) > self.unpushed {
                    self.rewrote = Some(at);
                }
                self.unpushed = (self.unpushed - i64::from(*count)).max(0) + 1;
            }
            Step::Reword { message } => {
                self.message(at, message);
                if self.upstream && self.unpushed == 0 {
                    self.rewrote = Some(at);
                }
            }
            Step::Check { command } => {
                if command.is_empty() {
                    self.error(Some(at), "check.none", "Choose a check.");
                } else if !self.world.checks.contains(command) {
                    self.error(
                        Some(at),
                        "check.missing",
                        format!("`{command}` is not one of this repository's checks."),
                    );
                }
            }
            Step::Ask {} => {}
            // It stops unless the tree is clean: below it, it is.
            Step::Clean {} => self.dirty = false,
        }
    }
}

/// Everything wrong with `recipe` in `world`, in the order of its steps, the
/// recipe's own findings first.
pub fn validate(recipe: &Recipe, world: &World) -> Vec<Finding> {
    let mut walk = Walk {
        recipe,
        world,
        found: Vec::new(),
        current: world.current.clone(),
        made: Vec::new(),
        stashes: Vec::new(),
        dirty: world.dirty,
        upstream: world.upstream,
        unpushed: if world.upstream {
            i64::from(world.unpushed)
        } else {
            UNKNOWN
        },
        rewrote: None,
    };

    let name = recipe.name.trim();
    if name.is_empty() {
        walk.error(None, "recipe.unnamed", "Name the recipe.");
    } else if name.chars().count() > MAX_NAME {
        walk.error(
            None,
            "recipe.longName",
            format!("The name is at most {MAX_NAME} characters."),
        );
    }
    if recipe.steps.is_empty() {
        walk.error(None, "recipe.empty", "Add a block.");
    }
    if recipe.steps.len() > MAX_STEPS {
        walk.error(
            None,
            "recipe.long",
            format!(
                "{} steps. A recipe has at most {MAX_STEPS}.",
                recipe.steps.len()
            ),
        );
    }
    if let Some(operation) = &world.operation {
        walk.error(
            None,
            "repo.busy",
            format!("A {operation} is in progress. Finish or abort it first."),
        );
    }
    if !recipe.branch.is_empty() && world.current.as_deref() != Some(recipe.branch.as_str()) {
        walk.error(
            None,
            "recipe.otherBranch",
            match &world.current {
                Some(current) => format!(
                    "This recipe runs on {}, and {current} is checked out.",
                    recipe.branch
                ),
                None => format!(
                    "This recipe runs on {}, and no branch is checked out.",
                    recipe.branch
                ),
            },
        );
    }
    if recipe.agent_help {
        if recipe.note.chars().count() > MAX_NOTE {
            walk.error(
                None,
                "note.long",
                format!(
                    "The note is {} characters. The most is {MAX_NOTE}.",
                    recipe.note.chars().count()
                ),
            );
        }
        // An agent is only called for a message. A stack that asks for none
        // runs without one, whoever is chosen.
        if recipe.steps.iter().any(Step::agent_writes) {
            match &recipe.agent {
                None => walk.error(
                    None,
                    "agent.none",
                    "A step's message is left to the agent. Choose one.",
                ),
                Some(agent) if !world.agents.contains(agent) => walk.error(
                    None,
                    "agent.unusable",
                    "The agent chosen cannot be used in this repository.",
                ),
                Some(_) => {}
            }
        }
    }
    if recipe.level > world.highest {
        walk.warning(
            None,
            "level.capped",
            format!(
                "This repository allows at most \"{}\". The recipe runs at that.",
                world.highest.label()
            ),
        );
    }

    for (at, step) in recipe.steps.iter().enumerate() {
        walk.step(at, step);
    }
    // A stash that nothing pops is left where it is, and says so.
    let left = std::mem::take(&mut walk.stashes);
    for at in left {
        walk.warning(
            Some(at),
            "stash.kept",
            "No Pop follows: the changes stay stashed.",
        );
    }

    walk.found.sort_by_key(|finding| finding.step);
    walk.found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::PullMode;

    fn world() -> World {
        World {
            branches: vec!["main".into(), "feature/avatars".into(), "old".into()],
            remotes: vec!["origin".into()],
            current: Some("feature/avatars".into()),
            dirty: false,
            operation: None,
            upstream: true,
            unpushed: 2,
            rewrites_pushed: Vec::new(),
            protected: vec!["main".into()],
            checks: vec!["cargo test".into()],
            agents: vec!["claude".into()],
            highest: Level::Unattended,
        }
    }

    fn recipe(steps: Vec<Step>) -> Recipe {
        Recipe {
            steps,
            ..Recipe::named("r")
        }
    }

    fn codes(recipe: &Recipe, world: &World) -> Vec<(Option<usize>, &'static str)> {
        validate(recipe, world)
            .into_iter()
            .map(|finding| (finding.step, finding.code))
            .collect()
    }

    fn commit(text: &str) -> Step {
        Step::Commit {
            all: true,
            message: Message::text(text),
        }
    }

    fn push() -> Step {
        Step::Push {
            remote: "origin".into(),
        }
    }

    fn merge(branch: &str) -> Step {
        Step::Merge {
            branch: branch.into(),
            how: MergeHow::Commit,
        }
    }

    fn pull() -> Step {
        Step::Pull {
            remote: "origin".into(),
            mode: PullMode::FastForwardOnly,
        }
    }

    /// The spec's own example: stash, pull, merge, pop, commit, push.
    #[test]
    fn the_specs_recipe_has_nothing_wrong_with_it() {
        let sync = recipe(vec![
            Step::Stash { untracked: true },
            pull(),
            merge("main"),
            Step::Pop {},
            commit("Sync with main"),
            push(),
        ]);
        let mut dirty = world();
        dirty.dirty = true;

        assert_eq!(validate(&sync, &dirty), []);
    }

    #[test]
    fn a_pop_needs_a_stash_above_it() {
        assert_eq!(
            codes(&recipe(vec![Step::Pop {}]), &world()),
            [(Some(0), "pop.noStash")]
        );
        // One stash is popped once.
        let twice = recipe(vec![
            Step::Stash { untracked: false },
            Step::Pop {},
            Step::Pop {},
        ]);
        assert_eq!(codes(&twice, &world()), [(Some(2), "pop.noStash")]);
        // A stash the repository already had is not the recipe's to pop.
        assert_eq!(
            codes(
                &recipe(vec![Step::Stash { untracked: false }, Step::Pop {}]),
                &world()
            ),
            []
        );
    }

    #[test]
    fn a_stash_nothing_pops_stays_stashed_and_says_so() {
        let found = validate(&recipe(vec![Step::Stash { untracked: true }]), &world());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].code, "stash.kept");
        assert_eq!(errors(&found), 0);
    }

    /// Force is not available, so a push that would need it cannot be in a
    /// stack that runs.
    #[test]
    fn a_push_after_rewriting_what_the_remote_has_is_an_error() {
        let mut pushed = world();
        pushed.unpushed = 0;

        for rewrite in [
            Step::Amend { message: None },
            Step::Reword {
                message: Message::text("New words"),
            },
            Step::Squash {
                count: 2,
                message: Message::text("One"),
            },
        ] {
            let stack = recipe(vec![rewrite.clone(), push()]);
            assert_eq!(
                codes(&stack, &pushed),
                [(Some(1), "push.needsForce")],
                "{rewrite:?}"
            );
        }

        let mut onto_main = pushed.clone();
        onto_main.rewrites_pushed = vec!["main".into()];
        let rebased = recipe(vec![
            Step::Rebase {
                onto: "main".into(),
            },
            push(),
        ]);
        assert_eq!(codes(&rebased, &onto_main), [(Some(1), "push.needsForce")]);
        // The same rebase replays only what is not pushed: nothing to force.
        assert_eq!(codes(&rebased, &pushed), []);
    }

    #[test]
    fn rewriting_commits_the_remote_does_not_have_is_no_error() {
        // Two commits are unpushed: the last one can be amended and reworded,
        // and both squashed, and then pushed.
        for rewrite in [
            Step::Amend { message: None },
            Step::Reword {
                message: Message::text("New words"),
            },
            Step::Squash {
                count: 2,
                message: Message::text("One"),
            },
        ] {
            assert_eq!(codes(&recipe(vec![rewrite, push()]), &world()), []);
        }
        // Three is one more than the upstream lacks.
        let three = recipe(vec![
            Step::Squash {
                count: 3,
                message: Message::text("One"),
            },
            push(),
        ]);
        assert_eq!(codes(&three, &world()), [(Some(1), "push.needsForce")]);
    }

    #[test]
    fn a_commit_made_in_the_recipe_can_be_amended_before_it_is_pushed() {
        let mut pushed = world();
        pushed.unpushed = 0;
        let stack = recipe(vec![commit("First"), Step::Amend { message: None }, push()]);

        assert_eq!(codes(&stack, &pushed), []);
        // After the push it is the remote's too.
        let after = recipe(vec![
            commit("First"),
            push(),
            Step::Amend { message: None },
            push(),
        ]);
        assert_eq!(codes(&after, &pushed), [(Some(3), "push.needsForce")]);
    }

    #[test]
    fn a_branch_with_no_upstream_has_pushed_nothing_to_rewrite() {
        let mut local = world();
        local.upstream = false;
        local.unpushed = 0;

        let stack = recipe(vec![Step::Amend { message: None }, push()]);

        assert_eq!(codes(&stack, &local), []);
    }

    #[test]
    fn a_branch_is_not_deleted_while_checked_out_or_while_a_later_step_uses_it() {
        let here = recipe(vec![Step::DeleteBranch {
            branch: "feature/avatars".into(),
        }]);
        assert_eq!(codes(&here, &world()), [(Some(0), "delete.current")]);

        let used = recipe(vec![
            Step::DeleteBranch {
                branch: "old".into(),
            },
            merge("old"),
        ]);
        let found = validate(&used, &world());
        assert_eq!(found[0].code, "delete.used");
        assert_eq!(found[0].text, "Step 2 still uses old.");

        // Checked out by a Switch above, not when the recipe starts.
        let switched = recipe(vec![
            Step::Switch {
                branch: "old".into(),
            },
            Step::DeleteBranch {
                branch: "old".into(),
            },
        ]);
        assert_eq!(codes(&switched, &world()), [(Some(1), "delete.current")]);

        // Merged, switched away from, and then deleted: the way to land a branch.
        let landed = recipe(vec![
            Step::Switch {
                branch: "main".into(),
            },
            merge("feature/avatars"),
            Step::DeleteBranch {
                branch: "feature/avatars".into(),
            },
        ]);
        assert_eq!(codes(&landed, &world()), []);
    }

    #[test]
    fn a_branch_must_exist_or_be_made_by_a_step_above() {
        assert_eq!(
            codes(&recipe(vec![merge("nowhere")]), &world()),
            [(Some(0), "branch.missing")]
        );
        assert_eq!(
            codes(&recipe(vec![merge("")]), &world()),
            [(Some(0), "branch.none")]
        );

        let made = recipe(vec![
            Step::NewBranch {
                name: "topic".into(),
                from: String::new(),
            },
            Step::Switch {
                branch: "main".into(),
            },
            merge("topic"),
        ]);
        assert_eq!(codes(&made, &world()), []);
        // Made below is not made above.
        let late = recipe(vec![
            merge("topic"),
            Step::NewBranch {
                name: "topic".into(),
                from: String::new(),
            },
        ]);
        assert_eq!(codes(&late, &world()), [(Some(0), "branch.missing")]);
    }

    #[test]
    fn the_limits_are_thirty_steps_seventy_two_characters_and_a_note_of_a_hundred_and_forty() {
        let long = recipe(vec![Step::Ask {}; 31]);
        assert_eq!(codes(&long, &world()), [(None, "recipe.long")]);
        assert_eq!(codes(&recipe(vec![Step::Ask {}; 30]), &world()), []);

        let wordy = recipe(vec![commit(&"m".repeat(73))]);
        assert_eq!(codes(&wordy, &world()), [(Some(0), "message.long")]);
        assert_eq!(codes(&recipe(vec![commit(&"m".repeat(72))]), &world()), []);
        // Characters, not bytes.
        assert_eq!(codes(&recipe(vec![commit(&"é".repeat(72))]), &world()), []);

        let mut noted = recipe(vec![Step::Ask {}]);
        noted.agent_help = true;
        noted.note = "n".repeat(141);
        assert_eq!(codes(&noted, &world()), [(None, "note.long")]);
        noted.note = "n".repeat(140);
        assert_eq!(codes(&noted, &world()), []);
    }

    #[test]
    fn a_message_is_one_line_with_something_on_it() {
        assert_eq!(
            codes(&recipe(vec![commit("  ")]), &world()),
            [(Some(0), "message.empty")]
        );
        assert_eq!(
            codes(&recipe(vec![commit("one\ntwo")]), &world()),
            [(Some(0), "message.lines")]
        );
    }

    /// The author's note: off, *agent writes* is not there to choose.
    #[test]
    fn a_message_left_to_the_agent_needs_the_agents_help_and_an_agent() {
        let mut stack = recipe(vec![Step::Commit {
            all: true,
            message: Message::Agent,
        }]);
        assert_eq!(codes(&stack, &world()), [(Some(0), "message.agentOff")]);

        stack.agent_help = true;
        assert_eq!(codes(&stack, &world()), [(None, "agent.none")]);

        stack.agent = Some("gone".into());
        assert_eq!(codes(&stack, &world()), [(None, "agent.unusable")]);

        stack.agent = Some("claude".into());
        assert_eq!(codes(&stack, &world()), []);
    }

    #[test]
    fn with_the_help_off_the_note_and_the_agent_are_not_read() {
        let mut stack = recipe(vec![commit("Typed")]);
        stack.note = "n".repeat(500);
        stack.agent = Some("gone".into());

        assert_eq!(codes(&stack, &world()), []);
    }

    #[test]
    fn with_the_help_on_a_stack_that_asks_the_agent_for_nothing_needs_no_agent() {
        let mut stack = recipe(vec![commit("Typed")]);
        stack.agent_help = true;

        assert_eq!(codes(&stack, &world()), []);
    }

    #[test]
    fn changes_in_the_tree_that_nothing_puts_away_are_warned_about() {
        let mut dirty = world();
        dirty.dirty = true;

        for step in [
            pull(),
            merge("main"),
            Step::Rebase {
                onto: "main".into(),
            },
            Step::Switch {
                branch: "main".into(),
            },
        ] {
            let found = validate(&recipe(vec![step.clone()]), &dirty);
            assert_eq!(found.len(), 1, "{step:?}");
            assert_eq!(found[0].code, "dirty");
            assert_eq!(found[0].severity, Severity::Warning);
            // Clean, there is nothing to warn about.
            assert_eq!(codes(&recipe(vec![step]), &world()), []);
        }

        // A stash, a commit and a guard each put them away.
        for above in [
            Step::Stash { untracked: true },
            commit("Save"),
            Step::Clean {},
        ] {
            let stack = recipe(vec![above.clone(), merge("main")]);
            let found = codes(&stack, &dirty);
            assert!(!found.contains(&(Some(1), "dirty")), "{above:?}: {found:?}");
        }
        // And a pop brings them back.
        let popped = recipe(vec![
            Step::Stash { untracked: true },
            Step::Pop {},
            merge("main"),
        ]);
        assert_eq!(codes(&popped, &world()), [(Some(2), "dirty")]);
    }

    #[test]
    fn a_push_to_a_never_unattended_branch_is_an_error_only_at_full_access() {
        let mut on_main = world();
        on_main.current = Some("main".into());
        let mut stack = recipe(vec![push()]);

        for level in [Level::Suggest, Level::StepByStep, Level::SignOff] {
            stack.level = level;
            assert_eq!(codes(&stack, &on_main), [], "{level:?}");
        }
        stack.level = Level::Unattended;
        assert_eq!(codes(&stack, &on_main), [(Some(0), "push.protected")]);
        // Another branch is pushed unattended.
        assert_eq!(codes(&stack, &world()), []);
        // And the branch is the one checked out at the push, not at the start.
        let switched = Recipe {
            level: Level::Unattended,
            ..recipe(vec![
                Step::Switch {
                    branch: "main".into(),
                },
                push(),
            ])
        };
        assert_eq!(codes(&switched, &world()), [(Some(1), "push.protected")]);
    }

    #[test]
    fn a_level_above_the_repositorys_highest_runs_at_the_highest_and_says_so() {
        let mut capped = world();
        capped.highest = Level::SignOff;
        let stack = Recipe {
            level: Level::Unattended,
            ..recipe(vec![Step::Ask {}])
        };

        let found = validate(&stack, &capped);

        assert_eq!(found[0].code, "level.capped");
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(found[0].text.contains("Ask before pushing"));
    }

    #[test]
    fn a_new_branch_needs_a_name_git_would_take_that_nothing_has() {
        let named = |name: &str| {
            recipe(vec![Step::NewBranch {
                name: name.into(),
                from: String::new(),
            }])
        };

        assert_eq!(codes(&named(""), &world()), [(Some(0), "name.none")]);
        assert_eq!(codes(&named("main"), &world()), [(Some(0), "name.taken")]);
        for bad in [
            "two words",
            "-option",
            "a..b",
            "ends.",
            "ends/",
            "/starts",
            "a//b",
            "x.lock",
            ".hidden",
            "a/.b",
            "what?",
            "star*",
            "a:b",
            "a~1",
            "a^",
            "a[b",
            "back\\slash",
            "@",
            "a@{b",
        ] {
            assert!(!branch_name_ok(bad), "{bad}");
            assert_eq!(
                codes(&named(bad), &world()),
                [(Some(0), "name.bad")],
                "{bad}"
            );
        }
        for good in ["topic", "feature/avatars-2", "fix_1.2", "user@work"] {
            assert!(branch_name_ok(good), "{good}");
            assert_eq!(codes(&named(good), &world()), [], "{good}");
        }
    }

    #[test]
    fn a_new_branch_starts_from_a_branch_or_something_that_could_be_a_revision() {
        let from = |from: &str| {
            recipe(vec![Step::NewBranch {
                name: "topic".into(),
                from: from.into(),
            }])
        };

        assert_eq!(codes(&from("main"), &world()), []);
        assert_eq!(codes(&from("v1.0"), &world()), []);
        assert_eq!(codes(&from("a3f9c1e"), &world()), []);
        assert_eq!(codes(&from("--orphan"), &world()), [(Some(0), "from.bad")]);
        assert_eq!(codes(&from("two words"), &world()), [(Some(0), "from.bad")]);
    }

    #[test]
    fn a_remote_must_be_one_the_repository_has() {
        let to = |remote: &str| {
            recipe(vec![Step::Push {
                remote: remote.into(),
            }])
        };
        assert_eq!(codes(&to(""), &world()), [(Some(0), "remote.none")]);
        assert_eq!(codes(&to("fork"), &world()), [(Some(0), "remote.missing")]);

        // A fetch of every remote and a pull from the upstream name none.
        let quiet = recipe(vec![
            Step::Fetch {
                remote: String::new(),
            },
            Step::Pull {
                remote: String::new(),
                mode: PullMode::Merge,
            },
        ]);
        assert_eq!(codes(&quiet, &world()), []);
        let gone = recipe(vec![Step::Fetch {
            remote: "fork".into(),
        }]);
        assert_eq!(codes(&gone, &world()), [(Some(0), "remote.missing")]);
    }

    #[test]
    fn a_merge_or_a_rebase_is_not_of_the_branch_onto_itself() {
        assert_eq!(
            codes(&recipe(vec![merge("feature/avatars")]), &world()),
            [(Some(0), "merge.itself")]
        );
        let onto_itself = recipe(vec![Step::Rebase {
            onto: "feature/avatars".into(),
        }]);
        assert_eq!(codes(&onto_itself, &world()), [(Some(0), "rebase.itself")]);
    }

    #[test]
    fn a_squash_merge_with_no_commit_after_it_leaves_the_changes_staged() {
        let squash = Step::Merge {
            branch: "main".into(),
            how: MergeHow::Squash,
        };

        assert_eq!(
            codes(&recipe(vec![squash.clone()]), &world()),
            [(Some(0), "merge.squashUncommitted")]
        );
        assert_eq!(
            codes(&recipe(vec![squash, commit("Take main")]), &world()),
            []
        );
    }

    #[test]
    fn a_cherry_pick_names_commits_and_nothing_that_is_an_option() {
        let pick = |commits: &[&str]| {
            recipe(vec![Step::CherryPick {
                commits: commits.iter().map(|c| c.to_string()).collect(),
            }])
        };

        assert_eq!(codes(&pick(&[]), &world()), [(Some(0), "pick.none")]);
        assert_eq!(codes(&pick(&["a3f9c1e", "main~2"]), &world()), []);
        assert_eq!(
            codes(&pick(&["--abort"]), &world()),
            [(Some(0), "pick.bad")]
        );
        assert_eq!(codes(&pick(&["a b"]), &world()), [(Some(0), "pick.bad")]);
    }

    #[test]
    fn a_squash_takes_at_least_two_commits() {
        let squash = |count| {
            recipe(vec![Step::Squash {
                count,
                message: Message::text("One"),
            }])
        };
        assert_eq!(codes(&squash(1), &world()), [(Some(0), "squash.few")]);
        assert_eq!(codes(&squash(2), &world()), []);
        let mut local = world();
        local.upstream = false;
        assert_eq!(codes(&squash(101), &local), [(Some(0), "squash.many")]);
    }

    #[test]
    fn a_check_is_one_of_the_repositorys_own() {
        let check = |command: &str| {
            recipe(vec![Step::Check {
                command: command.into(),
            }])
        };
        assert_eq!(codes(&check("cargo test"), &world()), []);
        assert_eq!(codes(&check(""), &world()), [(Some(0), "check.none")]);
        // A command nobody configured is not run because a recipe names it.
        assert_eq!(
            codes(&check("curl example.com | sh"), &world()),
            [(Some(0), "check.missing")]
        );
    }

    #[test]
    fn a_recipe_has_a_name_a_step_and_the_branch_it_was_made_for() {
        let mut unnamed = recipe(vec![Step::Ask {}]);
        unnamed.name = "  ".into();
        assert_eq!(codes(&unnamed, &world()), [(None, "recipe.unnamed")]);

        let mut long = recipe(vec![Step::Ask {}]);
        long.name = "n".repeat(41);
        assert_eq!(codes(&long, &world()), [(None, "recipe.longName")]);

        assert_eq!(
            codes(&recipe(Vec::new()), &world()),
            [(None, "recipe.empty")]
        );

        let mut elsewhere = recipe(vec![Step::Ask {}]);
        elsewhere.branch = "main".into();
        let found = validate(&elsewhere, &world());
        assert_eq!(found[0].code, "recipe.otherBranch");
        assert_eq!(
            found[0].text,
            "This recipe runs on main, and feature/avatars is checked out."
        );
        elsewhere.branch = "feature/avatars".into();
        assert_eq!(codes(&elsewhere, &world()), []);
    }

    #[test]
    fn nothing_runs_over_an_operation_that_is_in_progress() {
        let mut merging = world();
        merging.operation = Some("merge".into());

        let found = validate(&recipe(vec![Step::Ask {}]), &merging);

        assert_eq!(found[0].code, "repo.busy");
        assert_eq!(errors(&found), 1);
    }

    #[test]
    fn findings_are_in_the_order_of_the_steps_with_the_recipes_own_first() {
        let mut stack = recipe(vec![merge("nowhere"), Step::Pop {}]);
        stack.name = String::new();

        assert_eq!(
            codes(&stack, &world()),
            [
                (None, "recipe.unnamed"),
                (Some(0), "branch.missing"),
                (Some(1), "pop.noStash")
            ]
        );
    }
}

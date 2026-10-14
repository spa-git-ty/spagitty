// SPDX-License-Identifier: GPL-3.0-or-later

//! What an agent is told about a recipe (FEAT-133).
//!
//! Two parts, in order: `recipes.md`, which ships with Spagitty and is the
//! same for every recipe, and then this recipe, generated from its stack. The
//! person sees both, exactly as they are sent.
//!
//! Only with the agent's help on. With it off nobody is briefed, and there is
//! no brief.

use super::command::{commands, shown_arg};
use super::{Level, Recipe, Step};

/// The guide every brief begins with, compiled in, as the file is on disk.
const GUIDE: &str = include_str!("recipes.md");

/// The guide, with the line endings a checkout on Windows may have given it
/// taken out: what is sent is the same text from every build.
pub fn guide() -> String {
    GUIDE.replace("\r\n", "\n")
}

/// The guide's own version, from its `Version:` line. Shown on the screen so
/// the person knows which rules an agent was given.
pub fn guide_version() -> u32 {
    GUIDE
        .lines()
        .find_map(|line| line.strip_prefix("Version:"))
        .and_then(|version| version.trim().parse().ok())
        .unwrap_or(0)
}

/// What a brief's header says besides the recipe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct About<'a> {
    /// The repository's name.
    pub repo: &'a str,
    /// The branch checked out.
    pub branch: &'a str,
    /// The agent's name, as Settings shows it.
    pub agent: &'a str,
    /// The level it runs at, after the repository's cap.
    pub level: Level,
    /// The person's name, for the note's heading.
    pub person: &'a str,
}

/// The commands a recipe may run, each once, as git subcommands with their
/// options and without the names and messages: `git commit -m`.
pub fn allowed(recipe: &Recipe, prune: bool) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    let mut add = |line: String| {
        if !all.contains(&line) {
            all.push(line);
        }
    };
    for step in &recipe.steps {
        if let Step::Check { command } = step {
            add(command.clone());
            continue;
        }
        for args in commands(step, prune, None) {
            // The subcommand and its options. What follows the first word
            // that is neither is a name or a message, which is this recipe's
            // and not a kind of command.
            let mut line = String::from("git");
            for (at, arg) in args.iter().enumerate() {
                let option = arg.starts_with('-');
                let second = at == 1 && matches!(args[0].as_str(), "stash");
                if at == 0 || option || second {
                    line.push(' ');
                    line.push_str(&shown_arg(arg));
                } else {
                    break;
                }
            }
            add(line);
        }
    }
    // Always, because reading them is how a failure is explained.
    add("git status".into());
    add("git log".into());
    all
}

/// The generated half: this recipe, in the spec's form (§8.5).
pub fn recipe_section(recipe: &Recipe, about: &About<'_>, prune: bool) -> String {
    let mut text = String::new();
    text.push_str("---\n");
    text.push_str(&format!("recipe: {}\n", recipe.name));
    text.push_str(&format!(
        "repo: {} · branch: {}\n",
        about.repo,
        if about.branch.is_empty() {
            "(detached)"
        } else {
            about.branch
        }
    ));
    text.push_str(&format!(
        "agent: {} · level: {}\n",
        about.agent,
        about.level.word()
    ));
    text.push_str("restore-point: yes\n");
    text.push_str("---\n");
    text.push_str("# These are the steps, in order\n");
    for index in 0..recipe.steps.len() {
        text.push_str(&format!(
            "{}. {}\n",
            index + 1,
            recipe.describe(index, about.branch)
        ));
    }
    text.push_str("\n## Allowed commands\n");
    text.push_str(&allowed(recipe, prune).join(" · "));
    text.push_str("\n\n## Never\n");
    text.push_str("- force-push, reset --hard, clean -f\n");
    text.push_str("- touch a branch not named above\n");
    text.push_str("- edit files\n");
    text.push_str("\n## When a step fails\n");
    text.push_str(
        "Spagitty stops. It gives you the step, the command, its exit code and `git status`.\n",
    );
    let note = recipe.note.trim();
    if !note.is_empty() {
        text.push_str(&format!(
            "\n## Note from {}\n{note}\n",
            if about.person.is_empty() {
                "the person"
            } else {
                about.person
            }
        ));
    }
    text
}

/// The whole brief: the guide, then the recipe.
pub fn brief(recipe: &Recipe, about: &About<'_>, prune: bool) -> String {
    format!(
        "{}\n{}",
        guide().trim_end(),
        recipe_section(recipe, about, prune)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::{MergeHow, Message};
    use crate::shell::PullMode;

    fn sync() -> Recipe {
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
            agent_help: true,
            agent: Some("claude".into()),
            note: "Keep my merge commit message as git suggests it.".into(),
            ..Recipe::named("sync-and-ship")
        }
    }

    fn about() -> About<'static> {
        About {
            repo: "spagitty",
            branch: "feature/avatars",
            agent: "Claude Code",
            level: Level::StepByStep,
            person: "Ada",
        }
    }

    #[test]
    fn the_guide_carries_its_own_version() {
        assert_eq!(guide_version(), 1);
        assert!(guide().starts_with("# Spagitty recipes"));
        assert!(!guide().contains('\r'));
    }

    /// The agent is told what it may never be told otherwise.
    #[test]
    fn the_guide_says_the_agent_runs_nothing_and_signs_nothing() {
        let guide = guide();
        assert!(guide.contains("You run nothing."));
        assert!(guide.contains("Do not edit files, and do not run commands."));
        assert!(guide.contains("Never add\ntrailers, co-author lines"));
        assert!(guide.contains("at most 72\ncharacters"));
    }

    #[test]
    fn the_generated_section_is_the_specs_form() {
        let section = recipe_section(&sync(), &about(), false);

        assert_eq!(
            section,
            "---\n\
recipe: sync-and-ship\n\
repo: spagitty · branch: feature/avatars\n\
agent: Claude Code · level: step\n\
restore-point: yes\n\
---\n\
# These are the steps, in order\n\
1. Stash — include untracked\n\
2. Pull from origin — fast-forward only\n\
3. Merge main into feature/avatars — always a merge commit\n\
4. Pop the stash from step 1 — on conflict, stop\n\
5. Commit all changes — message: \"Sync with main\"\n\
6. Push to origin — never force\n\
\n\
## Allowed commands\n\
git stash push -u · git pull --no-edit --ff-only · git merge --no-ff --no-edit · git stash pop · git add -A · git commit -m · git push --set-upstream · git status · git log\n\
\n\
## Never\n\
- force-push, reset --hard, clean -f\n\
- touch a branch not named above\n\
- edit files\n\
\n\
## When a step fails\n\
Spagitty stops. It gives you the step, the command, its exit code and `git status`.\n\
\n\
## Note from Ada\n\
Keep my merge commit message as git suggests it.\n"
        );
    }

    #[test]
    fn the_brief_is_the_guide_and_then_the_recipe() {
        let whole = brief(&sync(), &about(), false);

        assert!(whole.starts_with("# Spagitty recipes"));
        let guide_ends = whole
            .find("---\nrecipe: sync-and-ship")
            .expect("the recipe");
        assert!(whole[..guide_ends].contains("## Talking"));
        assert!(whole.ends_with("Keep my merge commit message as git suggests it.\n"));
    }

    #[test]
    fn a_recipe_with_no_note_has_no_note_section() {
        let mut quiet = sync();
        quiet.note = "   ".into();

        assert!(!recipe_section(&quiet, &about(), false).contains("## Note"));
    }

    #[test]
    fn allowed_commands_are_each_listed_once_without_names_or_messages() {
        let mut twice = sync();
        twice.steps.push(Step::Commit {
            all: false,
            message: Message::text("Another message entirely"),
        });
        twice.steps.push(Step::Check {
            command: "cargo test".into(),
        });
        twice.steps.push(Step::Switch {
            branch: "main".into(),
        });

        let list = allowed(&twice, true);

        assert_eq!(
            list.iter().filter(|line| *line == "git commit -m").count(),
            1
        );
        assert!(list.contains(&"cargo test".to_string()));
        assert!(list.contains(&"git switch".to_string()));
        assert!(!list.iter().any(|line| line.contains("Another")));
        assert!(!list.iter().any(|line| line.contains("origin")));
    }

    #[test]
    fn a_detached_head_and_an_unnamed_person_are_said_plainly() {
        let nowhere = About {
            branch: "",
            person: "",
            ..about()
        };

        let section = recipe_section(&sync(), &nowhere, false);

        assert!(section.contains("branch: (detached)"));
        assert!(section.contains("3. Merge main into this branch"));
        assert!(section.contains("## Note from the person"));
    }
}

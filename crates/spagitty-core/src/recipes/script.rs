// SPDX-License-Identifier: GPL-3.0-or-later

//! A recipe as a shell script (FEAT-133).
//!
//! *Copy as script*: the same steps as a POSIX shell script with `set -e`,
//! each step a commented line. It is the way out at the *Read-only* level,
//! where nothing runs, and what a person takes to a machine without Spagitty.
//!
//! It is the steps and not the run: no restore point, no stops for the level,
//! and a message left to the agent is left to the person's editor.

use super::command::commands;
use super::{Message, Recipe, Step};

/// An argument for a POSIX shell: as it is when it is plainly one word, and
/// in single quotes otherwise.
pub fn sh_arg(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+,~".contains(c));
    if plain {
        arg.to_string()
    } else {
        // A single quote ends the quoting, is written escaped, and starts it again.
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn line(args: &[String]) -> String {
    let mut text = String::from("git");
    for arg in args {
        text.push(' ');
        text.push_str(&sh_arg(arg));
    }
    text
}

/// The recipe as a script. `start` is the branch it was built on, for the
/// comments.
pub fn script(recipe: &Recipe, start: &str, prune: bool) -> String {
    let mut text = String::from("#!/bin/sh\n");
    text.push_str(&format!("# {} — a Spagitty recipe\n", recipe.name));
    if !recipe.branch.is_empty() {
        text.push_str(&format!("# Made to run on {}\n", recipe.branch));
    }
    text.push_str("set -e\n");
    for (index, step) in recipe.steps.iter().enumerate() {
        text.push_str(&format!(
            "\n# {}. {}\n",
            index + 1,
            recipe.describe(index, start)
        ));
        match step {
            Step::Check { command } => {
                text.push_str(command);
                text.push('\n');
            }
            Step::Ask {} => {
                text.push_str("printf 'Go on? Enter to continue, Ctrl+C to stop. '\nread _\n");
            }
            Step::Clean {} => {
                text.push_str("test -z \"$(git status --porcelain)\"\n");
            }
            _ if step.agent_writes() => {
                // No agent here: git opens the editor for the message.
                for mut args in commands(step, prune, None) {
                    if let Some(at) = args.iter().position(|arg| arg == "-m") {
                        args.drain(at..at + 2);
                    }
                    text.push_str(&line(&args));
                    text.push('\n');
                }
            }
            _ => {
                for args in commands(step, prune, None) {
                    text.push_str(&line(&args));
                    text.push('\n');
                }
            }
        }
    }
    text
}

/// True when the script has a step whose message git will ask for.
pub fn asks_for_a_message(recipe: &Recipe) -> bool {
    recipe
        .steps
        .iter()
        .any(|step| matches!(step.message(), Some(Message::Agent)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::MergeHow;
    use crate::shell::PullMode;

    #[test]
    fn the_script_is_the_steps_each_under_its_comment() {
        let recipe = Recipe {
            branch: "feature/avatars".into(),
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
        };

        assert_eq!(
            script(&recipe, "feature/avatars", false),
            "#!/bin/sh\n\
# sync-and-ship — a Spagitty recipe\n\
# Made to run on feature/avatars\n\
set -e\n\
\n\
# 1. Stash — include untracked\n\
git stash push -u\n\
\n\
# 2. Pull from origin — fast-forward only\n\
git pull --no-edit --ff-only origin\n\
\n\
# 3. Merge main into feature/avatars — always a merge commit\n\
git merge --no-ff --no-edit main\n\
\n\
# 4. Pop the stash from step 1 — on conflict, stop\n\
git stash pop\n\
\n\
# 5. Commit all changes — message: \"Sync with main\"\n\
git add -A\n\
git commit -m 'Sync with main'\n\
\n\
# 6. Push to origin — never force\n\
git push --set-upstream origin HEAD\n"
        );
    }

    #[test]
    fn the_guards_are_written_as_what_a_shell_can_do() {
        let recipe = Recipe {
            steps: vec![
                Step::Clean {},
                Step::Check {
                    command: "cargo test --workspace".into(),
                },
                Step::Ask {},
            ],
            ..Recipe::named("guarded")
        };

        let text = script(&recipe, "main", false);

        assert!(text.contains("test -z \"$(git status --porcelain)\"\n"));
        assert!(text.contains("\ncargo test --workspace\n"));
        assert!(text.contains("read _\n"));
    }

    /// With no agent to write it, the message is the editor's.
    #[test]
    fn a_message_left_to_the_agent_is_left_to_the_editor() {
        let recipe = Recipe {
            steps: vec![
                Step::Commit {
                    all: true,
                    message: Message::Agent,
                },
                Step::Squash {
                    count: 2,
                    message: Message::Agent,
                },
            ],
            ..Recipe::named("agent-writes")
        };

        let text = script(&recipe, "main", false);

        assert!(text.contains("git add -A\ngit commit\n"));
        assert!(text.contains("git reset --soft HEAD~2\ngit commit\n"));
        assert!(!text.contains("agent writes>"));
        assert!(asks_for_a_message(&recipe));
    }

    #[test]
    fn what_is_not_plainly_one_word_is_quoted_for_the_shell() {
        assert_eq!(sh_arg("main"), "main");
        assert_eq!(sh_arg("feature/avatars-2"), "feature/avatars-2");
        assert_eq!(sh_arg("Sync with main"), "'Sync with main'");
        assert_eq!(sh_arg("it's done"), "'it'\\''s done'");
        assert_eq!(sh_arg("$(rm -rf .)"), "'$(rm -rf .)'");
        assert_eq!(sh_arg("a;b"), "'a;b'");
        assert_eq!(sh_arg(""), "''");
    }
}

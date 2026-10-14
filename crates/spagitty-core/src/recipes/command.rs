// SPDX-License-Identifier: GPL-3.0-or-later

//! A block to the git commands it runs (FEAT-133).
//!
//! One function, read twice: for the command shown beside a block, and for
//! the command that is run. They cannot disagree, because they are the same
//! list.
//!
//! A command is a list of arguments and never a line for a shell. A branch
//! called `main; rm -rf .` is one argument git refuses, not two commands.

use super::{MergeHow, Message, Step};
use crate::shell::PullMode;

/// What stands where a message will be, until the agent has written it.
pub const AGENT_WRITES: &str = "<agent writes>";

fn own(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

/// The git commands `step` is, in order, each without the leading `git`.
///
/// `prune` is the person's *Prune when fetching* setting. `written` is the
/// message the agent wrote, for a step set to *agent writes*; without one the
/// message is [`AGENT_WRITES`], which is what is shown before a run.
///
/// Run checks and *Stop and ask me* run no git command and return none.
pub fn commands(step: &Step, prune: bool, written: Option<&str>) -> Vec<Vec<String>> {
    let message = |message: &Message| match message {
        Message::Text { text } => text.clone(),
        Message::Agent => written.unwrap_or(AGENT_WRITES).to_string(),
    };
    match step {
        Step::Stash { untracked: true } => vec![own(&["stash", "push", "-u"])],
        Step::Stash { untracked: false } => vec![own(&["stash", "push"])],
        Step::Pop {} => vec![own(&["stash", "pop"])],
        Step::Commit { all, message: text } => {
            let mut all_of_it = Vec::new();
            if *all {
                all_of_it.push(own(&["add", "-A"]));
            }
            all_of_it.push(own(&["commit", "-m", &message(text)]));
            all_of_it
        }
        Step::Amend { message: None } => vec![own(&["commit", "--amend", "--no-edit"])],
        Step::Amend {
            message: Some(text),
        } => vec![own(&["commit", "--amend", "-m", &message(text)])],
        Step::Fetch { remote } => {
            let mut args = own(&["fetch"]);
            if prune {
                args.push("--prune".into());
            }
            args.push(if remote.is_empty() {
                "--all".into()
            } else {
                remote.clone()
            });
            vec![args]
        }
        Step::Pull { remote, mode } => {
            let mut args = own(&["pull", "--no-edit"]);
            args.push(
                match mode {
                    PullMode::FastForwardOnly => "--ff-only",
                    PullMode::Merge => "--no-rebase",
                    PullMode::Rebase => "--rebase",
                }
                .into(),
            );
            if !remote.is_empty() {
                args.push(remote.clone());
            }
            vec![args]
        }
        // `HEAD` and not a name: the branch checked out, to the branch of
        // the same name, whatever a Switch above made that. The upstream is
        // set when it is missing and left alone when it is not.
        Step::Push { remote } => vec![own(&["push", "--set-upstream", remote, "HEAD"])],
        Step::Merge { branch, how } => {
            let mut args = own(&["merge"]);
            match how {
                MergeHow::Commit => args.extend(own(&["--no-ff", "--no-edit"])),
                MergeHow::FastForward => args.extend(own(&["--ff", "--no-edit"])),
                MergeHow::Squash => args.push("--squash".into()),
            }
            args.push(branch.clone());
            vec![args]
        }
        Step::Rebase { onto } => vec![own(&["rebase", onto])],
        Step::CherryPick { commits } => {
            let mut args = own(&["cherry-pick"]);
            args.extend(commits.iter().cloned());
            vec![args]
        }
        Step::Switch { branch } => vec![own(&["switch", branch])],
        Step::NewBranch { name, from } => {
            let mut args = own(&["switch", "-c", name]);
            if !from.is_empty() {
                args.push(from.clone());
            }
            vec![args]
        }
        Step::DeleteBranch { branch } => vec![own(&["branch", "-d", branch])],
        Step::Squash {
            count,
            message: text,
        } => vec![
            own(&["reset", "--soft", &format!("HEAD~{count}")]),
            own(&["commit", "-m", &message(text)]),
        ],
        // `--only` with no path: the message, and nothing that is staged.
        Step::Reword { message: text } => {
            vec![own(&["commit", "--amend", "--only", "-m", &message(text)])]
        }
        Step::Clean {} => vec![own(&["status", "--porcelain"])],
        Step::Check { .. } | Step::Ask {} => Vec::new(),
    }
}

/// An argument as it is shown: quoted when it would not read as one word.
pub fn shown_arg(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=~@+,<>".contains(c));
    if plain {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// A command as it is shown: `git commit -m "Sync with main"`.
pub fn shown_command(args: &[String]) -> String {
    let mut line = String::from("git");
    for arg in args {
        line.push(' ');
        line.push_str(&shown_arg(arg));
    }
    line
}

/// What is shown beside a block: its commands, or the check it runs.
pub fn shown(step: &Step, prune: bool) -> String {
    match step {
        Step::Check { command } => command.clone(),
        Step::Ask {} => String::new(),
        _ => commands(step, prune, None)
            .iter()
            .map(|args| shown_command(args))
            .collect::<Vec<_>>()
            .join(" && "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(step: &Step) -> String {
        shown(step, false)
    }

    #[test]
    fn each_block_is_the_command_the_catalogue_says() {
        let text = |text: &str| Message::text(text);
        for (step, expected) in [
            (Step::Stash { untracked: true }, "git stash push -u"),
            (Step::Stash { untracked: false }, "git stash push"),
            (Step::Pop {}, "git stash pop"),
            (
                Step::Commit {
                    all: true,
                    message: text("Sync with main"),
                },
                "git add -A && git commit -m \"Sync with main\"",
            ),
            (
                Step::Commit {
                    all: false,
                    message: text("Staged"),
                },
                "git commit -m Staged",
            ),
            (
                Step::Amend { message: None },
                "git commit --amend --no-edit",
            ),
            (
                Step::Amend {
                    message: Some(text("Better words")),
                },
                "git commit --amend -m \"Better words\"",
            ),
            (
                Step::Fetch {
                    remote: String::new(),
                },
                "git fetch --all",
            ),
            (
                Step::Fetch {
                    remote: "upstream".into(),
                },
                "git fetch upstream",
            ),
            (
                Step::Pull {
                    remote: "origin".into(),
                    mode: PullMode::FastForwardOnly,
                },
                "git pull --no-edit --ff-only origin",
            ),
            (
                Step::Pull {
                    remote: String::new(),
                    mode: PullMode::Merge,
                },
                "git pull --no-edit --no-rebase",
            ),
            (
                Step::Pull {
                    remote: "origin".into(),
                    mode: PullMode::Rebase,
                },
                "git pull --no-edit --rebase origin",
            ),
            (
                Step::Push {
                    remote: "origin".into(),
                },
                "git push --set-upstream origin HEAD",
            ),
            (
                Step::Merge {
                    branch: "main".into(),
                    how: MergeHow::Commit,
                },
                "git merge --no-ff --no-edit main",
            ),
            (
                Step::Merge {
                    branch: "main".into(),
                    how: MergeHow::FastForward,
                },
                "git merge --ff --no-edit main",
            ),
            (
                Step::Merge {
                    branch: "topic".into(),
                    how: MergeHow::Squash,
                },
                "git merge --squash topic",
            ),
            (
                Step::Rebase {
                    onto: "main".into(),
                },
                "git rebase main",
            ),
            (
                Step::CherryPick {
                    commits: vec!["a3f9c1e".into(), "9d02b7f".into()],
                },
                "git cherry-pick a3f9c1e 9d02b7f",
            ),
            (
                Step::Switch {
                    branch: "main".into(),
                },
                "git switch main",
            ),
            (
                Step::NewBranch {
                    name: "topic".into(),
                    from: String::new(),
                },
                "git switch -c topic",
            ),
            (
                Step::NewBranch {
                    name: "topic".into(),
                    from: "v1.0".into(),
                },
                "git switch -c topic v1.0",
            ),
            (
                Step::DeleteBranch {
                    branch: "topic".into(),
                },
                "git branch -d topic",
            ),
            (
                Step::Squash {
                    count: 3,
                    message: text("One"),
                },
                "git reset --soft HEAD~3 && git commit -m One",
            ),
            (
                Step::Reword {
                    message: text("Say it"),
                },
                "git commit --amend --only -m \"Say it\"",
            ),
            (Step::Clean {}, "git status --porcelain"),
            (
                Step::Check {
                    command: "bun test".into(),
                },
                "bun test",
            ),
            (Step::Ask {}, ""),
        ] {
            assert_eq!(lines(&step), expected, "{step:?}");
        }
    }

    /// No level, no setting and no parameter reaches these.
    #[test]
    fn no_block_runs_a_force_a_hard_reset_a_clean_or_a_capital_d() {
        for step in Step::examples() {
            for prune in [false, true] {
                for args in commands(&step, prune, Some("written")) {
                    let line = shown_command(&args);
                    for never in ["--force", "-f", "--hard", "clean", "-D", "--no-verify"] {
                        assert!(
                            !args.iter().any(|arg| arg == never),
                            "{line} has {never} in it"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fetching_prunes_when_the_setting_says_so() {
        let all = Step::Fetch {
            remote: String::new(),
        };

        assert_eq!(shown(&all, true), "git fetch --prune --all");
        assert_eq!(shown(&all, false), "git fetch --all");
    }

    #[test]
    fn a_message_the_agent_writes_is_marked_until_it_has_been_written() {
        let step = Step::Commit {
            all: true,
            message: Message::Agent,
        };

        assert_eq!(
            shown(&step, false),
            "git add -A && git commit -m \"<agent writes>\""
        );
        assert_eq!(
            commands(&step, false, Some("Cache avatars on disk"))[1],
            ["commit", "-m", "Cache avatars on disk"]
        );
        // A typed message is not replaced by anything an agent wrote.
        let typed = Step::Reword {
            message: Message::text("Mine"),
        };
        assert_eq!(commands(&typed, false, Some("Not mine"))[0][4], "Mine");
    }

    /// A name is one argument, whatever is in it.
    #[test]
    fn a_name_with_a_shells_characters_in_it_is_still_one_argument() {
        let step = Step::Switch {
            branch: "main; rm -rf .".into(),
        };

        let run = commands(&step, false, None);

        assert_eq!(run, [["switch", "main; rm -rf ."]]);
        assert_eq!(shown(&step, false), "git switch \"main; rm -rf .\"");
        assert_eq!(shown_arg("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(shown_arg(""), "\"\"");
    }
}

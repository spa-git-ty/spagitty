// SPDX-License-Identifier: GPL-3.0-or-later

//! An agent builds the blocks from a sentence (FEAT-133).
//!
//! The author's note of 2026-10-13: with the agent's help on, the person
//! writes what they want and the agent proposes the stack. What it proposes
//! lands in the builder, to be read and edited. Nothing it says is run.
//!
//! The agent is shown the whole vocabulary — one example of every block — and
//! the repository's branches, remotes and checks, and is asked for steps in
//! the JSON form recipes are saved in. Its answer is parsed into [`Step`],
//! the same enum a saved recipe is, so a block that is not in the catalogue is
//! not a step and the answer is refused. What parses is then validated like
//! any other stack.

use super::validate::World;
use super::{Step, CATALOGUE, MAX_SENTENCE, MAX_STEPS};

/// What a sentence must be before it is sent: something, on one line, and
/// short. Free text is limited on purpose: the blocks are the vocabulary.
pub fn sentence_ok(sentence: &str) -> Result<&str, String> {
    let sentence = sentence.trim();
    if sentence.is_empty() {
        return Err("Say what the recipe should do.".into());
    }
    let length = sentence.chars().count();
    if length > MAX_SENTENCE {
        return Err(format!(
            "That is {length} characters. The most is {MAX_SENTENCE}."
        ));
    }
    Ok(sentence)
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".into()
    } else {
        items.join(", ")
    }
}

/// What the agent is asked.
pub fn prompt(sentence: &str, world: &World) -> String {
    let mut text = String::new();
    text.push_str(
        "You are helping a person build a Spagitty recipe: an ordered list of git steps made \
only from the fixed blocks below. Spagitty runs the steps itself, later, after the person has \
read them. You run nothing, and you edit nothing.\n\n",
    );
    text.push_str("## The blocks\nThese are all of them. One example of each:\n\n");
    for (block, example) in CATALOGUE.iter().zip(Step::examples()) {
        text.push_str(&format!(
            "- {} — `{}`\n  {}\n",
            block.label,
            block.command,
            serde_json::to_string(&example).expect("a step is JSON")
        ));
    }
    text.push_str(
        "\nParameters:\n\
- `message` is `{\"kind\":\"text\",\"text\":\"…\"}`, one line of at most 72 characters, or \
`{\"kind\":\"agent\"}` to have it written from the changes when the step runs.\n\
- `amend`'s `message` may be `null`, which keeps the message the commit has.\n\
- `pull`'s `mode` is `fastForwardOnly`, `merge` or `rebase`. An empty `remote` is the upstream.\n\
- `merge`'s `how` is `commit` (always a merge commit), `fastForward` or `squash`.\n\
- `fetch` with an empty `remote` fetches every remote.\n\
- `newBranch` with an empty `from` starts from here.\n\
- `pop` pops the newest stash a `stash` above it made.\n\
- `check`'s `command` must be one of the repository's checks, exactly.\n\
There is no block that forces a push, deletes an unmerged branch or resets hard, and none may \
be invented.\n\n",
    );
    text.push_str("## The repository\n");
    text.push_str(&format!(
        "- Checked out: {}\n",
        world.current.as_deref().unwrap_or("(detached)")
    ));
    text.push_str(&format!("- Branches: {}\n", list(&world.branches)));
    text.push_str(&format!("- Remotes: {}\n", list(&world.remotes)));
    text.push_str(&format!("- Checks: {}\n", list(&world.checks)));
    text.push_str(&format!(
        "- Working tree: {}\n\n",
        if world.dirty { "has changes" } else { "clean" }
    ));
    text.push_str(
        "Use only branches and remotes from these lists, or a branch an earlier `newBranch` step \
makes.\n\n",
    );
    text.push_str("## What the person wants\n");
    text.push_str(sentence.trim());
    text.push_str(&format!(
        "\n\n## Your answer\nAnswer with one JSON array of steps, at most {MAX_STEPS}, in a \
```json block, and one short sentence before it saying what the recipe does. If what they want \
cannot be done with these blocks, answer with an empty array and say which part cannot.\n"
    ));
    text
}

/// What an agent proposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub steps: Vec<Step>,
    /// What it said besides the steps, in a line.
    pub said: String,
}

/// The JSON array in `answer`: the one in a fenced block if there is one,
/// and otherwise the outermost `[` to `]`.
fn array_in(answer: &str) -> Option<&str> {
    let fenced = answer.find("```").and_then(|open| {
        let after = &answer[open + 3..];
        let body = after.find('\n').map(|line| &after[line + 1..])?;
        body.find("```").map(|close| &body[..close])
    });
    let within = fenced.unwrap_or(answer);
    let open = within.find('[')?;
    let close = within.rfind(']')?;
    (close > open).then(|| &within[open..=close])
}

/// Read an agent's answer to [`prompt`].
///
/// An answer with a block that is not in the catalogue, or a parameter of the
/// wrong shape, is an error and no steps: half a proposal is not offered.
pub fn read(answer: &str) -> Result<Proposal, String> {
    let array =
        array_in(answer).ok_or_else(|| "The agent did not answer with steps.".to_string())?;
    let steps: Vec<Step> = serde_json::from_str(array)
        .map_err(|error| format!("The agent's steps could not be read: {error}"))?;
    if steps.len() > MAX_STEPS {
        return Err(format!(
            "The agent proposed {} steps. A recipe has at most {MAX_STEPS}.",
            steps.len()
        ));
    }
    let before = answer.find("```").unwrap_or_else(|| {
        answer
            .find('[')
            .expect("an array was found, so there is a bracket")
    });
    let said = answer[..before]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Proposal { steps, said })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::{MergeHow, Message};
    use crate::shell::PullMode;

    fn world() -> World {
        World {
            branches: vec!["main".into(), "feature/avatars".into()],
            remotes: vec!["origin".into()],
            current: Some("feature/avatars".into()),
            checks: vec!["cargo test".into()],
            dirty: true,
            ..World::default()
        }
    }

    #[test]
    fn the_agent_is_shown_every_block_and_only_what_the_repository_has() {
        let asked = prompt("Bring main in and push", &world());

        for block in CATALOGUE {
            assert!(
                asked.contains(&format!("- {} — ", block.label)),
                "{}",
                block.label
            );
            assert!(
                asked.contains(&format!("\"block\":\"{}\"", block.id)),
                "{}",
                block.id
            );
        }
        assert!(asked.contains("- Checked out: feature/avatars"));
        assert!(asked.contains("- Branches: main, feature/avatars"));
        assert!(asked.contains("- Remotes: origin"));
        assert!(asked.contains("- Checks: cargo test"));
        assert!(asked.contains("- Working tree: has changes"));
        assert!(asked.contains("## What the person wants\nBring main in and push"));
        assert!(asked.contains("You run nothing, and you edit nothing."));
        assert!(asked.contains("none may be invented"));
    }

    #[test]
    fn a_repository_with_nothing_to_list_says_none() {
        let asked = prompt("Commit", &World::default());

        assert!(asked.contains("- Checked out: (detached)"));
        assert!(asked.contains("- Branches: (none)"));
        assert!(asked.contains("- Checks: (none)"));
        assert!(asked.contains("- Working tree: clean"));
    }

    #[test]
    fn steps_in_a_fenced_block_are_read_with_what_was_said_before_them() {
        let answer = "This stashes, brings main in and pushes.\n\n```json\n[\n  {\"block\":\"stash\",\"params\":{\"untracked\":true}},\n  {\"block\":\"pull\",\"params\":{\"remote\":\"origin\",\"mode\":\"fastForwardOnly\"}},\n  {\"block\":\"merge\",\"params\":{\"branch\":\"main\",\"how\":\"commit\"}},\n  {\"block\":\"pop\",\"params\":{}},\n  {\"block\":\"commit\",\"params\":{\"all\":true,\"message\":{\"kind\":\"agent\"}}},\n  {\"block\":\"push\",\"params\":{\"remote\":\"origin\"}}\n]\n```\nLet me know if you want changes.";

        let proposal = read(answer).expect("a proposal");

        assert_eq!(proposal.said, "This stashes, brings main in and pushes.");
        assert_eq!(
            proposal.steps,
            [
                Step::Stash { untracked: true },
                Step::Pull {
                    remote: "origin".into(),
                    mode: PullMode::FastForwardOnly
                },
                Step::Merge {
                    branch: "main".into(),
                    how: MergeHow::Commit
                },
                Step::Pop {},
                Step::Commit {
                    all: true,
                    message: Message::Agent
                },
                Step::Push {
                    remote: "origin".into()
                },
            ]
        );
    }

    #[test]
    fn steps_with_no_fence_are_read_too() {
        let proposal = read("Here: [{\"block\":\"ask\",\"params\":{}}]").expect("a proposal");

        assert_eq!(proposal.steps, [Step::Ask {}]);
        assert_eq!(proposal.said, "Here:");
    }

    /// The catalogue is closed to an agent as it is to a person.
    #[test]
    fn a_block_that_is_not_in_the_catalogue_refuses_the_whole_answer() {
        let answer = "```json\n[{\"block\":\"stash\",\"params\":{\"untracked\":false}},{\"block\":\"forcePush\",\"params\":{\"remote\":\"origin\"}}]\n```";

        let refused = read(answer).expect_err("refused");

        assert!(refused.starts_with("The agent's steps could not be read"));
        assert!(refused.contains("forcePush"));
    }

    #[test]
    fn a_parameter_of_the_wrong_shape_refuses_the_answer() {
        for answer in [
            "[{\"block\":\"pull\",\"params\":{\"remote\":\"origin\",\"mode\":\"force\"}}]",
            "[{\"block\":\"squash\",\"params\":{\"count\":\"all\",\"message\":{\"kind\":\"agent\"}}}]",
            "[{\"block\":\"commit\",\"params\":{\"all\":true}}]",
            "[\"git push --force\"]",
        ] {
            assert!(read(answer).is_err(), "{answer}");
        }
    }

    #[test]
    fn an_answer_with_no_steps_in_it_is_said_to_have_none() {
        assert_eq!(
            read("I cannot help with that.").expect_err("no steps"),
            "The agent did not answer with steps."
        );
    }

    #[test]
    fn an_empty_array_is_an_answer_that_nothing_can_be_done() {
        let proposal =
            read("These blocks cannot rewrite another person's branch.\n```json\n[]\n```")
                .expect("an answer");

        assert!(proposal.steps.is_empty());
        assert_eq!(
            proposal.said,
            "These blocks cannot rewrite another person's branch."
        );
    }

    #[test]
    fn more_steps_than_a_recipe_holds_are_refused() {
        let many = format!(
            "[{}]",
            vec!["{\"block\":\"ask\",\"params\":{}}"; MAX_STEPS + 1].join(",")
        );

        assert!(read(&many).expect_err("too many").contains("at most 30"));
    }

    #[test]
    fn a_sentence_is_something_short() {
        assert_eq!(sentence_ok("  Sync and ship  "), Ok("Sync and ship"));
        assert!(sentence_ok("   ").is_err());
        assert!(sentence_ok(&"w".repeat(MAX_SENTENCE)).is_ok());
        assert!(sentence_ok(&"w".repeat(MAX_SENTENCE + 1))
            .expect_err("long")
            .contains("281 characters"));
    }
}

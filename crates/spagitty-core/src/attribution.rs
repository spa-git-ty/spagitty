// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether a commit was written by an agent.
//!
//! One rule, and it is the rule gate 1 already applies to a pull request
//! (`tools/attribution.mjs`, TASK-064 and BUG-078): a `Co-authored-by` trailer
//! that names an agent or a tool, or carries an address only one writes from.
//! Files draws such a line in the agent's colour (FEAT-131), and a line that
//! is called agent-written there must be one the check would have called
//! agent-written too — so this is that table of names, kept in step with it by
//! a test that reads the script.
//!
//! Read from the trailer rather than guessed from prose: a commit that merely
//! mentions Claude in its body is not credited to it. And a person can be
//! called Claude, Cody or Devin, which is why those names count only with what
//! follows them — a model, "Code", "AI".

/// Names only a product has: nobody is called these.
const PRODUCTS: &[&str] = &[
    "openai codex",
    "codex",
    "chatgpt",
    "google gemini",
    "gemini",
    "github copilot",
    "copilot",
    "cursor",
    "aider",
    "windsurf",
    "coderabbit",
    "opencode",
    "agy",
    "antigravity",
    "anthropic",
    "openai",
];

/// A name a person can have, and what has to follow it to make it a product.
const PRODUCTS_BY_WHAT_FOLLOWS: &[(&str, &[&str])] = &[
    (
        "claude",
        &["code", "opus", "sonnet", "haiku", "fable", "instant"],
    ),
    ("devin", &["ai"]),
    ("cody", &["ai"]),
    ("sourcegraph", &["cody"]),
];

/// Addresses only an agent or a tool writes from.
const AGENT_DOMAINS: &[&str] = &[
    "anthropic.com",
    "openai.com",
    "cursor.com",
    "aider.chat",
    "cognition.ai",
    "codeium.com",
    "windsurf.com",
    "coderabbit.ai",
];

/// The agent a commit message credits, by the name its trailer gives.
///
/// The first trailer that names one wins: a commit two agents signed is still
/// one colour in a gutter.
pub fn agent_in(message: &str) -> Option<String> {
    message.lines().find_map(|line| {
        let who = trailer(line)?;
        let (name, address) = split(who);
        is_agent(name, address).then(|| {
            if name.is_empty() {
                address.to_string()
            } else {
                name.to_string()
            }
        })
    })
}

/// Whether a `Co-authored-by` value, `Name <address>`, names an agent or a tool.
pub fn names_an_agent(co_author: &str) -> bool {
    let (name, address) = split(co_author);
    is_agent(name, address)
}

/// The value of a `Co-authored-by:` line, in any case, or nothing.
fn trailer(line: &str) -> Option<&str> {
    const KEY: &str = "co-authored-by";
    let line = line.trim_start();
    let head = line.get(..KEY.len())?;
    if !head.eq_ignore_ascii_case(KEY) {
        return None;
    }
    line[KEY.len()..].trim_start().strip_prefix(':')
}

/// `Name <address>` as its two halves. Either may be empty.
fn split(who: &str) -> (&str, &str) {
    match who.split_once('<') {
        Some((name, rest)) => (
            name.trim(),
            rest.split_once('>')
                .map_or(rest, |(address, _)| address)
                .trim(),
        ),
        None => (who.trim(), ""),
    }
}

fn is_agent(name: &str, address: &str) -> bool {
    let name = name.to_ascii_lowercase();
    is_product(&name) || is_product_by_what_follows(&name) || is_agent_address(address)
}

/// A word ends here: the end, or anything that is not a letter, digit or `_`.
fn ends_word(rest: &str) -> bool {
    !rest
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_product(name: &str) -> bool {
    if PRODUCTS
        .iter()
        .any(|product| name.strip_prefix(product).is_some_and(ends_word))
    {
        return true;
    }
    // `gpt-4o`, `gpt5`: the letters, an optional hyphen, and a digit. Whatever
    // runs on from the digit is the model's name.
    name.strip_prefix("gpt").is_some_and(|rest| {
        rest.strip_prefix('-')
            .unwrap_or(rest)
            .starts_with(|c: char| c.is_ascii_digit())
    })
}

fn is_product_by_what_follows(name: &str) -> bool {
    PRODUCTS_BY_WHAT_FOLLOWS.iter().any(|(first, seconds)| {
        let Some(rest) = name.strip_prefix(first) else {
            return false;
        };
        let after = rest.trim_start();
        // At least one space between the two words.
        if after.len() == rest.len() {
            return false;
        }
        if seconds
            .iter()
            .any(|second| after.strip_prefix(second).is_some_and(ends_word))
        {
            return true;
        }
        // `Claude 3.5`, `Claude 4`: a model number is a model.
        *first == "claude"
            && after
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() && ends_word(&after[1..]))
    })
}

fn is_agent_address(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    if address.contains("[bot]") {
        return true;
    }
    let Some((_, host)) = address.rsplit_once('@') else {
        return false;
    };
    AGENT_DOMAINS.iter().any(|domain| {
        host == *domain
            || host
                .strip_suffix(domain)
                .is_some_and(|sub| sub.ends_with('.') && is_subdomain(&sub[..sub.len() - 1]))
    })
}

/// `mail`, `eu.mail`: labels of letters, digits, `_` and `-`, joined by dots.
fn is_subdomain(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|label| {
            !label.is_empty()
                && label
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trailer_naming_a_product_is_an_agent() {
        for who in [
            "Codex <noreply@example.com>",
            "OpenAI Codex <codex@example.com>",
            "ChatGPT <a@b.c>",
            "GPT-4o <a@b.c>",
            "gpt5 <a@b.c>",
            "Gemini <a@b.c>",
            "Google Gemini <a@b.c>",
            "GitHub Copilot <a@b.c>",
            "Copilot <a@b.c>",
            "Cursor Agent <a@b.c>",
            "aider (gpt-4) <a@b.c>",
            "Windsurf <a@b.c>",
            "CodeRabbit <a@b.c>",
            "opencode <a@b.c>",
            "Antigravity <a@b.c>",
            "Anthropic <a@b.c>",
            "Claude Code <noreply@example.com>",
            "Claude Opus 5.5 <a@b.c>",
            "Claude Sonnet 4 <a@b.c>",
            "Claude 3.5 Sonnet <a@b.c>",
            "Devin AI <a@b.c>",
            "Cody AI <a@b.c>",
            "Sourcegraph Cody <a@b.c>",
        ] {
            assert!(names_an_agent(who), "{who}");
        }
    }

    /// BUG-078: a person can be called any of these.
    #[test]
    fn a_person_who_shares_a_name_with_an_agent_is_a_person() {
        for who in [
            "Claude Shannon <claude@bell-labs.example>",
            "Claude <claude@example.com>",
            "Claude Monet",
            "Devin Booker <devin@example.com>",
            "Cody Rhodes <cody@example.com>",
            "Gemina Okafor <gemina@example.com>",
            "Cursory Reader <reader@example.com>",
            "Codexa Lin <lin@example.com>",
            "Ada Lovelace <ada@example.com>",
            "",
        ] {
            assert!(!names_an_agent(who), "{who}");
        }
    }

    #[test]
    fn an_address_only_an_agent_writes_from_is_enough() {
        for who in [
            "Somebody <noreply@anthropic.com>",
            "Somebody <bot@mail.openai.com>",
            "Somebody <x@cursor.com>",
            "dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
            "<noreply@anthropic.com>",
        ] {
            assert!(names_an_agent(who), "{who}");
        }
        for who in [
            "Somebody <me@notanthropic.com>",
            "Somebody <anthropic.com@example.org>",
            "Somebody <me@openai.com.example.org>",
        ] {
            assert!(!names_an_agent(who), "{who}");
        }
    }

    #[test]
    fn the_agent_is_read_from_the_trailer_and_named_as_it_names_itself() {
        let message = "Cache avatars on disk\n\nNo more memory.\n\nCo-authored-by: Codex <noreply@example.com>\n";
        assert_eq!(agent_in(message).as_deref(), Some("Codex"));

        let upper = "Fix\n\nCO-AUTHORED-BY:   Claude Code <noreply@anthropic.com>";
        assert_eq!(agent_in(upper).as_deref(), Some("Claude Code"));

        let nameless = "Fix\n\nCo-authored-by: <noreply@anthropic.com>";
        assert_eq!(agent_in(nameless).as_deref(), Some("noreply@anthropic.com"));
    }

    #[test]
    fn a_commit_that_only_mentions_an_agent_is_not_credited_to_it() {
        assert_eq!(
            agent_in("Ask Claude Code about this\n\nCodex said so."),
            None
        );
        assert_eq!(
            agent_in("Fix\n\nCo-authored-by: Ada Lovelace <ada@example.com>"),
            None
        );
        assert_eq!(agent_in(""), None);
    }

    #[test]
    fn the_first_agent_named_wins() {
        let message = "Fix\n\nCo-authored-by: Ada <ada@example.com>\nCo-authored-by: Gemini <g@example.com>\nCo-authored-by: Codex <c@example.com>";
        assert_eq!(agent_in(message).as_deref(), Some("Gemini"));
    }

    /// The two tables are one rule written twice. This reads the script gate 1
    /// runs and fails when a name or a domain is in one and not the other.
    #[test]
    fn the_names_are_the_ones_gate_one_checks() {
        let script = include_str!("../../../tools/attribution.mjs");
        for product in PRODUCTS {
            let last = product.rsplit(' ').next().expect("a word");
            assert!(script.contains(last), "{last} is not in attribution.mjs");
        }
        for (first, seconds) in PRODUCTS_BY_WHAT_FOLLOWS {
            assert!(script.contains(first), "{first} is not in attribution.mjs");
            for second in *seconds {
                assert!(
                    script.contains(second),
                    "{second} is not in attribution.mjs"
                );
            }
        }
        for domain in AGENT_DOMAINS {
            let escaped = domain.replace('.', "\\.");
            assert!(
                script.contains(&escaped),
                "{domain} is not in attribution.mjs"
            );
        }
        // And the other way: every product the script names is in the table.
        for word in [
            "codex",
            "chatgpt",
            "gemini",
            "copilot",
            "cursor",
            "aider",
            "windsurf",
            "coderabbit",
            "opencode",
            "agy",
            "antigravity",
            "anthropic",
            "openai",
        ] {
            assert!(script.contains(word));
            assert!(PRODUCTS.contains(&word), "{word} is missing from PRODUCTS");
        }
    }
}

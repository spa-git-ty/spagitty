// SPDX-License-Identifier: GPL-3.0-or-later

//! Secrets out of anything that is kept or shown.
//!
//! A worker's stderr, a tool's output, a provider's message — any of them can
//! echo a credential back, and each of them ends up somewhere a person reads or
//! a diagnostic export carries. Like `spagitty_core::record::redact`, this
//! runs on the way **in**: a stored line that never held the secret cannot
//! leak it later.
//!
//! Recognised, without a regex dependency:
//!
//! - `Bearer <anything>` and `token <anything>` after an authorization header;
//! - well-known token prefixes: GitHub's `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`,
//!   `github_pat_`; GitLab's `glpat-`; CodeRabbit's `cr-` agentic keys;
//! - `--api-key <value>`, `--api-key=<value>`, `api_key=`, `token=`,
//!   `password=`;
//! - the userinfo of a URL: `https://user:secret@host`.

const MARK: &str = "[redacted]";

const PREFIXES: &[(&str, usize)] = &[
    ("github_pat_", 20),
    ("ghp_", 20),
    ("gho_", 20),
    ("ghu_", 20),
    ("ghs_", 20),
    ("ghr_", 20),
    ("glpat-", 16),
    ("cr-", 16),
];

const KEYS: &[&str] = &[
    "api_key=",
    "api-key=",
    "apikey=",
    "token=",
    "password=",
    "secret=",
];

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '~' | '+' | '/' | '=')
}

/// `text` with every recognised secret replaced by `[redacted]`.
pub fn redact(text: &str) -> String {
    let text = redact_userinfo(text);
    let mut out = String::with_capacity(text.len());
    let mut hide_next = false;
    let mut rest = text.as_str();

    while !rest.is_empty() {
        // Split off the next run of token characters, keeping the separator.
        let start = rest.find(is_token_char).unwrap_or(rest.len());
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if rest.is_empty() {
            break;
        }
        let end = rest.find(|c: char| !is_token_char(c)).unwrap_or(rest.len());
        let word = &rest[..end];
        rest = &rest[end..];

        if hide_next {
            out.push_str(MARK);
            hide_next = false;
            continue;
        }

        let lower = word.to_ascii_lowercase();
        if lower == "bearer" || lower == "--api-key" || lower == "private-token" {
            out.push_str(word);
            // The value is the next word, after whatever separates them.
            hide_next = true;
            continue;
        }
        if let Some(key) = KEYS
            .iter()
            .find(|key| lower.starts_with(**key) || lower.starts_with(&format!("--{key}")))
        {
            let cut = lower.find(key).unwrap_or(0) + key.len();
            out.push_str(&word[..cut]);
            if word.len() > cut {
                out.push_str(MARK);
            } else {
                hide_next = true;
            }
            continue;
        }
        if PREFIXES
            .iter()
            .any(|(prefix, min)| word.starts_with(prefix) && word.len() >= prefix.len() + min)
        {
            out.push_str(MARK);
            continue;
        }
        out.push_str(word);
    }
    out
}

/// `scheme://user:secret@host` → `scheme://[redacted]@host`.
fn redact_userinfo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (before, after) = rest.split_at(at + 3);
        out.push_str(before);
        let authority_end = after
            .find(|c: char| c == '/' || c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(after.len());
        let authority = &after[..authority_end];
        match authority.rfind('@') {
            Some(sign) => {
                out.push_str(MARK);
                out.push_str(&authority[sign..]);
            }
            None => out.push_str(authority),
        }
        rest = &after[authority_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_headers_lose_their_value() {
        assert_eq!(
            redact("Authorization: Bearer abc.def-ghi"),
            "Authorization: Bearer [redacted]"
        );
        assert_eq!(redact("PRIVATE-TOKEN: glxyz"), "PRIVATE-TOKEN: [redacted]");
    }

    #[test]
    fn well_known_token_shapes_are_hidden_wherever_they_appear() {
        let text = "push failed for ghp_0123456789abcdefghijABCDEF, key cr-0123456789abcdefXYZ";
        let out = redact(text);
        assert!(!out.contains("ghp_0123"), "{out}");
        assert!(!out.contains("cr-0123"), "{out}");
        assert_eq!(out.matches(MARK).count(), 2);
    }

    #[test]
    fn short_words_that_merely_start_like_a_token_are_left_alone() {
        assert_eq!(
            redact("the cr-lf setting and ghp_ prefix"),
            "the cr-lf setting and ghp_ prefix"
        );
    }

    #[test]
    fn api_keys_on_a_command_line_are_hidden_in_both_spellings() {
        assert_eq!(
            redact("cr auth login --api-key secretvalue"),
            "cr auth login --api-key [redacted]"
        );
        assert_eq!(
            redact("cr --api-key=secretvalue"),
            "cr --api-key=[redacted]"
        );
        assert_eq!(redact("url?token=abc&x=1"), "url?token=[redacted]&x=1");
    }

    #[test]
    fn url_userinfo_is_removed_and_the_host_kept() {
        assert_eq!(
            redact("cloning https://me:hunter2@github.com/o/r.git now"),
            "cloning https://[redacted]@github.com/o/r.git now"
        );
        assert_eq!(
            redact("see https://github.com/o/r"),
            "see https://github.com/o/r"
        );
    }

    #[test]
    fn ordinary_text_is_unchanged() {
        let text = "Reviewing 12 files against main (3 findings)\n";
        assert_eq!(redact(text), text);
    }
}

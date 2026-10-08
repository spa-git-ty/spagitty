// SPDX-License-Identifier: GPL-3.0-or-later

//! A repository's rules for agents.
//!
//! Which agents exist is a fact about the machine; how far they may go in one
//! repository is a fact about that repository. These are the second kind:
//! kept per repository on this machine, changed in Settings › Agents, and read
//! by the engine before every gate rather than trusted to the agent.

use serde::{Deserialize, Serialize};

use super::level::Level;

/// One repository's rules. Every field has a default, so a repository nobody
/// has configured is the safe one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RepoRules {
    /// Agents that may be used here, by id. `None` is all of them.
    pub agents: Option<Vec<String>>,
    /// Providers this repository's code may be sent to, by slug — asked once,
    /// at the first assignment of a remote agent here.
    pub consent: Vec<String>,
    /// The highest level allowed here. *Unattended* is not offered anywhere
    /// until a person raises this.
    pub highest: Level,
    /// Branches an agent may never land into unattended. `*` matches within a
    /// path segment's worth of characters, so `release/*` covers
    /// `release/2.0`.
    pub never_unattended: Vec<String>,
    /// Whether an agent may give a verdict on the host — Approve or Request
    /// changes — rather than only comment.
    pub verdicts: bool,
    /// Whether comments an agent drafted say so on the host.
    pub mark_comments: bool,
}

impl Default for RepoRules {
    fn default() -> Self {
        RepoRules {
            agents: None,
            consent: Vec::new(),
            highest: Level::SignOff,
            never_unattended: vec!["main".into(), "master".into()],
            verdicts: false,
            mark_comments: true,
        }
    }
}

impl RepoRules {
    /// The level an assignment actually runs at: the one asked for, capped.
    pub fn cap(&self, asked: Level) -> Level {
        asked.min(self.highest)
    }

    /// May this agent be used here?
    pub fn allows(&self, agent: &str) -> bool {
        match &self.agents {
            None => true,
            Some(list) => list.iter().any(|id| id == agent),
        }
    }

    /// Has the person agreed to send this repository's code to `provider`?
    pub fn consented(&self, provider: &str) -> bool {
        self.consent.iter().any(|slug| slug == provider)
    }

    /// May an agent land into `branch` without the person?
    pub fn may_land_unattended(&self, branch: &str) -> bool {
        !self
            .never_unattended
            .iter()
            .any(|pattern| glob(pattern, branch))
    }
}

/// `*` matches any run of characters, anything else matches itself.
///
/// Deliberately small: branch patterns people write are `main` and
/// `release/*`, and a full glob language would be a second thing to learn for
/// a list of three entries.
pub fn glob(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while t < text.len() {
        if p < pattern.len() && pattern[p] != '*' && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if let Some(at) = star {
            p = at + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_nobody_configured_is_the_safe_one() {
        let rules = RepoRules::default();
        assert_eq!(rules.highest, Level::SignOff);
        assert!(!rules.verdicts);
        assert!(rules.mark_comments);
        assert!(rules.consent.is_empty());
        assert!(!rules.may_land_unattended("main"));
    }

    #[test]
    fn the_level_is_capped_at_the_repositorys_highest() {
        let rules = RepoRules {
            highest: Level::SignOff,
            ..RepoRules::default()
        };
        assert_eq!(rules.cap(Level::Unattended), Level::SignOff);
        assert_eq!(rules.cap(Level::Suggest), Level::Suggest);
    }

    #[test]
    fn an_agent_list_limits_who_may_work_here() {
        let rules = RepoRules {
            agents: Some(vec!["claude".into()]),
            ..RepoRules::default()
        };
        assert!(rules.allows("claude"));
        assert!(!rules.allows("codex"));
        assert!(RepoRules::default().allows("anything"));
    }

    #[test]
    fn branch_patterns_match_as_people_write_them() {
        assert!(glob("main", "main"));
        assert!(!glob("main", "maintenance"));
        assert!(glob("release/*", "release/2.0"));
        assert!(!glob("release/*", "feat/release"));
        assert!(glob("*", "anything"));
        assert!(glob("feat/*-wip", "feat/tabs-wip"));
        assert!(!glob("feat/*-wip", "feat/tabs"));
    }

    #[test]
    fn a_never_unattended_branch_refuses_an_unattended_land() {
        let rules = RepoRules {
            never_unattended: vec!["main".into(), "release/*".into()],
            ..RepoRules::default()
        };
        assert!(!rules.may_land_unattended("release/2.0"));
        assert!(rules.may_land_unattended("feat/tab-drag"));
    }

    #[test]
    fn missing_fields_read_as_their_defaults() {
        let rules: RepoRules = serde_json::from_str(r#"{"verdicts": true}"#).unwrap();
        assert!(rules.verdicts);
        assert_eq!(rules.highest, Level::SignOff);
        assert!(rules.mark_comments);
    }
}

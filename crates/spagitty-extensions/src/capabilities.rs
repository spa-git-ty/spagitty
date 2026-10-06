// SPDX-License-Identifier: GPL-3.0-or-later

//! What an extension may ask the host to do for it.
//!
//! A closed set. A manifest naming anything else is refused rather than
//! ignored, because an extension that asked for a capability this build does
//! not know was written against a different host and would fail later, in a
//! way that is harder to explain.
//!
//! **A capability limits Spagitty's API, not the program.** It decides whether
//! the host answers `forge.pullRequest.comment`; it does not stop a native
//! worker from opening its own connection. The interface says so wherever a
//! capability is granted.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Capability {
    /// A repository's description and the files a snapshot covers.
    #[serde(rename = "repository.read")]
    RepositoryRead,
    /// Publishing review progress, findings and results for host-issued ids.
    #[serde(rename = "review.provide")]
    ReviewProvide,
    /// Running a declared external tool through a declared profile.
    #[serde(rename = "tools.execute")]
    ToolsExecute,
    /// A pull request's metadata, discussion, reviews and checks.
    #[serde(rename = "forge.pullRequest.read")]
    ForgePullRequestRead,
    /// Posting an exact, previewed comment on a pull request.
    #[serde(rename = "forge.pullRequest.comment")]
    ForgePullRequestComment,
}

impl Capability {
    pub const ALL: [Capability; 5] = [
        Capability::RepositoryRead,
        Capability::ReviewProvide,
        Capability::ToolsExecute,
        Capability::ForgePullRequestRead,
        Capability::ForgePullRequestComment,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Capability::RepositoryRead => "repository.read",
            Capability::ReviewProvide => "review.provide",
            Capability::ToolsExecute => "tools.execute",
            Capability::ForgePullRequestRead => "forge.pullRequest.read",
            Capability::ForgePullRequestComment => "forge.pullRequest.comment",
        }
    }

    pub fn parse(text: &str) -> Option<Capability> {
        Capability::ALL
            .into_iter()
            .find(|capability| capability.as_str() == text)
    }

    /// What granting it means, in one line for the Settings screen.
    pub fn describe(self) -> &'static str {
        match self {
            Capability::RepositoryRead => "Read this repository's branches and changed files",
            Capability::ReviewProvide => "Show review findings in Spagitty",
            Capability::ToolsExecute => "Run the external tools it declares",
            Capability::ForgePullRequestRead => {
                "Read pull request discussion and checks with your connected account"
            }
            Capability::ForgePullRequestComment => {
                "Post comments you approve on pull requests with your connected account"
            }
        }
    }

    /// Whether this capability's grant is per repository. Every one is: a
    /// grant for one repository says nothing about another.
    pub fn per_repository(self) -> bool {
        true
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capability_round_trips_through_its_name() {
        for capability in Capability::ALL {
            assert_eq!(Capability::parse(capability.as_str()), Some(capability));
            let json = serde_json::to_string(&capability).unwrap();
            assert_eq!(json, format!("\"{}\"", capability.as_str()));
            assert!(!capability.describe().is_empty());
            assert!(capability.per_repository());
        }
    }

    #[test]
    fn a_name_this_host_does_not_know_is_not_a_capability() {
        assert_eq!(Capability::parse("filesystem.write"), None);
        assert_eq!(Capability::parse("Repository.Read"), None);
        assert!(serde_json::from_str::<Capability>("\"network\"").is_err());
    }
}

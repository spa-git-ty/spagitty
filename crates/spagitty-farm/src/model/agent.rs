// SPDX-License-Identifier: GPL-3.0-or-later

//! What an agent *is* to the farm.
//!
//! The plan is explicit that an agent must not be represented only by the path
//! to its executable, and the reason shows up the first time two of them are
//! asked to do the same job: routing, prompt shape, cancellation and result
//! parsing all differ per provider, and a `PathBuf` carries none of it.
//!
//! So an agent is a definition — provider, executable, capabilities, config —
//! and the provider-specific behaviour lives behind
//! [`crate::agent::AgentAdapter`], never in the engine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::AgentId;

/// Which coding agent this is.
///
/// A closed set plus [`AgentProvider::Custom`], rather than an open string.
/// The four named ones each get an adapter with real knowledge of their command
/// line; `Custom` is the escape hatch for a binary Spagitty has never heard of,
/// driven by a command template the user supplies. Without `Custom` every new
/// tool would need a release; without the closed set the engine would have to
/// guess how to invoke things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentProvider {
    ClaudeCode,
    Codex,
    Cursor,
    OhMyPi,
    Custom,
}

impl AgentProvider {
    /// Every provider with a built-in adapter, in the order the settings screen
    /// lists them.
    pub const BUILT_IN: [AgentProvider; 4] = [
        AgentProvider::ClaudeCode,
        AgentProvider::Codex,
        AgentProvider::Cursor,
        AgentProvider::OhMyPi,
    ];

    /// The name a person would use.
    pub fn label(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "Claude Code",
            AgentProvider::Codex => "Codex",
            AgentProvider::Cursor => "Cursor",
            AgentProvider::OhMyPi => "Oh My Pi",
            AgentProvider::Custom => "Custom",
        }
    }

    /// The stable slug used in branch names, directory names and identifiers.
    ///
    /// Lower-case and hyphen-free on purpose: it ends up inside a git ref, and
    /// a ref component that needs quoting is a ref that breaks somebody's
    /// script.
    pub fn slug(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "claude",
            AgentProvider::Codex => "codex",
            AgentProvider::Cursor => "cursor",
            AgentProvider::OhMyPi => "pi",
            AgentProvider::Custom => "custom",
        }
    }
}

/// One thing an agent is considered good at.
///
/// Capabilities are what routing reads. They are declared rather than measured
/// — Spagitty cannot know whether a binary is good at frontend work — and the
/// defaults per provider are a starting point the user is expected to edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentCapability {
    Planning,
    Coding,
    Review,
    Testing,
    Frontend,
    Backend,
    Documentation,
    Research,
    LongContext,
    Vision,
    ToolUse,
}

impl AgentCapability {
    pub const ALL: [AgentCapability; 11] = [
        AgentCapability::Planning,
        AgentCapability::Coding,
        AgentCapability::Review,
        AgentCapability::Testing,
        AgentCapability::Frontend,
        AgentCapability::Backend,
        AgentCapability::Documentation,
        AgentCapability::Research,
        AgentCapability::LongContext,
        AgentCapability::Vision,
        AgentCapability::ToolUse,
    ];
}

/// The set an agent declares.
///
/// A `BTreeSet` rather than a `Vec`, because the settings screen writes it back
/// as a list of toggles and a list would let the same capability appear twice —
/// which would then count twice in routing.
pub type AgentCapabilities = BTreeSet<AgentCapability>;

/// How the farm talks to this provider.
///
/// The plan's rule is that adapters normalise capabilities but must never
/// pretend every agent works the same way. This enum is that rule made
/// explicit: it says how a prompt reaches the process, and the runner branches
/// on it rather than assuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentInputMode {
    /// The prompt is an argument on the command line.
    CliPrompt,
    /// The prompt is written to the process's standard input.
    Stdin,
    /// The agent client protocol. Not driven in this release; recorded so a
    /// definition that says so round-trips rather than being rewritten.
    Acp,
    /// A model context protocol server. Same treatment as `Acp`.
    Mcp,
    /// An HTTP API rather than a local process. Same treatment.
    Api,
}

/// What the farm may rely on this provider to do.
///
/// Separate from [`AgentCapability`], which is about subject matter. These are
/// mechanical: whether output can be streamed, whether a session can be
/// resumed, whether the thing can run without a terminal at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraits {
    pub resumable_sessions: bool,
    pub streaming: bool,
    pub structured_output: bool,
    pub tool_use: bool,
    /// Runs to completion with no terminal attached. An agent that cannot is
    /// refused before it is started rather than hanging on a prompt nobody can
    /// see — the same failure `GIT_TERMINAL_PROMPT=0` exists to prevent.
    pub headless: bool,
}

/// A registered agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub id: AgentId,
    pub provider: AgentProvider,
    pub display_name: String,
    /// Absolute path when detection found one; the bare command name otherwise,
    /// which is what a user typing `codex` into the settings screen means.
    pub executable: PathBuf,
    pub capabilities: AgentCapabilities,
    pub input_mode: AgentInputMode,
    pub traits: AgentTraits,
    /// The role this agent is *preferred* for. Routing is a preference, not a
    /// constraint: an unfilled role falls back to any capable agent rather than
    /// stalling the farm.
    pub role: AgentRole,
    /// Extra arguments the user wants on every invocation, and — for
    /// [`AgentProvider::Custom`] — the argument template itself.
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// False when the user has switched this agent off without deleting it.
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

/// The job an agent is preferred for.
///
/// This is the whole of V1 routing, and the plan says so: rule-based, not
/// scored. `score(agent, task)` arrives once there is history to score with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRole {
    Architect,
    Backend,
    Frontend,
    Reviewer,
    Tester,
    Researcher,
    #[default]
    General,
}

impl AgentRole {
    pub const ALL: [AgentRole; 7] = [
        AgentRole::Architect,
        AgentRole::Backend,
        AgentRole::Frontend,
        AgentRole::Reviewer,
        AgentRole::Tester,
        AgentRole::Researcher,
        AgentRole::General,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AgentRole::Architect => "Architect",
            AgentRole::Backend => "Backend",
            AgentRole::Frontend => "Frontend",
            AgentRole::Reviewer => "Reviewer",
            AgentRole::Tester => "Tester",
            AgentRole::Researcher => "Researcher",
            AgentRole::General => "General",
        }
    }
}

/// What detection found.
///
/// `Missing` is not an error. A machine with two of the four agents installed
/// is the normal case, and the settings screen shows all four so the user knows
/// what they could add.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum AgentAvailability {
    /// Found, and it answered `--version`.
    Available {
        path: PathBuf,
        version: String,
    },
    /// Found on disk, but running it did not work. Kept apart from `Missing`
    /// because the fix is different: this one is installed and broken.
    Broken {
        path: PathBuf,
        reason: String,
    },
    Missing,
}

impl AgentAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, AgentAvailability::Available { .. })
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            AgentAvailability::Available { path, .. } | AgentAvailability::Broken { path, .. } => {
                Some(path)
            }
            AgentAvailability::Missing => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_slugs_are_safe_inside_a_git_ref() {
        for provider in [
            AgentProvider::ClaudeCode,
            AgentProvider::Codex,
            AgentProvider::Cursor,
            AgentProvider::OhMyPi,
            AgentProvider::Custom,
        ] {
            let slug = provider.slug();
            assert!(!slug.is_empty());
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "{slug} would need quoting in a ref"
            );
        }
    }

    #[test]
    fn slugs_are_unique_so_two_agents_cannot_share_a_branch() {
        let slugs: BTreeSet<_> = AgentProvider::BUILT_IN.iter().map(|p| p.slug()).collect();
        assert_eq!(slugs.len(), AgentProvider::BUILT_IN.len());
    }

    #[test]
    fn availability_reports_a_path_for_broken_as_well_as_available() {
        let broken = AgentAvailability::Broken {
            path: PathBuf::from("/usr/bin/codex"),
            reason: "exited 127".into(),
        };
        assert!(!broken.is_available());
        assert_eq!(broken.path(), Some(&PathBuf::from("/usr/bin/codex")));
        assert_eq!(AgentAvailability::Missing.path(), None);
    }

    #[test]
    fn a_definition_written_without_enabled_reads_as_enabled() {
        // Settings files predate every field they gain. A definition saved
        // before the switch existed must not come back switched off.
        let json = r#"{
            "id": "claude",
            "provider": "claudeCode",
            "displayName": "Claude Code",
            "executable": "/usr/bin/claude",
            "capabilities": ["coding"],
            "inputMode": "cliPrompt",
            "traits": {
                "resumableSessions": true, "streaming": true,
                "structuredOutput": false, "toolUse": true, "headless": true
            },
            "role": "architect"
        }"#;
        let definition: AgentDefinition = serde_json::from_str(json).unwrap();
        assert!(definition.enabled);
        assert!(definition.extra_args.is_empty());
    }

    #[test]
    fn every_capability_is_listed_in_all() {
        // ALL drives the settings screen's checkbox list. A capability added to
        // the enum and forgotten here would be invisible in the interface.
        assert_eq!(AgentCapability::ALL.len(), 11);
        let unique: BTreeSet<_> = AgentCapability::ALL.iter().collect();
        assert_eq!(unique.len(), AgentCapability::ALL.len());
    }

    #[test]
    fn every_role_is_listed_in_all() {
        let unique: BTreeSet<_> = AgentRole::ALL.iter().collect();
        assert_eq!(unique.len(), AgentRole::ALL.len());
        assert_eq!(AgentRole::default(), AgentRole::General);
    }
}

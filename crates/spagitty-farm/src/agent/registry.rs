// SPDX-License-Identifier: GPL-3.0-or-later

//! Which agents this machine has, and what the user has said about them.
//!
//! The registry is the join between two things that must not be confused:
//!
//! * **Detection** — a fact about the machine, re-read whenever the settings
//!   screen is opened. Nobody edits it.
//! * **Definitions** — the user's configuration: capabilities, role, extra
//!   arguments, whether the agent is switched on. Edited constantly, and
//!   persisted.
//!
//! Keeping them apart is what lets an agent be configured before it is
//! installed, and lets a configuration survive a machine where the executable
//! has moved. A definition whose executable is missing is *disabled by
//! circumstance*, not deleted — deleting it would throw away a capability list
//! somebody spent time on because they were on the wrong laptop that morning.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::agent::adapter::AgentAdapter;
use crate::agent::adapters::{
    agy::AgyAdapter, claude::ClaudeAdapter, codex::CodexAdapter, cursor::CursorAdapter,
    custom::CustomAdapter, pi::PiAdapter,
};
use crate::error::{Error, Result};
use crate::model::{
    AgentAvailability, AgentCapability, AgentDefinition, AgentId, AgentProvider, AgentRole,
};

/// The adapter for one provider.
///
/// A function rather than a map, because the set is closed and a match is the
/// thing the compiler checks when a provider is added. A `HashMap` built at
/// startup would let a new provider compile with no adapter and fail at run
/// time in whatever screen reached it first.
pub fn adapter_for(provider: AgentProvider) -> &'static dyn AgentAdapter {
    match provider {
        AgentProvider::ClaudeCode => &ClaudeAdapter,
        AgentProvider::Codex => &CodexAdapter,
        AgentProvider::Cursor => &CursorAdapter,
        AgentProvider::OhMyPi => &PiAdapter,
        AgentProvider::Agy => &AgyAdapter,
        AgentProvider::Custom => &CustomAdapter,
    }
}

/// One row of the settings screen: what is configured, and what is true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub definition: AgentDefinition,
    pub availability: AgentAvailability,
}

impl AgentStatus {
    /// Can this agent be given a task right now?
    ///
    /// Both halves have to be true: switched on by the user, and present on the
    /// machine. The two are reported separately in the interface so the reason
    /// it cannot run is visible.
    pub fn is_usable(&self) -> bool {
        self.definition.enabled && self.availability.is_available()
    }
}

/// The configured agents, and the last detection result for each.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRegistry {
    /// Keyed by identifier and ordered, so the settings screen is stable
    /// between openings and a diff of the saved file is readable.
    definitions: BTreeMap<AgentId, AgentDefinition>,
    /// Not serialised: a fact about the machine, re-derived on load. Persisting
    /// it would mean showing a stale "detected" for an agent uninstalled since.
    #[serde(skip)]
    availability: BTreeMap<AgentId, AgentAvailability>,
}

impl AgentRegistry {
    /// Look for every built-in provider, adding a default definition for each
    /// one found that is not configured already.
    ///
    /// Existing definitions are *updated*, not replaced: the executable path is
    /// refreshed from detection — an agent that moved between releases should
    /// keep working — and everything the user chose is left alone.
    pub fn detect_all(&mut self) {
        for provider in AgentProvider::BUILT_IN {
            let adapter = adapter_for(provider);
            let found = adapter.detect();
            let id = adapter
                .default_definition(found.path().cloned().unwrap_or_default())
                .id;

            if let Some(path) = found.path() {
                match self.definitions.get_mut(&id) {
                    Some(existing) => existing.executable = path.clone(),
                    None => {
                        let definition = adapter.default_definition(path.clone());
                        self.definitions.insert(id.clone(), definition);
                    }
                }
            }
            self.availability.insert(id, found);
        }

        // A custom agent is never detected, but it still has to be probed:
        // the user typed a path, and whether it is there is worth knowing.
        let custom: Vec<AgentId> = self
            .definitions
            .values()
            .filter(|definition| definition.provider == AgentProvider::Custom)
            .map(|definition| definition.id.clone())
            .collect();
        for id in custom {
            let executable = self.definitions[&id].executable.clone();
            self.availability
                .insert(id, CustomAdapter.probe(&executable));
        }
    }

    /// Every configured agent with its availability, in identifier order.
    pub fn statuses(&self) -> Vec<AgentStatus> {
        self.definitions
            .values()
            .map(|definition| AgentStatus {
                definition: definition.clone(),
                availability: self
                    .availability
                    .get(&definition.id)
                    .cloned()
                    .unwrap_or(AgentAvailability::Missing),
            })
            .collect()
    }

    /// The providers with no definition, so the settings screen can offer them.
    ///
    /// Answering "what could I add" is the other half of the registry's job.
    /// Without it a machine with nothing installed shows an empty screen and no
    /// hint that four things would work if they were there.
    pub fn undetected(&self) -> Vec<AgentProvider> {
        AgentProvider::BUILT_IN
            .into_iter()
            .filter(|provider| {
                !self
                    .definitions
                    .values()
                    .any(|definition| definition.provider == *provider)
            })
            .collect()
    }

    pub fn get(&self, id: &AgentId) -> Option<&AgentDefinition> {
        self.definitions.get(id)
    }

    /// Add or replace a definition.
    ///
    /// Identifier collisions are the caller's problem, and deliberately so: a
    /// custom agent named the same as an existing one *should* overwrite it,
    /// because that is what saving an edited row means.
    pub fn put(&mut self, definition: AgentDefinition) {
        self.definitions.insert(definition.id.clone(), definition);
    }

    /// Forget a definition. Detection may put a built-in one straight back,
    /// which is correct: the machine still has it.
    pub fn remove(&mut self, id: &AgentId) {
        self.definitions.remove(id);
        self.availability.remove(id);
    }

    /// Every agent that could be given a task right now.
    pub fn usable(&self) -> Vec<AgentDefinition> {
        self.statuses()
            .into_iter()
            .filter(AgentStatus::is_usable)
            .map(|status| status.definition)
            .collect()
    }

    /// Resolve an identifier to something runnable, or say why not.
    ///
    /// Three distinct failures with three distinct messages, because "could not
    /// run codex" tells a user nothing about whether to install it, switch it
    /// on, or fix their `PATH`.
    pub fn runnable(&self, id: &AgentId) -> Result<AgentDefinition> {
        let definition = self
            .definitions
            .get(id)
            .ok_or_else(|| Error::NoSuchAgent(id.clone()))?;
        if !definition.enabled {
            return Err(Error::Refused(format!(
                "{} is switched off in Farm settings",
                definition.display_name
            )));
        }
        match self.availability.get(id) {
            Some(found) if found.is_available() => Ok(definition.clone()),
            Some(AgentAvailability::Broken { reason, .. }) => Err(Error::AgentUnavailable(
                format!("{} ({reason})", definition.display_name),
            )),
            _ => Err(Error::AgentUnavailable(definition.display_name.clone())),
        }
    }

    /// The best agent for a role, or nothing.
    ///
    /// Preference first, capability second, anything usable third. The
    /// three-step fallback is what stops a farm stalling because the machine has
    /// no agent whose *role* field says "Frontend" — a coding agent is still a
    /// coding agent.
    pub fn for_role(
        &self,
        role: AgentRole,
        capability: AgentCapability,
    ) -> Option<AgentDefinition> {
        let usable = self.usable();
        usable
            .iter()
            .find(|definition| definition.role == role)
            .or_else(|| {
                usable
                    .iter()
                    .find(|definition| definition.capabilities.contains(&capability))
            })
            .or_else(|| usable.first())
            .cloned()
    }

    /// Anyone usable other than `avoid`.
    ///
    /// The self-review rule made available to routing: a reviewer is picked
    /// from everybody except the agent that wrote the change. Returns nothing
    /// when there is only one agent, which is the case where review has to be
    /// the human's job.
    pub fn other_than(
        &self,
        avoid: &AgentId,
        capability: AgentCapability,
    ) -> Option<AgentDefinition> {
        let usable = self.usable();
        let others: Vec<_> = usable
            .into_iter()
            .filter(|definition| &definition.id != avoid)
            .collect();
        others
            .iter()
            .find(|definition| definition.capabilities.contains(&capability))
            .or_else(|| others.first())
            .cloned()
    }

    /// Probe one agent, leaving the others as they were.
    ///
    /// Saving an edited row should not re-run every agent CLI on the machine:
    /// a provider that takes a second to answer `--version` — a Node CLI
    /// resolving its dependency tree, a shim from a version manager — turns a
    /// settings edit into a visible pause for no reason. Detection of the whole
    /// set is a deliberate act, and it has its own button.
    pub fn probe(&mut self, id: &AgentId) {
        let Some(definition) = self.definitions.get(id) else {
            return;
        };
        let found = adapter_for(definition.provider).probe(&definition.executable);
        self.availability.insert(id.clone(), found);
    }

    /// Mark availability directly.
    ///
    /// Used by [`Self::detect_all`], and by the tests, which must not depend on
    /// which agents happen to be installed on the machine running them.
    pub fn set_availability(&mut self, id: AgentId, availability: AgentAvailability) {
        self.availability.insert(id, availability);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn available(registry: &mut AgentRegistry, definition: AgentDefinition) {
        let id = definition.id.clone();
        registry.put(definition);
        registry.set_availability(
            id,
            AgentAvailability::Available {
                path: PathBuf::from("/usr/bin/x"),
                version: "1".into(),
            },
        );
    }

    fn registry() -> AgentRegistry {
        let mut registry = AgentRegistry::default();
        available(
            &mut registry,
            ClaudeAdapter.default_definition(PathBuf::from("/usr/bin/claude")),
        );
        available(
            &mut registry,
            CodexAdapter.default_definition(PathBuf::from("/usr/bin/codex")),
        );
        registry
    }

    #[test]
    fn every_provider_has_an_adapter() {
        for provider in [
            AgentProvider::ClaudeCode,
            AgentProvider::Codex,
            AgentProvider::Cursor,
            AgentProvider::OhMyPi,
            AgentProvider::Agy,
            AgentProvider::Custom,
        ] {
            assert_eq!(adapter_for(provider).provider(), provider);
        }
    }

    #[test]
    fn a_configured_agent_that_is_not_installed_cannot_run() {
        let mut registry = AgentRegistry::default();
        registry.put(ClaudeAdapter.default_definition(PathBuf::from("/usr/bin/claude")));
        let error = registry.runnable(&AgentId::new("claude")).unwrap_err();
        assert_eq!(error.kind(), "agentUnavailable");
        assert!(registry.usable().is_empty());
    }

    #[test]
    fn a_broken_agent_says_what_went_wrong() {
        let mut registry = AgentRegistry::default();
        registry.put(CodexAdapter.default_definition(PathBuf::from("/usr/bin/codex")));
        registry.set_availability(
            AgentId::new("codex"),
            AgentAvailability::Broken {
                path: PathBuf::from("/usr/bin/codex"),
                reason: "exited with 127".into(),
            },
        );
        let error = registry.runnable(&AgentId::new("codex")).unwrap_err();
        assert!(error.to_string().contains("127"), "{error}");
    }

    #[test]
    fn switching_an_agent_off_is_a_different_refusal_from_not_having_it() {
        let mut registry = registry();
        let mut definition = registry.get(&AgentId::new("claude")).unwrap().clone();
        definition.enabled = false;
        registry.put(definition);
        let error = registry.runnable(&AgentId::new("claude")).unwrap_err();
        assert_eq!(error.kind(), "refused");
        assert!(error.to_string().contains("switched off"));
    }

    #[test]
    fn an_unknown_identifier_is_its_own_failure() {
        assert_eq!(
            registry()
                .runnable(&AgentId::new("nope"))
                .unwrap_err()
                .kind(),
            "noSuchAgent"
        );
    }

    #[test]
    fn routing_prefers_the_declared_role() {
        let picked = registry()
            .for_role(AgentRole::Backend, AgentCapability::Coding)
            .unwrap();
        assert_eq!(picked.id, AgentId::new("codex"));
    }

    #[test]
    fn routing_falls_back_to_capability_when_no_agent_holds_the_role() {
        // Nothing here is a Frontend agent; something can still write code.
        let picked = registry()
            .for_role(AgentRole::Frontend, AgentCapability::Coding)
            .unwrap();
        assert!(picked.capabilities.contains(&AgentCapability::Coding));
    }

    #[test]
    fn routing_falls_back_to_anyone_rather_than_stalling() {
        let picked = registry()
            .for_role(AgentRole::Tester, AgentCapability::Vision)
            .unwrap();
        assert!(matches!(picked.id.as_str(), "claude" | "codex"));
    }

    #[test]
    fn a_reviewer_is_never_the_agent_that_wrote_the_change() {
        let reviewer = registry()
            .other_than(&AgentId::new("codex"), AgentCapability::Review)
            .unwrap();
        assert_eq!(reviewer.id, AgentId::new("claude"));
    }

    #[test]
    fn a_farm_with_one_agent_has_no_reviewer_to_offer() {
        let mut registry = AgentRegistry::default();
        available(
            &mut registry,
            ClaudeAdapter.default_definition(PathBuf::from("/usr/bin/claude")),
        );
        assert!(registry
            .other_than(&AgentId::new("claude"), AgentCapability::Review)
            .is_none());
    }

    #[test]
    fn undetected_providers_are_offered() {
        let registry = registry();
        let missing = registry.undetected();
        assert!(missing.contains(&AgentProvider::Cursor));
        assert!(missing.contains(&AgentProvider::OhMyPi));
        assert!(!missing.contains(&AgentProvider::ClaudeCode));
    }

    #[test]
    fn detection_never_persists_availability() {
        // Round-tripping a registry must not carry "detected" onto a machine
        // where the agent is not installed.
        let json = serde_json::to_string(&registry()).unwrap();
        assert!(!json.contains("available"));
        let back: AgentRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.statuses().len(), 2);
        assert!(back.usable().is_empty());
    }

    #[test]
    fn removing_a_definition_removes_its_availability_too() {
        let mut registry = registry();
        registry.remove(&AgentId::new("claude"));
        assert!(registry.get(&AgentId::new("claude")).is_none());
        assert_eq!(registry.statuses().len(), 1);
    }

    #[test]
    fn a_status_needs_both_halves_to_be_usable() {
        let mut status = AgentStatus {
            definition: ClaudeAdapter.default_definition(PathBuf::from("/x")),
            availability: AgentAvailability::Missing,
        };
        assert!(!status.is_usable());
        status.availability = AgentAvailability::Available {
            path: PathBuf::from("/x"),
            version: "1".into(),
        };
        assert!(status.is_usable());
        status.definition.enabled = false;
        assert!(!status.is_usable());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! What the user decided, kept where a repository cannot reach it.
//!
//! **User state** — `state.json` in the application's data directory — holds
//! everything that is a permission: which extensions are installed and at
//! which version, which repositories each is enabled for, which capabilities
//! were granted (keyed by extension, version, capability and repository),
//! consent to send code to a service, the executables the user chose, and
//! user-scoped settings. A file committed to a repository can grant nothing,
//! because nothing here is read from a repository.
//!
//! **Repository state** — `.spagitty/extensions/` in the main checkout,
//! excluded from git the way the farm's directory is — holds what belongs to
//! the code: repository-scoped settings and review history (see
//! [`crate::history`]).
//!
//! Every write is a whole-file write to a temporary name and a rename, so a
//! crash mid-write leaves the previous file rather than half of a new one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::capabilities::Capability;
use crate::{Error, Result};

pub const STATE_FILE: &str = "state.json";
pub const REPOSITORY_DIR: &str = ".spagitty";

/// Write `bytes` to `path` by writing a sibling and renaming it over.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Io(format!("{} has no parent directory", path.display())))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("state"),
        std::process::id()
    ));
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        Error::Io(format!("could not save {}: {error}", path.display()))
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    /// The version the pointer names.
    pub current: String,
    /// The version before the last update, kept for rollback.
    #[serde(default)]
    pub previous: Option<String>,
    /// Archive digest per installed version.
    #[serde(default)]
    pub digests: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    pub extension: String,
    pub version: String,
    pub capability: Capability,
    pub repository: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Consent {
    /// What the person agreed to, as it was worded when they agreed.
    pub statement: String,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(default)]
    pub installed: BTreeMap<String, Installation>,
    #[serde(default)]
    pub development: BTreeMap<String, PathBuf>,
    /// Extension id → repository ids it is enabled for.
    #[serde(default)]
    pub enabled: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub grants: BTreeSet<Grant>,
    /// Extension id → repository id → consent.
    #[serde(default)]
    pub consent: BTreeMap<String, BTreeMap<String, Consent>>,
    /// Extension id → key → value, for user-scoped settings.
    #[serde(default)]
    pub settings: BTreeMap<String, BTreeMap<String, Value>>,
    /// Extension id → tool id → the executable the user chose.
    #[serde(default)]
    pub executables: BTreeMap<String, BTreeMap<String, PathBuf>>,
    /// Extension id → scope → key → value, for `storage.get`/`storage.set`.
    #[serde(default)]
    pub data: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>>,
}

impl State {
    pub fn is_enabled(&self, extension: &str, repository: &str) -> bool {
        self.enabled
            .get(extension)
            .is_some_and(|repos| repos.contains(repository))
    }

    pub fn granted(
        &self,
        extension: &str,
        version: &str,
        repository: &str,
    ) -> BTreeSet<Capability> {
        self.grants
            .iter()
            .filter(|g| {
                g.extension == extension && g.version == version && g.repository == repository
            })
            .map(|g| g.capability)
            .collect()
    }

    pub fn grant(
        &mut self,
        extension: &str,
        version: &str,
        repository: &str,
        capability: Capability,
    ) {
        self.grants.insert(Grant {
            extension: extension.into(),
            version: version.into(),
            capability,
            repository: repository.into(),
        });
    }

    pub fn revoke(&mut self, extension: &str, repository: &str, capability: Capability) {
        self.grants.retain(|g| {
            !(g.extension == extension && g.repository == repository && g.capability == capability)
        });
    }

    /// Carry an extension's grants from one version to the next, but only for
    /// capabilities both versions ask for. Anything new waits for a person.
    pub fn carry_grants(
        &mut self,
        extension: &str,
        from: &str,
        to: &str,
        still_asked: &BTreeSet<Capability>,
    ) {
        let carried: Vec<Grant> = self
            .grants
            .iter()
            .filter(|g| {
                g.extension == extension && g.version == from && still_asked.contains(&g.capability)
            })
            .map(|g| Grant {
                version: to.into(),
                ..g.clone()
            })
            .collect();
        self.grants.extend(carried);
    }

    /// Forget everything about an extension except, optionally, nothing else:
    /// grants, enablement, consent, settings, executables and data all go.
    pub fn forget(&mut self, extension: &str) {
        self.installed.remove(extension);
        self.enabled.remove(extension);
        self.grants.retain(|g| g.extension != extension);
        self.consent.remove(extension);
        self.settings.remove(extension);
        self.executables.remove(extension);
        self.data.remove(extension);
    }

    pub fn consented(&self, extension: &str, repository: &str) -> bool {
        self.consent
            .get(extension)
            .is_some_and(|repos| repos.contains_key(repository))
    }
}

/// The user state file, loaded once and written on every change.
#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    state: Mutex<State>,
}

impl Store {
    /// Load `dir/state.json`. A missing or unreadable file is an empty state —
    /// a hand-edited file must never be why Spagitty will not start — and an
    /// unreadable one is kept aside rather than overwritten.
    pub fn open(dir: &Path) -> Store {
        let path = dir.join(STATE_FILE);
        let state = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(state) => state,
                Err(_) => {
                    let _ = std::fs::rename(&path, dir.join("state.unreadable.json"));
                    State::default()
                }
            },
            Err(_) => State::default(),
        };
        Store {
            path,
            state: Mutex::new(state),
        }
    }

    pub fn read<T>(&self, f: impl FnOnce(&State) -> T) -> T {
        f(&self.state.lock().expect("extension state lock"))
    }

    /// Change the state and save it. The change is kept in memory only if it
    /// was saved, so memory and disk cannot disagree.
    pub fn update<T>(&self, f: impl FnOnce(&mut State) -> T) -> Result<T> {
        let mut state = self.state.lock().expect("extension state lock");
        let mut next = state.clone();
        let out = f(&mut next);
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| Error::Io(e.to_string()))?;
        write_atomic(&self.path, &bytes)?;
        *state = next;
        Ok(out)
    }
}

/// `.spagitty/extensions` in the main checkout of the repository at
/// `main_workdir`, created and excluded from git on first use.
pub fn repository_dir(main_workdir: &Path) -> PathBuf {
    let dir = main_workdir.join(REPOSITORY_DIR).join("extensions");
    exclude(main_workdir);
    dir
}

/// Add `/.spagitty/` to the repository's `info/exclude` if it is not there.
/// The same entry the farm writes; whichever comes first adds it once.
fn exclude(main_workdir: &Path) {
    let file = main_workdir.join(".git").join("info").join("exclude");
    if !main_workdir.join(".git").is_dir() {
        return;
    }
    let current = std::fs::read_to_string(&file).unwrap_or_default();
    let entry = format!("/{REPOSITORY_DIR}/");
    if current.lines().any(|line| line.trim() == entry) {
        return;
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("# Spagitty: agent farm and extension state.\n");
    next.push_str(&entry);
    next.push('\n');
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&file, next);
}

/// Repository-scoped settings: `settings.json` beside the history.
pub fn read_repository_settings(main_workdir: &Path) -> BTreeMap<String, BTreeMap<String, Value>> {
    let path = repository_dir(main_workdir).join("settings.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn write_repository_setting(
    main_workdir: &Path,
    extension: &str,
    key: &str,
    value: Value,
) -> Result<()> {
    let mut all = read_repository_settings(main_workdir);
    all.entry(extension.to_string())
        .or_default()
        .insert(key.to_string(), value);
    let bytes = serde_json::to_vec_pretty(&all).map_err(|e| Error::Io(e.to_string()))?;
    write_atomic(&repository_dir(main_workdir).join("settings.json"), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_survives_a_reopen_and_an_unreadable_file_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path());
        store
            .update(|s| {
                s.enabled
                    .entry("a.b".into())
                    .or_default()
                    .insert("repo".into());
                s.grant("a.b", "1.0.0", "repo", Capability::RepositoryRead);
            })
            .unwrap();
        let again = Store::open(dir.path());
        assert!(again.read(|s| s.is_enabled("a.b", "repo")));
        assert!(again.read(|s| s
            .granted("a.b", "1.0.0", "repo")
            .contains(&Capability::RepositoryRead)));

        std::fs::write(dir.path().join(STATE_FILE), "{broken").unwrap();
        let broken = Store::open(dir.path());
        assert!(!broken.read(|s| s.is_enabled("a.b", "repo")));
        assert!(dir.path().join("state.unreadable.json").exists());
    }

    #[test]
    fn grants_are_per_version_and_per_repository() {
        let mut state = State::default();
        state.grant("a.b", "1.0.0", "one", Capability::ToolsExecute);
        assert!(state.granted("a.b", "1.0.0", "two").is_empty());
        assert!(state.granted("a.b", "1.1.0", "one").is_empty());
        state.revoke("a.b", "one", Capability::ToolsExecute);
        assert!(state.granted("a.b", "1.0.0", "one").is_empty());
    }

    #[test]
    fn an_update_carries_only_the_grants_both_versions_ask_for() {
        let mut state = State::default();
        state.grant("a.b", "1.0.0", "r", Capability::RepositoryRead);
        state.grant("a.b", "1.0.0", "r", Capability::ToolsExecute);
        let asked: BTreeSet<_> =
            [Capability::RepositoryRead, Capability::ForgePullRequestRead].into();
        state.carry_grants("a.b", "1.0.0", "2.0.0", &asked);
        let now = state.granted("a.b", "2.0.0", "r");
        assert!(now.contains(&Capability::RepositoryRead));
        assert!(!now.contains(&Capability::ToolsExecute));
        assert!(
            !now.contains(&Capability::ForgePullRequestRead),
            "a new capability is never granted silently"
        );
    }

    #[test]
    fn forgetting_an_extension_forgets_every_decision_about_it() {
        let mut state = State::default();
        state.grant("a.b", "1", "r", Capability::RepositoryRead);
        state
            .enabled
            .entry("a.b".into())
            .or_default()
            .insert("r".into());
        state.consent.entry("a.b".into()).or_default().insert(
            "r".into(),
            Consent {
                statement: "s".into(),
                at_ms: 1,
            },
        );
        state.grant("c.d", "1", "r", Capability::RepositoryRead);
        state.forget("a.b");
        assert!(!state.is_enabled("a.b", "r"));
        assert!(!state.consented("a.b", "r"));
        assert_eq!(state.grants.len(), 1);
    }

    #[test]
    fn repository_settings_live_in_an_excluded_directory() {
        let fixture = spagitty_core::fixture::Fixture::woven();
        write_repository_setting(fixture.path(), "a.b", "farmMode", "required".into()).unwrap();
        let read = read_repository_settings(fixture.path());
        assert_eq!(read["a.b"]["farmMode"], Value::from("required"));
        let exclude = std::fs::read_to_string(fixture.path().join(".git/info/exclude")).unwrap();
        assert_eq!(exclude.matches("/.spagitty/").count(), 1);
        repository_dir(fixture.path());
        let exclude = std::fs::read_to_string(fixture.path().join(".git/info/exclude")).unwrap();
        assert_eq!(exclude.matches("/.spagitty/").count(), 1, "added once");
    }
}

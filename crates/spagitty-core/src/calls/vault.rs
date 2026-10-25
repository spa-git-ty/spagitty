// SPDX-License-Identifier: GPL-3.0-or-later

//! Where secret and kept values live (FEAT-161): the OS keychain, through
//! the same seam as a forge token (`forge/keychain.rs`), and never a file in
//! the repository, a share or history.
//!
//! One keychain entry per repository and environment holds that
//! environment's values as a JSON object, so a repository with three
//! environments is three entries rather than one per variable, and macOS
//! asks once for each rather than once for every name. The shared
//! variables are the environment `*`.
//!
//! The keychain is behind [`Vault`] so the rules can be tested with
//! [`Memory`]; the application uses [`Keychain`].

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use crate::forge::keychain;
use crate::{Error, Result};

/// The entries' "host", as a forge token's is its host.
const HOST: &str = "calls";
/// The stem the shared variables are kept under.
pub const SHARED: &str = "*";

pub trait Vault: Send + Sync {
    fn read(&self, key: &str) -> Result<Option<String>>;
    fn write(&self, key: &str, value: &str) -> Result<()>;
    fn forget(&self, key: &str) -> Result<()>;
}

/// The OS keychain.
pub struct Keychain;

impl Vault for Keychain {
    fn read(&self, key: &str) -> Result<Option<String>> {
        keychain::read(HOST, key)
    }

    fn write(&self, key: &str, value: &str) -> Result<()> {
        keychain::store(HOST, key, value)
    }

    fn forget(&self, key: &str) -> Result<()> {
        keychain::forget(HOST, key)
    }
}

/// A vault in memory, for tests.
#[derive(Default)]
pub struct Memory(pub Mutex<HashMap<String, String>>);

impl Vault for Memory {
    fn read(&self, key: &str) -> Result<Option<String>> {
        Ok(self.0.lock().map_err(poisoned)?.get(key).cloned())
    }

    fn write(&self, key: &str, value: &str) -> Result<()> {
        self.0
            .lock()
            .map_err(poisoned)?
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn forget(&self, key: &str) -> Result<()> {
        self.0.lock().map_err(poisoned)?.remove(key);
        Ok(())
    }
}

fn poisoned<T>(_: T) -> Error {
    Error::Keychain("the vault's lock was poisoned".into())
}

/// The entry for `repo` (a hash of its path) and an environment's stem, or
/// the shared variables for `None`.
pub fn key(repo: &str, environment: Option<&str>) -> String {
    format!("{repo}/{}", environment.unwrap_or(SHARED))
}

/// Every value kept for one environment. An entry that is not a JSON object
/// of strings reads as empty rather than failing the send.
pub fn values(
    vault: &dyn Vault,
    repo: &str,
    environment: Option<&str>,
) -> Result<BTreeMap<String, String>> {
    Ok(vault
        .read(&key(repo, environment))?
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default())
}

fn write_all(
    vault: &dyn Vault,
    repo: &str,
    environment: Option<&str>,
    values: &BTreeMap<String, String>,
) -> Result<()> {
    let key = key(repo, environment);
    if values.is_empty() {
        return vault.forget(&key);
    }
    let text = serde_json::to_string(values).map_err(|e| Error::Calls(e.to_string()))?;
    vault.write(&key, &text)
}

/// Set one value, or remove it with `None`.
pub fn set(
    vault: &dyn Vault,
    repo: &str,
    environment: Option<&str>,
    name: &str,
    value: Option<&str>,
) -> Result<()> {
    let mut all = values(vault, repo, environment)?;
    match value {
        Some(value) => all.insert(name.to_string(), value.to_string()),
        None => all.remove(name),
    };
    write_all(vault, repo, environment, &all)
}

/// Several values at once: one read and one write.
pub fn set_many(
    vault: &dyn Vault,
    repo: &str,
    environment: Option<&str>,
    changes: &[(String, String)],
) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    let mut all = values(vault, repo, environment)?;
    for (name, value) in changes {
        all.insert(name.clone(), value.clone());
    }
    write_all(vault, repo, environment, &all)
}

/// Move an environment's values to its new stem.
pub fn rename(vault: &dyn Vault, repo: &str, from: &str, to: &str) -> Result<()> {
    if from == to {
        return Ok(());
    }
    let all = values(vault, repo, Some(from))?;
    write_all(vault, repo, Some(to), &all)?;
    vault.forget(&key(repo, Some(from)))
}

/// Copy an environment's values to another, as *Duplicate* does.
pub fn copy(vault: &dyn Vault, repo: &str, from: &str, to: &str) -> Result<()> {
    let all = values(vault, repo, Some(from))?;
    write_all(vault, repo, Some(to), &all)
}

pub fn drop_environment(vault: &dyn Vault, repo: &str, environment: &str) -> Result<()> {
    vault.forget(&key(repo, Some(environment)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_per_repository_and_environment() {
        let vault = Memory::default();
        set(&vault, "r1", Some("staging"), "token", Some("ey.J")).unwrap();
        set(&vault, "r1", Some("staging"), "password", Some("hunter2")).unwrap();
        set(&vault, "r1", None, "apiKey", Some("k")).unwrap();
        set(&vault, "r2", Some("staging"), "token", Some("other")).unwrap();

        let staging = values(&vault, "r1", Some("staging")).unwrap();
        assert_eq!(staging.get("token").map(String::as_str), Some("ey.J"));
        assert_eq!(staging.len(), 2);
        assert_eq!(values(&vault, "r1", None).unwrap().len(), 1);
        assert_eq!(
            values(&vault, "r2", Some("staging")).unwrap()["token"],
            "other"
        );

        set(&vault, "r1", Some("staging"), "token", None).unwrap();
        set(&vault, "r1", Some("staging"), "password", None).unwrap();
        // The last value gone, the entry goes too.
        assert!(!vault.0.lock().unwrap().contains_key("r1/staging"));
    }

    #[test]
    fn renaming_copying_and_dropping_an_environment_move_its_values() {
        let vault = Memory::default();
        set_many(
            &vault,
            "r",
            Some("stage"),
            &[("a".into(), "1".into()), ("b".into(), "2".into())],
        )
        .unwrap();
        rename(&vault, "r", "stage", "staging").unwrap();
        assert!(values(&vault, "r", Some("stage")).unwrap().is_empty());
        assert_eq!(values(&vault, "r", Some("staging")).unwrap().len(), 2);
        copy(&vault, "r", "staging", "staging-2").unwrap();
        assert_eq!(values(&vault, "r", Some("staging-2")).unwrap().len(), 2);
        drop_environment(&vault, "r", "staging").unwrap();
        assert!(values(&vault, "r", Some("staging")).unwrap().is_empty());
    }

    #[test]
    fn an_entry_that_is_not_a_map_reads_as_empty() {
        let vault = Memory::default();
        vault.write("r/staging", "not json").unwrap();
        assert!(values(&vault, "r", Some("staging")).unwrap().is_empty());
    }
}

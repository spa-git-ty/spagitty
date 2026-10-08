// SPDX-License-Identifier: GPL-3.0-or-later

//! Where a forge token lives.
//!
//! The OS keychain, and nowhere else. The item is explicit — *the token in the
//! OS keychain and never in a config file* — and so is CONTRIBUTING's design
//! rule: credentials go in the OS keychain. Spagitty's own `settings.json` sits
//! in a directory people read, edit, sync and back up, and a token in it would
//! be a token in all four.
//!
//! What the keychain holds is the secret. What the account *is* — the host and
//! the login — is ordinary configuration and lives beside the other settings,
//! because losing it costs a re-entry and knowing it costs nothing.
//!
//! # Failing to reach the keychain is not fatal
//!
//! Every function here reports rather than panics. A locked keyring, a headless
//! session with no secret service running, a keychain the user denied access
//! to — each is a reason the Pull requests screen cannot read, and none is a
//! reason the application should stop. Screens that do not need a token are
//! unaffected.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use keyring::Entry;

use crate::{Error, Result};

/// Tokens already read this session, by [`key`].
///
/// macOS asks for the keychain password on every read by a build it does not
/// trust yet — and an ad-hoc signed update is one it does not trust. Every
/// forge request reads the token, so a session could ask ten times. Reading
/// each account once and keeping it here makes that once per launch. Only
/// found tokens are kept: a refusal or a missing entry is asked again.
fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

fn remember(host: &str, user: &str, token: Option<&str>) {
    if let Ok(mut held) = cache().lock() {
        match token {
            Some(token) => held.insert(key(host, user), token.to_string()),
            None => held.remove(&key(host, user)),
        };
    }
}

/// The service name entries are filed under.
///
/// One per application, with the account as the entry's user, which is the
/// convention every keychain browser expects. A person looking through their
/// keyring sees one Spagitty group rather than a scatter.
const SERVICE: &str = "dev.spagitty.app";

/// The key one account's token is stored under.
///
/// Host and login together: two accounts on the same host are two tokens, and
/// the same login on two hosts is two tokens as well.
fn key(host: &str, user: &str) -> String {
    format!("{host}:{user}")
}

fn entry(host: &str, user: &str) -> Result<Entry> {
    Entry::new(SERVICE, &key(host, user)).map_err(|error| Error::Keychain(error.to_string()))
}

/// Store a token, replacing whatever was there.
pub fn store(host: &str, user: &str, token: &str) -> Result<()> {
    entry(host, user)?
        .set_password(token)
        .map_err(|error| Error::Keychain(error.to_string()))?;
    remember(host, user, Some(token));
    Ok(())
}

/// Read a token back.
///
/// A missing entry is `Ok(None)` rather than an error: an account whose token
/// was removed from the keychain by hand is a disconnected account, which the
/// screen can say, and not a failure it has to report as one.
pub fn read(host: &str, user: &str) -> Result<Option<String>> {
    if let Some(token) = cache()
        .lock()
        .ok()
        .and_then(|held| held.get(&key(host, user)).cloned())
    {
        return Ok(Some(token));
    }
    match entry(host, user)?.get_password() {
        Ok(token) => {
            remember(host, user, Some(&token));
            Ok(Some(token))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(Error::Keychain(error.to_string())),
    }
}

/// Remove a token. Removing one that is not there is not an error — the
/// intended state is "no token for this account", and it already holds.
pub fn forget(host: &str, user: &str) -> Result<()> {
    remember(host, user, None);
    match entry(host, user)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(Error::Keychain(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_accounts_on_one_host_are_two_different_entries() {
        assert_ne!(key("github.com", "ada"), key("github.com", "grace"));
    }

    #[test]
    fn one_login_on_two_hosts_is_two_different_entries() {
        // The same person's name on github.com and on an enterprise
        // installation are two tokens, and handing one to the other would be
        // sending a credential to a host it was never issued for.
        assert_ne!(key("github.com", "ada"), key("github.example.com", "ada"));
    }

    #[test]
    fn the_key_carries_both_parts_so_neither_can_be_confused_for_the_other() {
        assert_eq!(key("github.com", "ada"), "github.com:ada");
    }

    #[test]
    fn a_token_read_once_is_not_read_from_the_keychain_again() {
        // The cache is checked before the keychain is touched, so this needs
        // no secret service — and a second read would be a second macOS prompt.
        remember("cache.example", "ada", Some("t0ken"));
        assert_eq!(
            read("cache.example", "ada").unwrap().as_deref(),
            Some("t0ken")
        );

        remember("cache.example", "ada", None);
        assert!(cache().lock().unwrap().get(&key("cache.example", "ada")).is_none());
    }

    // Reading and writing a real keychain is not tested here. It needs a
    // running secret service, a session bus and an unlocked keyring, none of
    // which a CI container has — and a test that silently skipped when they
    // were missing would be a test that never ran anywhere. What is testable
    // without them is the key scheme, which is the part with a decision in it.
    //
    // SWEEP-017-05 covers the round trip by hand, on a desktop, which is where
    // it actually has to work.
}

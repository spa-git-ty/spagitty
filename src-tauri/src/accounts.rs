// SPDX-License-Identifier: GPL-3.0-or-later

//! Which hosting accounts are connected.
//!
//! Application state, like [`crate::recents`] and [`crate::settings`], and
//! stored the same way — a JSON file in Spagitty's own configuration directory.
//!
//! **The tokens are not here.** This file holds a host and a login, which is
//! ordinary configuration: losing it costs a re-entry, and anybody who can read
//! it learns something they could have read off the git remote anyway. The
//! secret lives in the OS keychain, keyed by the same pair, and
//! `spagitty_core::forge::keychain` is the only thing that touches it.
//!
//! That split is the whole design. A token in this file would be a token in a
//! directory people read, edit, sync and back up.

use std::path::PathBuf;

use spagitty_core::forge::Account;
use tauri::{AppHandle, Manager, Runtime};

const FILE: &str = "accounts.json";

/// The connected accounts.
///
/// A missing or unreadable file is an empty list rather than an error: the same
/// treatment the repository list gets, and for the same reason — a hand-edited
/// file must never be why the application will not start.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Vec<Account> {
    let Some(path) = file(app) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse(&text)
}

/// Add an account, replacing any entry for the same host and login.
pub fn remember<R: Runtime>(app: &AppHandle<R>, account: Account) -> Vec<Account> {
    let next = with(load(app), account);
    save(app, &next);
    next
}

/// Remove one account. The token is forgotten by the caller — this file and the
/// keychain are two writes, and only one of them can fail in a way worth
/// reporting.
pub fn forget<R: Runtime>(app: &AppHandle<R>, host: &str, user: &str) -> Vec<Account> {
    let next = without(load(app), host, user);
    save(app, &next);
    next
}

/// The account for one host, if one is connected.
///
/// The first for that host. Two accounts on one host is a shape this supports
/// storing but does not ask a person to choose between per repository — that
/// would be a question on every read, for a case almost nobody has.
pub fn for_host<'a>(accounts: &'a [Account], host: &str) -> Option<&'a Account> {
    accounts.iter().find(|account| account.host == host)
}

fn parse(text: &str) -> Vec<Account> {
    serde_json::from_str(text).unwrap_or_default()
}

fn with(mut accounts: Vec<Account>, account: Account) -> Vec<Account> {
    accounts.retain(|held| !(held.host == account.host && held.user == account.user));
    accounts.push(account);
    accounts
}

fn without(mut accounts: Vec<Account>, host: &str, user: &str) -> Vec<Account> {
    accounts.retain(|held| !(held.host == host && held.user == user));
    accounts
}

/// Let git sign in to each account's host with its token (BUG-097): when
/// the application starts, and whenever the accounts change.
pub fn share_with_git(accounts: &[Account]) {
    spagitty_core::shell::sign_in_with_accounts(
        accounts
            .iter()
            .map(|account| (account.host.clone(), account.user.clone()))
            .collect(),
    );
}

fn save<R: Runtime>(app: &AppHandle<R>, accounts: &[Account]) {
    share_with_git(accounts);
    let Some(path) = file(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(accounts) {
        let _ = std::fs::write(path, text);
    }
}

fn file<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::forge::Kind;

    fn account(host: &str, user: &str) -> Account {
        Account {
            kind: Kind::GitHub,
            host: host.into(),
            user: user.into(),
        }
    }

    #[test]
    fn no_file_means_no_accounts_connected() {
        assert_eq!(parse(""), Vec::<Account>::new());
    }

    #[test]
    fn a_hand_edited_file_that_is_not_a_list_does_not_stop_the_application() {
        // It sits in the user's config directory and invites editing.
        for corrupt in ["{", "{\"accounts\": []}", "[1, 2, 3]", "null", "not json"] {
            assert_eq!(parse(corrupt), Vec::<Account>::new(), "for {corrupt:?}");
        }
    }

    #[test]
    fn an_account_reads_back_as_it_was_written() {
        let written = serde_json::to_string(&vec![account("github.com", "ada")]).unwrap();

        assert_eq!(parse(&written), vec![account("github.com", "ada")]);
    }

    #[test]
    fn the_stored_file_carries_no_token() {
        // The point of the whole split. `Account` has no token field, so this
        // asserts the shape rather than the contents — a field added later
        // would fail here before it ever reached a disk.
        let text = serde_json::to_string(&vec![account("github.com", "ada")]).unwrap();

        assert!(!text.contains("token"), "{text}");
        assert!(!text.contains("password"), "{text}");
        assert!(!text.contains("secret"), "{text}");
    }

    #[test]
    fn reconnecting_the_same_account_replaces_it_rather_than_listing_it_twice() {
        let list = with(
            vec![account("github.com", "ada")],
            account("github.com", "ada"),
        );

        assert_eq!(list.len(), 1);
    }

    #[test]
    fn two_logins_on_one_host_are_two_accounts() {
        let list = with(
            vec![account("github.com", "ada")],
            account("github.com", "grace"),
        );

        assert_eq!(list.len(), 2);
    }

    #[test]
    fn one_login_on_two_hosts_is_two_accounts() {
        // github.com and an enterprise installation are different services and
        // different tokens, even for the same person.
        let list = with(
            vec![account("github.com", "ada")],
            account("github.example.com", "ada"),
        );

        assert_eq!(list.len(), 2);
    }

    #[test]
    fn disconnecting_removes_that_one_and_leaves_the_rest() {
        let list = without(
            vec![account("github.com", "ada"), account("github.com", "grace")],
            "github.com",
            "ada",
        );

        assert_eq!(list, vec![account("github.com", "grace")]);
    }

    #[test]
    fn disconnecting_something_that_is_not_connected_changes_nothing() {
        let list = without(vec![account("github.com", "ada")], "github.com", "nobody");

        assert_eq!(list.len(), 1);
    }

    #[test]
    fn a_repository_is_read_with_the_account_for_its_own_host() {
        // Handing an enterprise token to github.com would be sending a
        // credential to a service it was never issued for.
        let accounts = vec![
            account("github.example.com", "ada"),
            account("github.com", "grace"),
        ];

        assert_eq!(for_host(&accounts, "github.com").unwrap().user, "grace");
        assert_eq!(
            for_host(&accounts, "github.example.com").unwrap().user,
            "ada"
        );
        assert!(for_host(&accounts, "gitlab.com").is_none());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Pull request notifications (FEAT-114).
//!
//! Two commands. One reads what every connected account's pull requests look
//! like now; the screen keeps the last answer and decides what changed, since
//! it is also the one that says so. The other hands a line to the operating
//! system's notification centre.
//!
//! The notification is sent **without a sound**. Spagitty plays its own cue,
//! at the level the Sound setting chose, so a person who picked Off hears
//! nothing and nobody hears the system chime and Spagitty's on top of it.

use spagitty_core::forge::watch::{self, Watched};
use spagitty_core::forge::{self, Account};
use spagitty_core::{Error, Result};
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::accounts;

/// What every connected account's pull requests look like now.
///
/// One account failing does not hide the others: the answer is everything
/// that was read, and only when nothing could be read is it an error.
#[tauri::command]
pub async fn watch_pull_requests<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Watched>> {
    let connected = accounts::load(&app);
    match tauri::async_runtime::spawn_blocking(move || read_all(&connected)).await {
        Ok(result) => result,
        Err(error) => Err(Error::Forge {
            host: String::new(),
            detail: format!("the request could not be run: {error}"),
        }),
    }
}

fn read_all(connected: &[Account]) -> Result<Vec<Watched>> {
    let mut found = Vec::new();
    let mut failed = None;
    let mut answered = false;

    for account in connected {
        match read_one(account) {
            Ok(rows) => {
                answered = true;
                found.extend(rows);
            }
            Err(error) => {
                failed.get_or_insert(error);
            }
        }
    }

    match failed {
        Some(error) if !answered => Err(error),
        _ => Ok(found),
    }
}

fn read_one(account: &Account) -> Result<Vec<Watched>> {
    let Some(token) = forge::keychain::read(&account.host, &account.user)? else {
        return Err(Error::ForgeUnauthorized {
            host: account.host.clone(),
            detail: "the token for this account is no longer in the keychain".into(),
        });
    };
    watch::watch(account.kind, &account.host, &token, &account.user)
}

/// Show one notification in the operating system's notification centre.
#[tauri::command]
pub fn notify_desktop<R: Runtime>(
    app: AppHandle<R>,
    title: String,
    body: String,
) -> std::result::Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())
}

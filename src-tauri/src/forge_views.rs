// SPDX-License-Identifier: GPL-3.0-or-later

//! Local context shared by the forge screens. A remote name is resolved from
//! this repository before credentials are read; no authenticated URL comes
//! from the webview. Cached data contains no token.

use crate::{accounts, commands::AppState};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use spagitty_core::{
    forge::{self, Repo},
    Error, Result,
};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hosted {
    pub remote: String,
    pub repo: Repo,
    pub user: Option<String>,
}

pub fn path(state: &AppState, command: &'static str) -> Result<PathBuf> {
    state.open_path(command)
}
pub fn remotes<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<Vec<Hosted>> {
    let known = accounts::load(app);
    let git = spagitty_core::repo::open(path)?;
    Ok(spagitty_core::remotes::remotes(&git)
        .into_iter()
        .filter_map(|remote| {
            let repo = forge::identify_with(&remote.url, &known)?;
            let user = accounts::for_host(&known, &repo.host).map(|account| account.user.clone());
            Some(Hosted {
                remote: remote.name,
                repo,
                user,
            })
        })
        .collect())
}
pub fn selected<R: Runtime>(app: &AppHandle<R>, path: &Path, remote: &str) -> Result<Hosted> {
    remotes(app, path)?
        .into_iter()
        .find(|entry| entry.remote == remote)
        .ok_or_else(|| Error::Forge {
            host: String::new(),
            detail: "Choose one of this repository's hosted remotes.".into(),
        })
}
pub fn token(entry: &Hosted) -> Result<String> {
    let user = entry
        .user
        .as_deref()
        .ok_or_else(|| Error::ForgeUnauthorized {
            host: entry.repo.host.clone(),
            detail: "Connect an account for this host in Settings.".into(),
        })?;
    forge::keychain::read(&entry.repo.host, user)?.ok_or_else(|| Error::ForgeUnauthorized {
        host: entry.repo.host.clone(),
        detail: "The connected account's token is no longer in the keychain.".into(),
    })
}
pub fn cache_file<R: Runtime>(
    app: &AppHandle<R>,
    entry: &Hosted,
    category: &str,
    key: &str,
) -> Result<PathBuf> {
    let account = format!(
        "{}/{}/{}",
        entry.repo.host,
        entry.repo.slug(),
        entry.user.as_deref().unwrap_or("")
    );
    let bucket = spagitty_core::hooks::generate::hash(account.as_bytes());
    let key = spagitty_core::hooks::generate::hash(key.as_bytes());
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| Error::Config(error.to_string()))?
        .join("forge")
        .join(format!("{bucket:016x}"))
        .join(category)
        .join(format!("{key:016x}.json")))
}
pub fn cached<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > 8 * 1024 * 1024 {
        return None;
    }
    serde_json::from_reader(file).ok()
}
pub fn save<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    static WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITES
        .lock()
        .map_err(|error| Error::Config(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("partial");
    std::fs::write(
        &temporary,
        serde_json::to_vec(data).map_err(|error| Error::Config(error.to_string()))?,
    )?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Read<T> {
    pub data: T,
    pub cache_warning: Option<String>,
}
pub fn keep<T: Serialize>(path: &Path, data: T) -> Read<T> {
    let cache_warning = save(path, &data).err().map(|error| {
        format!("The answer arrived, but its local cache could not be saved: {error}")
    });
    Read {
        data,
        cache_warning,
    }
}

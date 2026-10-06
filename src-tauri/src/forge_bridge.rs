// SPDX-License-Identifier: GPL-3.0-or-later

//! Forge calls made on an extension's behalf (FEAT-096, FEAT-099).
//!
//! The extension never sees the token, an authenticated URL or the HTTP
//! client: it names a repository handle and a pull request number, and this
//! module finds the connected account for that repository's host, reads the
//! token from the keychain, and calls the core's typed forge operations — the
//! same single HTTP boundary every other forge feature uses.

use std::path::Path;

use serde_json::Value;
use spagitty_core::forge::{self, Repo};
use spagitty_extensions::protocol::{code, RpcError};
use tauri::{AppHandle, Runtime};

fn refused(code: i64, message: impl Into<String>) -> RpcError {
    RpcError::new(code, message)
}

/// The forge repository a working directory points at.
pub fn repo_for(workdir: &Path) -> Result<Repo, RpcError> {
    let repo =
        spagitty_core::repo::open(workdir).map_err(|e| refused(code::INTERNAL, e.to_string()))?;
    forge::identify_repo(&repo)
        .map_err(|e| refused(code::INTERNAL, e.to_string()))?
        .ok_or_else(|| {
            refused(
                code::UNSUPPORTED,
                "this repository is not on a service Spagitty can read",
            )
        })
}

/// "owner/name#412", for the confirmation the person reads.
pub fn describe_pull_request(workdir: &Path, number: u64) -> Result<String, RpcError> {
    let repo = repo_for(workdir)?;
    Ok(format!("{}#{number}", repo.slug()))
}

pub fn pull_request_snapshot<R: Runtime>(
    _app: &AppHandle<R>,
    _workdir: &Path,
    _number: u64,
) -> Result<Value, RpcError> {
    Err(refused(
        code::UNSUPPORTED,
        "pull request snapshots are not available in this build",
    ))
}

pub fn post_pull_request_comment<R: Runtime>(
    _app: &AppHandle<R>,
    _workdir: &Path,
    _number: u64,
    _body: &str,
) -> Result<Value, RpcError> {
    Err(refused(
        code::UNSUPPORTED,
        "pull request comments are not available in this build",
    ))
}

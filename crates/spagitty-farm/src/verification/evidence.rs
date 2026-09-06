// SPDX-License-Identifier: GPL-3.0-or-later

//! Evidence is trusted only for a committed, clean task worktree.
use crate::error::{Error, Result};
use std::path::Path;

pub fn clean_commit(path: &Path) -> Result<String> {
    let repo = spagitty_core::repo::open(path)?;
    if spagitty_core::status::working_copy(&repo)?.changed_paths() != 0 {
        return Err(Error::Refused(
            "Commit or discard the task's remaining changes before merging.".into(),
        ));
    }
    spagitty_core::repo::head(&repo)
        .id
        .ok_or_else(|| Error::Refused("The task has no commit to verify.".into()))
}

pub fn branch(path: &Path) -> Result<String> {
    let repo = spagitty_core::repo::open(path)?;
    spagitty_core::repo::head(&repo).branch.ok_or_else(|| {
        Error::Refused("Select a destination branch before running this task.".into())
    })
}

#[derive(Debug, Clone)]
pub struct CheckedCommit {
    pub commit: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewedCommit {
    pub commit: String,
    pub reviewer: crate::model::AgentId,
}

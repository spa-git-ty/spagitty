// SPDX-License-Identifier: GPL-3.0-or-later

//! Every branch move the farm made (FEAT-144).
//!
//! The landing lane moves a branch only by compare-and-swap, and each move is
//! written down: which ref, from where to where, for which task, made by which
//! agent and passed by which reviewer. It answers "which agent landed this, and
//! who passed it" without parsing merge messages.

use serde::{Deserialize, Serialize};

use super::{AgentId, TaskId};

/// What kind of move an entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedgerKind {
    /// The lane moved the branch to a checked candidate.
    Land,
    /// A compare-and-swap that lost: the branch moved between reading its tip
    /// and moving it, and the candidate was rebuilt.
    RetryLost,
    /// A person merged the task in Merger, and the farm learned of it.
    Merger,
}

/// Who did the work an entry lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerAgent {
    pub task: TaskId,
    #[serde(default)]
    pub agent: Option<AgentId>,
    #[serde(default)]
    pub reviewer: Option<AgentId>,
}

/// One line of the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub seq: u64,
    pub at_ms: u64,
    /// The full ref: `refs/heads/main`.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Where it was. Unknown for a move made in Merger.
    #[serde(default)]
    pub old: Option<String>,
    pub new: String,
    pub kind: LedgerKind,
    #[serde(default)]
    pub tasks: Vec<TaskId>,
    #[serde(default)]
    pub by: Vec<LedgerAgent>,
    /// How the candidate was checked: `before:pass`, `before:skipped` (the tip
    /// had not moved, so the task's own checks stood), or `merger`.
    #[serde(default)]
    pub checks: String,
}

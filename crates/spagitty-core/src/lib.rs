// SPDX-License-Identifier: GPL-3.0-or-later

//! Git operations for Spagitty.
//!
//! Everything here is UI-agnostic: no Tauri, no window handles, no events. The
//! Tauri layer (`src-tauri`) is a thin adapter that calls into this crate and
//! forwards the results.
//!
//! # Where the work happens
//!
//! [`gix`] does the reading — log walking, refs, diffing, blame, status. It is
//! MIT/Apache-2.0, so it links cleanly into a GPL-3 program.
//!
//! A small set of operations is *deliberately not reimplemented* and shells out
//! to the `git` binary instead. That is the entire contents of [`shell`], and it
//! is the only module in this crate that spawns a process. See its header for
//! which operations and why.
//!
//! Because there is exactly one such module, there is exactly one place that
//! knows what was executed: [`record`] holds it, written by [`shell`] as it
//! spawns, and read by the Settings toggle "Show the git command behind each
//! action".

pub mod avatars;
pub mod blame;
pub mod branches;
pub mod clone;
pub mod conflicts;
pub mod diff;
pub mod error;
pub mod forge;
pub mod graph;
pub mod identity;
pub mod ops;
pub mod rebase;
pub mod record;
pub mod reflog;
pub mod refs;
pub mod remotes;
pub mod repo;
pub mod search;
pub mod shell;
pub mod signing;
pub mod stash;
pub mod status;
pub mod submodules;
pub mod tags;
pub mod tools;
pub mod update;
pub mod work;
pub mod worktrees;

/// Repository fixtures, shared by the tests of every module that reads one,
/// and by the Tauri layer's tests behind the `fixture` feature.
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// Re-exported so callers can name gix types (`ThreadSafeRepository` and
/// friends) without depending on gix themselves. There is one gix version in
/// the workspace and this is where it comes from.
pub use gix;

pub use error::{Error, Result};
pub use graph::{GraphRow, LaneEdge, LaneState, ROW_PITCH};
pub use refs::{RefChip, RefIndex, RefKind};
pub use repo::{HeadInfo, RepoInfo};
pub use status::{StatusEntry, WorkingCopy};
pub use submodules::Submodule;
pub use tools::{ExternalToolInfo, ExternalToolsConfig};
pub use worktrees::Worktree;

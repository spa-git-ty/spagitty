// SPDX-License-Identifier: GPL-3.0-or-later

//! Where the work happens: a branch and a working tree per task, and the rule
//! that stops two of them editing the same file.
//!
//! Nothing above this module knows where a worktree lives or what a farm branch
//! is called. That is what lets the naming change — and it did, once, before
//! this was written — without touching the scheduler.

pub mod branch;
pub mod cleanup;
pub mod lock;
pub mod slice;
pub mod worktree;

pub use branch::{branch_name, is_farm_branch, provider_of, task_of, worktree_dir, NAMESPACE};
pub use lock::Leases;
pub use slice::{SliceProvider, WorktreeSlice};
pub use worktree::{create, remove, worktree_path, Workspace, FARM_DIR};

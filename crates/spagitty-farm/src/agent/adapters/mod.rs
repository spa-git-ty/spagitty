// SPDX-License-Identifier: GPL-3.0-or-later

//! One module per provider.
//!
//! Each knows exactly one thing: how to invoke its own CLI without a terminal.
//! Nothing here knows about tasks, worktrees or review — see
//! [`crate::agent::adapter`] for why the boundary is drawn there.
//!
//! # These command lines will go out of date
//!
//! They are somebody else's flags, and they change. Three things keep that from
//! becoming a support burden:
//!
//! 1. Every adapter's arguments are built in one function, named in one file.
//! 2. [`crate::model::AgentDefinition::extra_args`] is appended to every
//!    invocation, so a user whose provider has moved on can correct it from the
//!    settings screen without waiting for a release.
//! 3. [`custom`] exists, so an agent Spagitty has never heard of is a
//!    configuration rather than a patch.

pub mod claude;
pub mod codex;
pub mod cursor;
pub mod custom;
pub mod pi;

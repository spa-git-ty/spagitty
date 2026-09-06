// SPDX-License-Identifier: GPL-3.0-or-later

//! Running the repository's own checks against an agent's work.
//!
//! This is the module that makes the difference between a farm and four
//! terminal windows: no agent's own report can move a task to
//! [`crate::model::TaskStatus::Done`] without passing through here.

pub mod command;
pub mod verifier;

pub use command::CommandResult;
pub use verifier::{commands_for, Verification};

pub(crate) mod evidence;

// SPDX-License-Identifier: GPL-3.0-or-later

//! Starting an agent, reading it, and stopping it.
//!
//! One thread per running agent, a transcript on disk, and a kill signal for
//! cancellation — the same shape as every other long-running job in Spagitty.
//! See [`process`] for why none of it is async.

pub mod log;
pub mod narrate;
pub mod process;
pub(crate) mod tree;

pub use log::{log_path, tail, TranscriptWriter};
pub use narrate::{ClaudeStream, Narrator, Verbatim};
pub use process::{start, Cancellation, Collected, Ended, Session, Sink};

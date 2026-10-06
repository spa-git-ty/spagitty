// SPDX-License-Identifier: GPL-3.0-or-later

//! A process tree owned by one farm run.
//!
//! The implementation moved to `spagitty-process` (FEAT-096) so that extension
//! workers and the tools they run are contained by the same code as agents.
//! Its tests moved with it.

pub use spagitty_process::ProcessTree;

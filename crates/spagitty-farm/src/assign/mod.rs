// SPDX-License-Identifier: GPL-3.0-or-later

//! Assignments: one agent, one job — a review or a merge — at a level the
//! person chose (2.0).
//!
//! The farm runs agents on tasks of its own. An assignment hands an agent a
//! piece of the person's own work instead, from Review or Merger, and keeps
//! the person in charge of it: everything the agent makes is a proposal in the
//! material a person would have made by hand, and how far it goes alone is a
//! [`level::Level`] checked by Spagitty at every gate.
//!
//! ```text
//! level.rs     the four levels against the gates of each job
//! rules.rs     a repository's rules: highest level, branches, verdicts
//! protocol.rs  what an agent answers, and how it is read
//! classify.rs  Spagitty names a resolution in the resolver's words
//! prompt.rs    what an agent is told, one step at a time
//! guard.rs     what an agent wrote where it was not asked to, put back
//! record.rs    the record: who decided what
//! engine.rs    the steps, the gates, the controls
//! local.rs     a command-line agent, as a driver
//! world.rs     the repository, as the engine reaches it
//! ```

pub mod classify;
pub mod engine;
pub mod guard;
pub mod level;
pub mod local;
pub mod prompt;
pub mod protocol;
pub mod record;
pub mod rules;
pub mod world;

#[cfg(test)]
mod tests;

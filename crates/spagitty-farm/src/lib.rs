// SPDX-License-Identifier: GPL-3.0-or-later

//! Agent farm orchestration for Spagitty.
//!
//! Spagitty is a Git client first. This crate is the control plane that lets it
//! also run a farm of coding agents against the repository the user has open —
//! Claude Code, Codex, Cursor, Oh My Pi, or anything else with a command line.
//!
//! # Why this is not in `spagitty-core`
//!
//! `spagitty-core` answers questions about a repository. Nothing in it decides
//! *what should happen next*, and that is the whole of what this crate does.
//! Folding orchestration into the git crate would give every git function a
//! neighbour that spawns other people's programs, and would make the boundary
//! that keeps `shell` the only process-spawning module in that crate meaningless.
//!
//! The dependency runs one way: this crate calls `spagitty-core` for every git
//! operation and reimplements none of them.
//!
//! # The shape of it
//!
//! ```text
//! model/         what a goal, task, agent, run and handoff are
//! agent/         one adapter per provider, plus detection and the registry
//! workspace/     a branch and a worktree per task, and the leases between them
//! execution/     spawning an agent, streaming it, and killing it
//! verification/  running the repository's own checks against the result
//! review/        routing a finished task to a different agent
//! orchestrator/  the DAG, the scheduler, routing and the planner
//! persistence/   the farm on disk, so a crash is not a loss
//! context.rs     what an agent is told
//! policy.rs      the repository's own rules for agents
//! service.rs     the API the Tauri layer calls
//! ```
//!
//! # Two rules that hold everywhere
//!
//! **Agents never talk to each other.** Every handoff goes through Spagitty, so
//! there is one audit trail and one place that decides what happens next.
//!
//! **An agent saying "done" is not done.** Verification runs the repository's
//! own commands, and a review is performed by a different agent than the one
//! that wrote the change. Both are in the path to [`model::TaskStatus::Done`]
//! and neither can be skipped by an agent's own report.

pub mod agent;
pub mod assign;
pub mod context;
pub mod error;
pub mod execution;
pub mod model;
pub mod orchestrator;
pub mod persistence;
pub mod policy;
pub mod review;
pub mod service;
pub mod supplemental;
pub mod verification;
pub mod workspace;

pub use error::{Error, Result};

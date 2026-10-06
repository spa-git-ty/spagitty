// SPDX-License-Identifier: GPL-3.0-or-later

//! The extension host (FEAT-096).
//!
//! An extension is a native program that speaks the protocol in
//! `schemas/extensions/protocol.v1.md` over its stdin and stdout. This crate
//! reads its manifest, validates and installs its package, starts it when it is
//! first needed, answers the questions it asks within the capabilities it was
//! granted, and keeps a record of the reviews it produced.
//!
//! # What it is not
//!
//! **A sandbox.** An extension runs with the user's own operating-system
//! privileges. Process separation means a crash takes down the worker rather
//! than the window; capabilities mean this crate will only do for an extension
//! what it was granted. A worker can still read any file the user can and open
//! its own connections. Every place this crate talks to a person says so.
//!
//! # The shape of it
//!
//! ```text
//! manifest.rs      extension.json: parsing and every rule the schema states
//! version.rs       the three compatibility dimensions
//! capabilities.rs  what an extension may ask the host for
//! protocol.rs      JSON-RPC 2.0 messages, one per line
//! worker.rs        one running worker: the process, its reader, its writer
//! operations.rs    long-running work, exactly one terminal result each
//! tools.rs         external tools: detection and argv built from profiles
//! zip.rs           the package container, read strictly
//! package.rs       validation, install, update, rollback, uninstall
//! registry.rs      what is installed, bundled and attached for development
//! storage.rs       user state: grants, enablement, consent, settings
//! review.rs        the shared review model and the gate the host computes
//! history.rs       review records under `.spagitty/`
//! snapshot.rs      what a review covers, and its identity
//! redact.rs        secrets out of anything kept or shown
//! host.rs          the API the desktop calls, and the callbacks it serves
//! ```
//!
//! # Boundaries
//!
//! No Tauri and no frontend types: the desktop crate composes this one. No
//! dependency on `spagitty-farm`: the farm declares the supplemental review
//! interface and the desktop supplies it from here (FEAT-098), so neither
//! crate can reach into the other's state. Git is read through
//! `spagitty-core`; nothing here writes to a repository's history.

pub mod capabilities;
pub mod error;
pub mod history;
pub mod host;
pub mod manifest;
pub mod operations;
pub mod package;
pub mod protocol;
pub mod redact;
pub mod registry;
pub mod repair;
pub mod review;
pub mod snapshot;
pub mod storage;
pub mod tools;
pub mod version;
pub mod worker;
pub mod zip;

pub use error::{Error, Result};

/// The extension API this host implements.
pub const API_VERSION: &str = "1.0.0";

/// The manifest version this host reads.
pub const MANIFEST_VERSION: u64 = 1;

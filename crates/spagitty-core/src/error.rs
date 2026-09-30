// SPDX-License-Identifier: GPL-3.0-or-later

//! Error type for the core.
//!
//! Errors cross into JavaScript as plain strings, so the `Display` text is user
//! facing. Keep it in the same register as the rest of the UI: plain, specific,
//! no stack-trace jargon.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not a git repository")]
    NotARepository(PathBuf),

    #[error("no repository is open")]
    NoRepository,

    /// An open that a later open or close overtook before it finished. What
    /// the user asked for last wins; this one is dropped without being applied.
    #[error("opening {0} was overtaken by a later request")]
    Superseded(PathBuf),

    #[error("this repository has no commits yet")]
    EmptyRepository,

    #[error("could not read the repository: {0}")]
    Open(String),

    #[error("could not walk the history: {0}")]
    Walk(String),

    #[error("could not read refs: {0}")]
    Refs(String),

    #[error("could not diff: {0}")]
    Diff(String),

    #[error("could not read the working copy: {0}")]
    Status(String),

    #[error("could not read the git configuration: {0}")]
    Config(String),

    #[error("{0}")]
    NotStageable(String),

    /// The screen is showing something the repository no longer has. Raised
    /// rather than applying a stale patch, which would half-stage a file.
    #[error("{0} changed since it was read; reload and try again")]
    Stale(String),

    #[error("a commit needs a subject line")]
    EmptyMessage,

    #[error("no commit {0}")]
    UnknownCommit(String),

    #[error("no file {0} in that commit")]
    UnknownPath(String),

    /// A `git` subprocess failed. Carries git's own stderr, which is usually
    /// the most useful thing we can show.
    #[error("git {command} failed: {stderr}")]
    Git { command: String, stderr: String },

    /// A commit failed *because it could not be signed* (FEAT-019).
    ///
    /// Told apart from an ordinary `Git` failure on purpose. "commit failed" for
    /// a signing problem sends the reader looking at their message, their hooks
    /// and their index; naming the program git tried to run sends them to the
    /// one thing that is actually wrong.
    #[error("{program} could not sign this commit: {stderr}")]
    Signing { program: String, stderr: String },

    // --- Talking to a hosting service (FEAT-017) --------------------------
    //
    // Four variants rather than one, because the item asks for offline and
    // rate-limited behaviour that says *which one it is* — and a screen cannot
    // tell them apart from a single string.
    /// The host could not be reached at all. No status came back, so there is
    /// nothing to interpret: this is the network, not the service.
    #[error("could not reach {host}: {detail}")]
    ForgeOffline { host: String, detail: String },

    /// The host is refusing for now and will accept again later. Distinct from
    /// a permission problem, which never changes on its own.
    #[error("{host} is rate limiting — it should accept requests again {when}")]
    ForgeRateLimited { host: String, when: String },

    /// The token was refused, or does not reach this repository.
    #[error("{host} refused the token: {detail}")]
    ForgeUnauthorized { host: String, detail: String },

    /// The host answered, and what it said was not usable.
    #[error("{host}: {detail}")]
    Forge { host: String, detail: String },

    /// The OS keychain could not be read or written.
    #[error("the system keychain could not be used: {0}")]
    Keychain(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

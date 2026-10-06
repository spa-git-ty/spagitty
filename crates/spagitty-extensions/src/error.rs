// SPDX-License-Identifier: GPL-3.0-or-later

//! What goes wrong, in words a person can act on.
//!
//! `Display` is user-facing, the same rule as `spagitty-core`'s error: the
//! Settings screen shows it as written.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The manifest broke one or more rules. Each line names the field.
    #[error("The extension's manifest is not valid:\n{}", .0.join("\n"))]
    Manifest(Vec<String>),

    /// The package is unsafe or damaged. Nothing from it was run or kept.
    #[error("The package was refused: {0}")]
    Package(String),

    /// This build cannot run it — wrong application version, API or target.
    #[error("{0}")]
    Incompatible(String),

    #[error("No extension called {0} is installed.")]
    NotInstalled(String),

    /// A rule about who may own an identity.
    #[error("{0}")]
    Conflict(String),

    /// Something the user has to do first.
    #[error("{0}")]
    Refused(String),

    /// The worker misbehaved or went away.
    #[error("{0}")]
    Worker(String),

    #[error("{0}")]
    Io(String),

    #[error(transparent)]
    Git(#[from] spagitty_core::Error),
}

impl Error {
    /// A short machine-readable kind for the interface to branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Manifest(_) => "manifest",
            Error::Package(_) => "package",
            Error::Incompatible(_) => "incompatible",
            Error::NotInstalled(_) => "notInstalled",
            Error::Conflict(_) => "conflict",
            Error::Refused(_) => "refused",
            Error::Worker(_) => "worker",
            Error::Io(_) => "io",
            Error::Git(_) => "git",
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut out = serializer.serialize_struct("Error", 2)?;
        out.serialize_field("kind", self.kind())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

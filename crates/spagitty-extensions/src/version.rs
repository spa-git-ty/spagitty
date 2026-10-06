// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether this build can run an extension.
//!
//! Three questions, answered independently because they fail independently:
//!
//! - **Manifest version** — can this host read the file at all?
//! - **Extension API** — does the manifest's range admit the API this host
//!   implements? Negotiated again at the handshake, where the worker names the
//!   version it actually speaks.
//! - **Application version** — does the manifest's range admit this Spagitty?
//!
//! Plus the fourth thing a native package can get wrong: whether it carries an
//! executable for the machine it is on.
//!
//! Every "no" is a sentence, shown at install and on the extension's card,
//! before anything is executed.

use semver::{Version, VersionReq};
use serde::Serialize;

use crate::manifest::Manifest;

/// The target triple this build was compiled for, in the manifest's
/// vocabulary.
pub fn current_target() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else {
        "unsupported"
    }
}

/// Parse a manifest version requirement.
///
/// The Cargo dialect, which is what `semver::VersionReq` reads. One
/// leniency: comparators separated by whitespace instead of a comma
/// (`>=0.9.0 <1.0.0`, the npm habit) are accepted by inserting the comma, since
/// both mean the same intersection and refusing the one people type first
/// would be a rule about punctuation.
pub fn parse_range(text: &str) -> Option<VersionReq> {
    let text = text.trim();
    if text.is_empty() || text.len() > 100 {
        return None;
    }
    if let Ok(req) = VersionReq::parse(text) {
        return Some(req);
    }
    VersionReq::parse(&normalise_range(text)).ok()
}

/// `>=0.9.0 <1.0.0` → `>=0.9.0, <1.0.0`. Only whitespace that separates two
/// comparators becomes a comma; whitespace between an operator and its
/// version is kept.
fn normalise_range(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut pending_space = false;
    let mut previous: Option<char> = None;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space {
            let after_operator = matches!(previous, Some('<' | '>' | '=' | '~' | '^' | ','));
            let starts_comparator = matches!(character, '<' | '>' | '=' | '~' | '^');
            if !after_operator && starts_comparator {
                out.push(',');
            }
            out.push(' ');
            pending_space = false;
        }
        out.push(character);
        previous = Some(character);
    }
    out
}

pub fn parse_version(text: &str) -> Option<Version> {
    Version::parse(text.trim()).ok()
}

/// Whether `version` satisfies `range`.
///
/// A pre-release application (`0.9.0-alpha.1`) satisfies a range written
/// against its release (`>=0.9.0`) only if the range opts into pre-releases,
/// which is `semver`'s rule and the conservative one: an alpha is not the
/// release an author tested against.
pub fn satisfies(range: &str, version: &str) -> bool {
    match (parse_range(range), parse_version(version)) {
        (Some(range), Some(version)) => range.matches(&version),
        _ => false,
    }
}

/// The answer, for the card and the install dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub compatible: bool,
    /// One sentence per reason it is not. Empty when it is.
    pub reasons: Vec<String>,
}

/// Every reason `manifest` cannot run on this build of `app_version`.
pub fn check(manifest: &Manifest, app_version: &str, target: &str) -> Compatibility {
    let mut reasons = Vec::new();

    if !satisfies(&manifest.engines.extension_api, crate::API_VERSION) {
        reasons.push(format!(
            "It needs extension API {}, and this Spagitty provides {}.",
            manifest.engines.extension_api,
            crate::API_VERSION
        ));
    }
    if !satisfies(&manifest.engines.spagitty, app_version) {
        reasons.push(format!(
            "It needs Spagitty {}, and this is {}.",
            manifest.engines.spagitty, app_version
        ));
    }
    if !manifest.runtime.entrypoints.contains_key(target) {
        let mut targets: Vec<&str> = manifest
            .runtime
            .entrypoints
            .keys()
            .map(String::as_str)
            .collect();
        targets.sort_unstable();
        reasons.push(format!(
            "It has no program for this computer ({target}); it was built for {}.",
            targets.join(", ")
        ));
    }

    Compatibility {
        compatible: reasons.is_empty(),
        reasons,
    }
}

/// Whether the version a worker answered the handshake with is one this host
/// speaks: the same major version.
pub fn negotiated(answer: &str) -> bool {
    let (Some(ours), Some(theirs)) = (parse_version(crate::API_VERSION), parse_version(answer))
    else {
        return false;
    };
    ours.major == theirs.major && theirs.pre.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cargo_range_and_its_npm_spelling_mean_the_same() {
        for range in [
            ">=0.9.0, <1.0.0",
            ">=0.9.0 <1.0.0",
            ">= 0.9.0 , < 1.0.0",
            ">= 0.9.0 < 1.0.0",
        ] {
            assert!(satisfies(range, "0.9.0"), "{range}");
            assert!(satisfies(range, "0.9.7"), "{range}");
            assert!(!satisfies(range, "1.0.0"), "{range}");
            assert!(!satisfies(range, "0.8.1"), "{range}");
        }
    }

    #[test]
    fn caret_and_bare_versions_follow_the_cargo_rules() {
        assert!(satisfies("^1.0.0", "1.4.2"));
        assert!(!satisfies("^1.0.0", "2.0.0"));
        assert!(satisfies("1.0", "1.9.0"));
        assert!(satisfies("~0.9", "0.9.5"));
        assert!(!satisfies("~0.9", "0.10.0"));
    }

    #[test]
    fn a_range_that_does_not_parse_admits_nothing() {
        assert!(!satisfies("whenever", "1.0.0"));
        assert!(!satisfies("", "1.0.0"));
        assert!(!satisfies(">=1.0.0", "one"));
        assert!(parse_range(&"1".repeat(101)).is_none());
    }

    #[test]
    fn a_pre_release_is_not_the_release_a_range_names() {
        assert!(!satisfies(">=0.9.0", "0.9.0-alpha.1"));
        assert!(satisfies(">=0.9.0-alpha.1", "0.9.0-alpha.2"));
    }

    #[test]
    fn the_handshake_accepts_the_same_major_version_only() {
        assert!(negotiated("1.0.0"));
        assert!(negotiated("1.7.3"));
        assert!(!negotiated("2.0.0"));
        assert!(!negotiated("0.9.0"));
        assert!(!negotiated("1.0.0-beta"));
        assert!(!negotiated("yes"));
    }

    #[test]
    fn this_build_names_a_target_the_manifest_vocabulary_has() {
        assert_ne!(current_target(), "unsupported");
    }
}

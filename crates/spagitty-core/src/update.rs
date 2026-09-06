// SPDX-License-Identifier: GPL-3.0-or-later

//! Is there a newer Spagitty than this one?
//!
//! # What it sends
//!
//! One unauthenticated `GET` to the project's own releases endpoint, through
//! [`crate::forge::http`] — the same single call site everything else uses, so
//! "what does this application send, and where" still has one answer. No token,
//! no identifier, no version query string. The request says nothing about the
//! machine it came from beyond what any HTTP request says, and the answer is a
//! public page anybody can read without asking.
//!
//! # How it knows which build it is
//!
//! `SPAGITTY_RELEASE`, baked in at compile time by the release workflow and
//! **absent from every other build**. A version number alone cannot answer this
//! question: `Cargo.toml` says `0.1.0` and has said so across several releases,
//! because previews increment their own counter rather than the version. The
//! tag is the only thing that identifies a build.
//!
//! A build without it — anything compiled locally — reports
//! [`Channel::Development`] and never claims to be out of date. Telling a
//! developer that the release they are ahead of is newer than their working
//! tree would be worse than saying nothing.
//!
//! That last paragraph was also, until TASK-037, a description of every build
//! anybody had ever downloaded. Only the draft lane set the variable; gate 5,
//! which builds what a `main` release publishes, and the prerelease lane, which
//! builds every alpha, both left it unset. So a released Spagitty reported
//! itself as a development build and could never say a newer one existed — the
//! feature shipped, passed its tests, and did nothing. Nothing in this file was
//! wrong, which is why nothing in this file caught it.

use serde::Serialize;
use serde_json::Value;

use crate::forge::http;
use crate::{Error, Result};

/// Where the release list is read from.
///
/// The project's own repository, hard-coded. This is not a setting: a check for
/// a newer Spagitty that could be pointed somewhere else is a way to hand
/// somebody a different program.
///
/// Spelled the way the repository is spelled. GitHub resolves an owner login
/// case-insensitively, so `Spa-git-ty` — which this said until TASK-037 — did
/// answer; it was still a path this project does not have, and the AppImage's
/// embedded update source names the same repository in the same letters.
const RELEASES: &str = "https://api.github.com/repos/spa-git-ty/spagitty/releases/latest";

const HOST: &str = "api.github.com";

/// Which kind of build is asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    /// Built by the release workflow, and it knows its own tag.
    Released,
    /// Built by a person. It has no tag, so it cannot be behind one.
    Development,
}

/// What the check found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub channel: Channel,
    /// The tag this build was cut as, or `None` for a development build.
    pub current: Option<String>,
    /// The newest tag the project has published.
    pub latest: String,
    /// True when [`latest`](Self::latest) is something other than this build.
    pub newer: bool,
    /// Where a person goes to get it.
    pub url: String,
}

/// The tag this build was cut as.
///
/// `option_env!` rather than `env!`: absent is the ordinary case for anybody
/// who compiled it themselves, and it must not be a build error.
pub fn current_release() -> Option<&'static str> {
    option_env!("SPAGITTY_RELEASE")
}

/// Ask the project whether there is a newer release.
pub fn check() -> Result<Update> {
    let response = http::get_json(RELEASES, "", HOST)?;

    if response.status < 200 || response.status >= 300 {
        return Err(crate::forge::status_error(
            HOST,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    read(&response.body, current_release())
}

/// Turn the release endpoint's answer into a verdict. Makes no request.
pub fn read(body: &str, current: Option<&str>) -> Result<Update> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: HOST.into(),
        detail: "sent something that is not JSON".into(),
    })?;

    let latest = json["tag_name"]
        .as_str()
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| Error::Forge {
            host: HOST.into(),
            detail: "did not name a latest release".into(),
        })?
        .to_string();

    // The API gives a page for the release; fall back to the releases index
    // rather than building a URL out of the tag, which would guess at a scheme
    // the host is free to change.
    let url = json["html_url"]
        .as_str()
        .filter(|url| url.starts_with("https://"))
        .unwrap_or("https://github.com/spa-git-ty/spagitty/releases")
        .to_string();

    let channel = match current {
        Some(_) => Channel::Released,
        None => Channel::Development,
    };

    Ok(Update {
        channel,
        current: current.map(str::to_string),
        // A development build is never behind. It has no tag to be behind, and
        // it is usually ahead of every one of them.
        newer: matches!(channel, Channel::Released) && current != Some(latest.as_str()),
        latest,
        url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(tag: &str) -> String {
        serde_json::json!({
            "tag_name": tag,
            "html_url": format!("https://github.com/spa-git-ty/spagitty/releases/tag/{tag}")
        })
        .to_string()
    }

    #[test]
    fn a_build_older_than_the_latest_release_is_told_so() {
        let found = read(&answer("v0.1.0-preview.4"), Some("v0.1.0-preview.2")).unwrap();

        assert!(found.newer);
        assert_eq!(found.latest, "v0.1.0-preview.4");
        assert_eq!(found.current.as_deref(), Some("v0.1.0-preview.2"));
        assert_eq!(found.channel, Channel::Released);
        assert!(found.url.ends_with("v0.1.0-preview.4"));
    }

    #[test]
    fn a_build_that_is_the_latest_release_is_left_alone() {
        let found = read(&answer("v0.1.0-preview.2"), Some("v0.1.0-preview.2")).unwrap();

        assert!(!found.newer);
    }

    #[test]
    fn a_development_build_is_never_out_of_date() {
        // It has no tag to be behind, and it is usually ahead of all of them.
        // Telling somebody their working tree is older than the release they
        // are about to cut from it would be worse than saying nothing.
        let found = read(&answer("v0.1.0-preview.9"), None).unwrap();

        assert_eq!(found.channel, Channel::Development);
        assert!(!found.newer);
        assert_eq!(found.current, None);
        // It still reports what the latest is, because that is worth knowing.
        assert_eq!(found.latest, "v0.1.0-preview.9");
    }

    #[test]
    fn the_comparison_is_the_tag_and_not_the_version_number() {
        // `Cargo.toml` says 0.1.0 across every preview, so a version comparison
        // would report "up to date" forever. This is the whole reason the tag
        // is baked in.
        let stale = read(&answer("v0.1.0-preview.7"), Some("v0.1.0-preview.1")).unwrap();

        assert!(
            stale.newer,
            "0.1.0 == 0.1.0, and yet it is four releases behind"
        );
    }

    #[test]
    fn a_release_with_no_tag_is_an_error_rather_than_an_empty_answer() {
        let empty = read(r#"{"tag_name": ""}"#, Some("v0.1.0-preview.1"));
        let missing = read(r#"{}"#, Some("v0.1.0-preview.1"));

        assert!(matches!(empty, Err(Error::Forge { .. })));
        assert!(matches!(missing, Err(Error::Forge { .. })));
    }

    #[test]
    fn a_body_that_is_not_json_is_reported_rather_than_panicked_on() {
        assert!(matches!(
            read("<html>maintenance</html>", None),
            Err(Error::Forge { .. })
        ));
    }

    #[test]
    fn a_url_the_host_did_not_send_falls_back_to_the_releases_page() {
        // Rather than building one out of the tag, which guesses at a scheme
        // the host is free to change.
        let body = serde_json::json!({ "tag_name": "v9.9.9" }).to_string();
        let found = read(&body, Some("v0.1.0-preview.1")).unwrap();

        assert_eq!(found.url, "https://github.com/spa-git-ty/spagitty/releases");
    }

    #[test]
    fn a_url_that_is_not_https_is_refused_in_favour_of_the_one_we_know() {
        // The answer is used to open a browser, and a `javascript:` or `file:`
        // URL from a host is not something to hand to one.
        for hostile in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "http://example.com",
        ] {
            let body = serde_json::json!({ "tag_name": "v9.9.9", "html_url": hostile }).to_string();
            let found = read(&body, None).unwrap();

            assert_eq!(
                found.url, "https://github.com/spa-git-ty/spagitty/releases",
                "for {hostile}"
            );
        }
    }

    #[test]
    fn the_endpoint_is_this_project_and_is_not_configurable() {
        // A check for a newer Spagitty that could be pointed elsewhere is a way
        // to hand somebody a different program.
        assert!(RELEASES.starts_with("https://api.github.com/repos/spa-git-ty/spagitty/"));
    }

    #[test]
    fn the_endpoint_is_spelled_the_way_the_repository_is_spelled() {
        // Not pedantry about a host that happens to be case-insensitive. The
        // AppImage carries an update source naming the same owner and
        // repository (TASK-037), and `tools/appimage-update.test.ts` holds the
        // two together — two spellings of one project is how they drift apart.
        assert_eq!(RELEASES, RELEASES.to_lowercase());
    }
}

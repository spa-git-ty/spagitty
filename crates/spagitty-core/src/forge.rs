// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading pull requests from the service that hosts a repository (FEAT-017).
//!
//! # This is the only part of Spagitty that talks to the network
//!
//! Everything else reads the disk. The All repositories screen promises that
//! repositories are read from disk and nothing is uploaded, and that promise
//! survives this module because of what is *not* here: no telemetry, no
//! repository contents, no paths, no commit messages. What leaves the machine
//! is an HTTPS request to a host the user connected themselves, carrying a
//! token they issued, asking for pull requests they can already see in a
//! browser.
//!
//! [`http`] is the one place a request is made. There is exactly one, for the
//! same reason [`crate::shell`] is the one place a process is spawned: so that
//! "what does this application send, and where" has a single answer somebody
//! can read in an afternoon.
//!
//! # Read-only, by decision
//!
//! Nothing here approves, merges, comments or closes. The author's decision,
//! recorded in the item: the smallest privacy surface that still answers the
//! question the Pull requests screen asks, and every write would need its own
//! confirmation and its own failure story.
//!
//! # Host-agnostic in the interface, specific underneath
//!
//! The UI's vocabulary never names a host — that is a design rule of the
//! project, not a detail of this module. [`PullRequest`] is the shape the
//! screen renders and has no GitHub in it. [`github`] maps one host's JSON onto
//! it. A second host is a second module and a second arm of [`Kind`], and
//! nothing above this line changes.

pub mod bitbucket;
pub mod github;
pub mod gitlab;
pub mod http;
pub mod keychain;
pub mod review;
pub use review::MergeMethod;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Which hosting service a remote points at.
///
/// An enum rather than a string: the set of hosts Spagitty knows how to read is
/// closed, and a URL pointing somewhere else is [`None`] rather than a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    GitHub,
    GitLab,
    Bitbucket,
}

impl Kind {
    /// The API root for this host.
    pub fn api_base(self, host: &str) -> String {
        match self {
            Kind::GitHub if host == "github.com" => "https://api.github.com".into(),
            Kind::GitHub => format!("https://{host}/api/v3"),
            Kind::GitLab if host == "gitlab.com" => "https://gitlab.com/api/v4".into(),
            Kind::GitLab => format!("https://{host}/api/v4"),
            Kind::Bitbucket => "https://api.bitbucket.org/2.0".into(),
        }
    }

    /// What the host calls itself, for a sentence a person reads.
    pub fn label(self) -> &'static str {
        match self {
            Kind::GitHub => "GitHub",
            Kind::GitLab => "GitLab",
            Kind::Bitbucket => "Bitbucket",
        }
    }
}

/// A repository on a host, as identified from a git remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub kind: Kind,
    /// The hostname, so an Enterprise installation is not mistaken for
    /// `github.com` and handed the wrong token.
    pub host: String,
    pub owner: String,
    pub name: String,
}

impl Repo {
    /// `owner/name`, which is how a person says it.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Where a pull request sits with its reviewers.
///
/// The four states the Pull requests screen already renders. Deliberately not
/// a host's own vocabulary: GitHub says `CHANGES_REQUESTED`, a different host
/// says something else, and the screen says "changes requested" either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewState {
    AwaitingReview,
    ChangesRequested,
    Approved,
    NoReviewers,
}

/// Whether the host's checks passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckState {
    Passing,
    Failing,
    Running,
}

/// One pull request, in the shape the screen renders.
///
/// FEAT-010 built this shape before there was anything behind it, and it has
/// not been changed to suit a host — the mapping went the other way, which is
/// what keeps the vocabulary host-agnostic.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    /// The host's own identifier, whatever form it takes.
    pub id: String,
    /// The number people say out loud: "#412".
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author_name: String,
    /// Seconds since the unix epoch.
    pub updated: i64,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub review: ReviewState,
    /// Null when the host runs no checks on it.
    pub checks: Option<CheckState>,
    /// True when this one is waiting on the person using Spagitty.
    pub needs_you: bool,
    /// Why it needs you, in one line, when it does.
    pub needs_you_because: Option<String>,
    pub changed_files: u64,
    pub added: u64,
    pub removed: u64,
    /// Null when the host has not said.
    pub mergeable: Option<bool>,
}

/// A connected account: a host, and who the token belongs to.
///
/// **The token is not in here.** It lives in the OS keychain and is fetched at
/// the moment of a request. A struct that carried it would be one `Serialize`
/// away from a token in a log line or a config file, and the item is explicit
/// that it never goes in one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub kind: Kind,
    pub host: String,
    /// The login the token authenticates as, read back from the host when it
    /// was connected. Shown so a person can tell two accounts apart.
    pub user: String,
}

/// Which repository a remote URL points at, or `None` for a host Spagitty does
/// not read.
///
/// Both forms git writes are accepted, because both are what people have:
///
/// ```text
/// git@github.com:owner/repo.git
/// ssh://git@github.com/owner/repo.git
/// https://github.com/owner/repo.git
/// https://user@github.com/owner/repo
/// ```
///
/// A URL that parses but names an unknown host is `None` rather than an error.
/// Not every remote is a forge — plenty are a path on a NAS — and a repository
/// with one of those is not misconfigured.
pub fn identify(url: &str) -> Option<Repo> {
    let url = url.trim();
    let (host, path) = split_host_and_path(url)?;

    // Strip a userinfo prefix: `git@github.com` and `user@github.com` are the
    // same host, and the name before the `@` is not part of it.
    let host = host.rsplit('@').next()?.to_string();
    let host = host.split(':').next()?.to_lowercase();

    let kind = kind_of(&host)?;

    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);

    // `owner/name`, and nothing after it. A URL with more segments is not a
    // repository root and is not something to guess at.
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    if parts.next().is_some() || owner.is_empty() || name.is_empty() {
        return None;
    }

    Some(Repo {
        kind,
        host,
        owner,
        name,
    })
}

/// The host part and the path part of any of the URL forms git accepts.
fn split_host_and_path(url: &str) -> Option<(&str, &str)> {
    // scp-like: `git@host:owner/repo.git`. Told from a URL by having no `://`.
    if !url.contains("://") {
        let (host, path) = url.split_once(':')?;
        return Some((host, path));
    }

    let (scheme, rest) = url.split_once("://")?;
    // `file://` and friends are not a forge and never will be.
    if !matches!(scheme, "https" | "http" | "ssh" | "git") {
        return None;
    }
    rest.split_once('/')
}

/// Which service a hostname belongs to.
///
/// `github.com` by name, and anything whose hostname starts with `github.` —
/// which is what a GitHub Enterprise installation is usually called — so an
/// enterprise host works without being configured. Anything else is `None`,
/// and connecting it is a decision a person makes rather than a guess this
/// function makes.
///
/// Public because connecting an account asks it too (BUG-043): a token for a
/// `gitlab.` host is proved against GitLab, not against GitHub.
pub fn kind_of(host: &str) -> Option<Kind> {
    if host == "github.com" || host.starts_with("github.") {
        return Some(Kind::GitHub);
    }
    if host == "gitlab.com" || host.starts_with("gitlab.") {
        return Some(Kind::GitLab);
    }
    if host == "bitbucket.org" || host.starts_with("bitbucket.") {
        return Some(Kind::Bitbucket);
    }
    None
}

/// The pull requests for `repo`, as the account for its host sees them.
///
/// `me` is the login the token belongs to; it decides `needs_you`, which is the
/// whole ordering the screen is built around.
pub fn pull_requests(repo: &Repo, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    match repo.kind {
        Kind::GitHub => github::pull_requests(repo, token, me),
        Kind::GitLab => gitlab::pull_requests(repo, token, me),
        Kind::Bitbucket => bitbucket::pull_requests(repo, token, me),
    }
}

/// Create a pull request on the host (FEAT-070).
pub fn create_pull_request(
    repo: &Repo,
    token: &str,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
    draft: bool,
) -> Result<PullRequest> {
    match repo.kind {
        Kind::GitHub => github::create_pull_request(repo, token, title, body, head, base, draft),
        Kind::GitLab => gitlab::create_merge_request(repo, token, title, body, head, base, draft),
        Kind::Bitbucket => bitbucket::create_pull_request(repo, token, title, body, head, base),
    }
}

/// Who a token belongs to, asked of the host.
pub fn whoami(kind: Kind, host: &str, token: &str) -> Result<String> {
    match kind {
        Kind::GitHub => github::whoami(host, token),
        Kind::GitLab => gitlab::whoami(host, token),
        Kind::Bitbucket => bitbucket::whoami(host, token),
    }
}

/// The remote a forge would be read from, by name.
///
/// `origin` when there is one, and the only remote when there is exactly one
/// under another name. More than one and no `origin` is ambiguous, and this
/// returns `None` rather than picking — reading somebody's fork's pull requests
/// because it sorted first is worse than saying nothing.
pub fn forge_remote(remotes: &[(String, String)]) -> Option<&(String, String)> {
    if let Some(origin) = remotes.iter().find(|(name, _)| name == "origin") {
        return Some(origin);
    }
    match remotes {
        [only] => Some(only),
        _ => None,
    }
}

/// Which repository this git repository's remotes point at.
pub fn identify_repo(repo: &gix::Repository) -> Result<Option<Repo>> {
    let remotes = crate::remotes::remotes(repo)
        .into_iter()
        .map(|remote| (remote.name, remote.url))
        .collect::<Vec<_>>();

    Ok(forge_remote(&remotes).and_then(|(_, url)| identify(url)))
}

/// Turn a host's HTTP status into an error that says which problem it is.
///
/// The item asks for offline and rate-limited behaviour that says which one it
/// is, and this is where that distinction is made rather than at the screen.
/// A screen deciding what a 403 meant would be a screen guessing.
pub(crate) fn status_error(
    host: &str,
    status: u16,
    body: &str,
    retry_after: Option<&str>,
) -> Error {
    match status {
        401 => Error::ForgeUnauthorized {
            host: host.to_string(),
            detail: host_message(body).unwrap_or_else(|| {
                "the token was refused. It may have expired or been revoked.".into()
            }),
        },
        // GitHub answers a spent rate limit with 403 or 429, and says so in the
        // body or the headers. A 403 that is *not* about the rate limit is a
        // permission problem, and reporting it as rate limiting would send the
        // reader to wait for something that is never going to change.
        403 | 429 if is_rate_limit(body, retry_after) => Error::ForgeRateLimited {
            host: host.to_string(),
            when: retry_after
                .map(|seconds| format!("in {seconds} seconds"))
                .unwrap_or_else(|| "shortly".into()),
        },
        403 => Error::ForgeUnauthorized {
            host: host.to_string(),
            detail: host_message(body)
                .unwrap_or_else(|| "the token does not have access to this repository.".into()),
        },
        404 => Error::Forge {
            host: host.to_string(),
            detail: host_message(body)
                .unwrap_or_else(|| "no such repository, or the token cannot see it.".into()),
        },
        422 => Error::Forge {
            host: host.to_string(),
            detail: host_message(body).unwrap_or_else(|| "the request was rejected (422)".into()),
        },
        other => Error::Forge {
            host: host.to_string(),
            detail: host_message(body).unwrap_or_else(|| format!("responded {other}")),
        },
    }
}

fn host_message(body: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(msg) = json["message"].as_str() {
        if let Some(errors) = json["errors"].as_array() {
            let error_details: Vec<String> = errors
                .iter()
                .filter_map(|e| {
                    e.as_str()
                        .map(str::to_string)
                        .or_else(|| e["message"].as_str().map(str::to_string))
                })
                .collect();
            if !error_details.is_empty() {
                return Some(format!("{msg}: {}", error_details.join("; ")));
            }
        }
        return Some(msg.to_string());
    }
    None
}

fn is_rate_limit(body: &str, retry_after: Option<&str>) -> bool {
    retry_after.is_some() || body.to_lowercase().contains("rate limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scp_form_git_writes_for_an_ssh_remote_is_understood() {
        // The single most common remote URL in existence.
        let repo = identify("git@github.com:owner/repo.git").expect("a repository");

        assert_eq!(repo.kind, Kind::GitHub);
        assert_eq!(repo.host, "github.com");
        assert_eq!(repo.owner, "owner");
        assert_eq!(repo.name, "repo");
        assert_eq!(repo.slug(), "owner/repo");
    }

    #[test]
    fn every_other_form_git_accepts_is_understood_too() {
        for url in [
            "ssh://git@github.com/owner/repo.git",
            "https://github.com/owner/repo.git",
            "https://github.com/owner/repo",
            "https://someone@github.com/owner/repo.git",
            "git://github.com/owner/repo.git",
            "  https://github.com/owner/repo.git  ",
        ] {
            let repo = identify(url).unwrap_or_else(|| panic!("could not read {url}"));
            assert_eq!(repo.slug(), "owner/repo", "for {url}");
            assert_eq!(repo.host, "github.com", "for {url}");
        }
    }

    #[test]
    fn the_host_is_compared_without_case() {
        assert_eq!(
            identify("git@GitHub.com:owner/repo.git").unwrap().host,
            "github.com"
        );
    }

    #[test]
    fn a_repository_name_that_ends_in_git_keeps_its_name() {
        // `.git` is a suffix on the URL, not part of the repository's name —
        // but a repository can also genuinely be called `something.git`, and
        // only one `.git` comes off.
        assert_eq!(
            identify("git@github.com:owner/repo.git.git").unwrap().name,
            "repo.git"
        );
        assert_eq!(
            identify("git@github.com:owner/dotgit.git").unwrap().name,
            "dotgit"
        );
    }

    #[test]
    fn a_github_enterprise_host_is_read_as_github() {
        let repo = identify("git@github.example.com:team/thing.git").expect("a repository");

        assert_eq!(repo.kind, Kind::GitHub);
        assert_eq!(repo.host, "github.example.com");
        assert_eq!(
            repo.kind.api_base(&repo.host),
            "https://github.example.com/api/v3",
            "an enterprise installation answers under /api/v3, not at api.<host>"
        );
    }

    #[test]
    fn github_dot_com_is_answered_by_its_own_api_host() {
        assert_eq!(
            Kind::GitHub.api_base("github.com"),
            "https://api.github.com"
        );
    }

    #[test]
    fn a_remote_that_is_not_a_forge_is_not_an_error() {
        // Plenty of remotes are a path on a NAS, and a repository with one is
        // not misconfigured. Nothing to read is not the same as something wrong.
        for url in [
            "/srv/git/repo.git",
            "file:///srv/git/repo.git",
            "git@customhost.invalid:owner/repo.git",
            "https://example.com/owner/repo.git",
            "",
        ] {
            assert!(identify(url).is_none(), "expected nothing for {url}");
        }
    }

    #[test]
    fn a_self_hosted_gitlab_is_known_by_its_hostname() {
        // The host BUG-043 was raised against: a GitLab under a company domain,
        // named `gitlab.` like the enterprise GitHub rule expects.
        assert_eq!(
            kind_of("gitlab.apps.ocp-nonprod-01.hodomain.local"),
            Some(Kind::GitLab)
        );
        assert_eq!(kind_of("github.com"), Some(Kind::GitHub));
        assert_eq!(kind_of("git.example.com"), None);
    }

    #[test]
    fn gitlab_and_bitbucket_remotes_are_identified() {
        let gitlab = identify("git@gitlab.com:owner/repo.git").expect("gitlab repo");
        assert_eq!(gitlab.kind, Kind::GitLab);
        assert_eq!(gitlab.host, "gitlab.com");
        assert_eq!(gitlab.owner, "owner");
        assert_eq!(gitlab.name, "repo");

        let bitbucket = identify("https://bitbucket.org/team/project.git").expect("bitbucket repo");
        assert_eq!(bitbucket.kind, Kind::Bitbucket);
        assert_eq!(bitbucket.host, "bitbucket.org");
        assert_eq!(bitbucket.owner, "team");
        assert_eq!(bitbucket.name, "project");
    }

    #[test]
    fn a_url_that_is_not_a_repository_root_is_refused_rather_than_guessed_at() {
        // A tree or blob URL pasted as a remote. Two segments is a repository;
        // anything else is a page about one.
        assert!(identify("https://github.com/owner/repo/tree/main").is_none());
        assert!(identify("https://github.com/owner").is_none());
        assert!(identify("git@github.com:owner").is_none());
    }

    fn remotes(of: &[(&str, &str)]) -> Vec<(String, String)> {
        of.iter()
            .map(|(name, url)| (name.to_string(), url.to_string()))
            .collect()
    }

    #[test]
    fn origin_is_the_remote_a_forge_is_read_from() {
        let list = remotes(&[
            ("upstream", "git@github.com:upstream/repo.git"),
            ("origin", "git@github.com:me/repo.git"),
        ]);

        assert_eq!(forge_remote(&list).unwrap().0, "origin");
    }

    #[test]
    fn a_single_remote_under_another_name_is_used() {
        let list = remotes(&[("fork", "git@github.com:me/repo.git")]);

        assert_eq!(forge_remote(&list).unwrap().0, "fork");
    }

    #[test]
    fn several_remotes_and_no_origin_is_answered_with_nothing() {
        // Reading somebody's fork's pull requests because it sorted first is
        // worse than saying nothing.
        let list = remotes(&[
            ("upstream", "git@github.com:upstream/repo.git"),
            ("fork", "git@github.com:me/repo.git"),
        ]);

        assert!(forge_remote(&list).is_none());
        assert!(forge_remote(&[]).is_none());
    }

    #[test]
    fn a_refused_token_is_told_apart_from_a_repository_it_cannot_see() {
        assert!(matches!(
            status_error("github.com", 401, "", None),
            Error::ForgeUnauthorized { .. }
        ));
        assert!(matches!(
            status_error("github.com", 404, "", None),
            Error::Forge { .. }
        ));
    }

    #[test]
    fn a_spent_rate_limit_is_told_apart_from_a_permission_problem() {
        // Both arrive as 403. Reporting a permission problem as rate limiting
        // sends the reader away to wait for something that will never change.
        let limited = status_error("github.com", 403, "API rate limit exceeded", None);
        let forbidden = status_error("github.com", 403, "Must have admin rights", None);

        assert!(matches!(limited, Error::ForgeRateLimited { .. }));
        assert!(matches!(forbidden, Error::ForgeUnauthorized { .. }));
    }

    #[test]
    fn a_retry_after_header_is_enough_to_call_it_rate_limiting_and_it_says_when() {
        match status_error("github.com", 429, "", Some("60")) {
            Error::ForgeRateLimited { when, .. } => assert!(when.contains("60")),
            other => panic!("expected rate limiting, got {other:?}"),
        }
    }

    #[test]
    fn a_host_that_says_nothing_useful_still_names_the_status() {
        match status_error("github.com", 500, "", None) {
            Error::Forge { detail, .. } => assert!(detail.contains("500")),
            other => panic!("expected a forge error, got {other:?}"),
        }
    }
}

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
pub mod snapshot;
pub mod watch;
pub use review::MergeMethod;
pub use review::ReviewVerdict;

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

    /// The slug as one URL path segment, every reserved character escaped
    /// (FEAT-088).
    ///
    /// GitLab addresses a project as `:id`, which is its number or its whole
    /// path URL-encoded — `group%2Fsub%2Fproject`. Escaping only the one slash
    /// between owner and name, as this used to, sends a nested group's inner
    /// slashes raw, and GitLab answers 404.
    pub fn encoded_slug(&self) -> String {
        encode_segment(&self.slug())
    }
}

/// `text` percent-encoded so it is one path segment: everything but RFC 3986's
/// unreserved characters is escaped.
pub fn encode_segment(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Where a pull request sits with its reviewers.
///
/// The four states the Pull requests screen already renders. Deliberately not
/// a host's own vocabulary: GitHub says `CHANGES_REQUESTED`, a different host
/// says something else, and the screen says "changes requested" either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewState {
    AwaitingReview,
    ChangesRequested,
    Approved,
    #[default]
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
///
/// The last six fields were added for the Review screen (FEAT-087) and default
/// to nothing, so a host that cannot answer them reads as "none" rather than
/// as a guess.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
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
    /// The commit the head branch points at. Empty when the host did not say.
    pub head_sha: String,
    /// The person using Spagitty is a requested reviewer — the plain "needs
    /// you", apart from `needs_you`'s second reason (their own pull request
    /// with changes requested), which is not a review to do.
    pub review_requested: bool,
    /// Review threads not yet resolved, and resolved ones.
    pub open_threads: u32,
    pub resolved_threads: u32,
    /// Open threads the person started where somebody else spoke last: the
    /// author answered them.
    pub replies_to_you: u32,
    /// `owner/name` when the row came from a search across repositories rather
    /// than from one repository's list.
    pub repository: Option<String>,
    /// The person's own latest review, when they have left one and the host
    /// says (BUG-064). None from a host that does not.
    pub your_review: Option<YourReview>,
}

/// What the person last said on a pull request, and on which commit: a review
/// left on an older head is one the author has pushed past (BUG-064).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YourReview {
    pub verdict: ReviewVerdict,
    /// The head the review was left on. Empty when the host did not say.
    pub sha: String,
}

/// What the Review inbox learns about a pull request after the list (FEAT-088).
///
/// GitHub's list already carries all of it; GitLab's carries none, and the
/// inbox asks for it a few merge requests at a time. Merged into the row by
/// `number` on the screen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSummary {
    pub number: u64,
    pub checks: Option<CheckState>,
    pub open_threads: u32,
    pub resolved_threads: u32,
    pub replies_to_you: u32,
}

/// Checks and thread counts for `numbers` on `repo` (FEAT-088). Empty for a
/// host whose list already says.
pub fn review_summaries(repo: &Repo, token: &str, me: &str, numbers: &[u64]) -> Vec<ReviewSummary> {
    match repo.kind {
        Kind::GitLab => gitlab::review_summaries(repo, token, me, numbers),
        Kind::GitHub | Kind::Bitbucket => Vec::new(),
    }
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
    identify_with(url, &[])
}

/// [`identify`], also trusting the hosts of connected accounts (FEAT-088).
///
/// A self-hosted GitLab is often not called `gitlab.` anything. Once somebody
/// has connected an account for it and said it is GitLab, a remote on that
/// host is that kind; nothing else is guessed.
pub fn identify_with(url: &str, known: &[Account]) -> Option<Repo> {
    let url = url.trim();
    let (host, path) = split_host_and_path(url)?;

    // Strip a userinfo prefix: `git@github.com` and `user@github.com` are the
    // same host, and the name before the `@` is not part of it.
    let host = host.rsplit('@').next()?.to_string();
    let host = host.split(':').next()?.to_lowercase();

    let kind = kind_of(&host).or_else(|| {
        known
            .iter()
            .find(|account| account.host == host)
            .map(|account| account.kind)
    })?;

    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    // `owner/name`, and nothing after it. A URL with more segments is not a
    // repository root and is not something to guess at — except on GitLab,
    // where groups nest (`group/sub/project`) and everything before the last
    // segment is the namespace. A `-` segment is GitLab's mark for a page
    // about a project (`/-/tree/main`), never part of a project's path.
    let (owner, name) = match (kind, parts.as_slice()) {
        (_, [owner, name]) => (owner.to_string(), name.to_string()),
        (Kind::GitLab, [namespace @ .., name]) if namespace.len() >= 2 && !parts.contains(&"-") => {
            (namespace.join("/"), name.to_string())
        }
        _ => return None,
    };

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

/// The open pull requests on `host` that involve `me`, across every repository
/// the token can see (FEAT-087): the Review screen's "All my repos".
///
/// Each row carries its `repository`, since the rows no longer share one.
pub fn involved_pull_requests(
    kind: Kind,
    host: &str,
    token: &str,
    me: &str,
) -> Result<Vec<PullRequest>> {
    match kind {
        Kind::GitHub => github::involved_pull_requests(host, token, me),
        Kind::GitLab => gitlab::involved_merge_requests(host, token, me),
        Kind::Bitbucket => Err(Error::Forge {
            host: host.to_string(),
            detail: "Bitbucket has no search across repositories".into(),
        }),
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

/// Which kind of forge `host` is, and who the token belongs to there
/// (FEAT-088).
///
/// A host whose name says (`github.`, `gitlab.`) is asked as that. One whose
/// name does not — a self-hosted GitLab called `code.example.com` — is asked
/// GitLab's question first: a GitLab answers `/api/v4/user` even to a refused
/// token, with a 401, so a refusal there means "GitLab, wrong token" and is
/// reported as that. Anything else is asked as `fallback`.
pub fn identify_account(host: &str, token: &str, fallback: Kind) -> Result<(Kind, String)> {
    if let Some(kind) = kind_of(host) {
        return Ok((kind, whoami(kind, host, token)?));
    }
    match gitlab::whoami(host, token) {
        Ok(user) => Ok((Kind::GitLab, user)),
        Err(refused @ Error::ForgeUnauthorized { .. }) => Err(refused),
        Err(_) => Ok((fallback, whoami(fallback, host, token)?)),
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
    identify_repo_with(repo, &[])
}

/// [`identify_repo`], also trusting the hosts of connected accounts (FEAT-088).
pub fn identify_repo_with(repo: &gix::Repository, known: &[Account]) -> Result<Option<Repo>> {
    let remotes = crate::remotes::remotes(repo)
        .into_iter()
        .map(|remote| (remote.name, remote.url))
        .collect::<Vec<_>>();

    Ok(forge_remote(&remotes).and_then(|(_, url)| identify_with(url, known)))
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

    // FEAT-088 — GitLab as its API describes it.

    #[test]
    fn a_gitlab_project_in_a_nested_group_is_identified() {
        for url in [
            "git@gitlab.example.com:team/backend/payments.git",
            "https://gitlab.example.com/team/backend/payments.git",
            "ssh://git@gitlab.example.com:2222/team/backend/payments.git",
        ] {
            let repo = identify(url).unwrap_or_else(|| panic!("could not read {url}"));
            assert_eq!(repo.kind, Kind::GitLab, "{url}");
            assert_eq!(repo.owner, "team/backend", "{url}");
            assert_eq!(repo.name, "payments", "{url}");
            assert_eq!(repo.slug(), "team/backend/payments");
        }
    }

    #[test]
    fn a_gitlab_page_url_is_not_mistaken_for_a_nested_project() {
        assert!(identify("https://gitlab.com/team/app/-/tree/main").is_none());
        assert!(identify("https://gitlab.com/team").is_none());
    }

    #[test]
    fn only_gitlab_nests() {
        assert!(identify("https://github.com/a/b/c").is_none());
        assert!(identify("https://bitbucket.org/a/b/c").is_none());
    }

    #[test]
    fn a_project_path_is_one_segment_with_every_slash_escaped() {
        let repo = identify("git@gitlab.example.com:team/backend/payments.git").unwrap();
        assert_eq!(repo.encoded_slug(), "team%2Fbackend%2Fpayments");
        assert_eq!(encode_segment("a b+c.d_e~f-g"), "a%20b%2Bc.d_e~f-g");
    }

    #[test]
    fn a_connected_account_vouches_for_a_host_with_another_name() {
        let account = Account {
            kind: Kind::GitLab,
            host: "code.example.com".into(),
            user: "me".into(),
        };
        let url = "git@code.example.com:team/sub/app.git";

        assert!(identify(url).is_none());
        let repo = identify_with(url, std::slice::from_ref(&account)).expect("a repository");
        assert_eq!(repo.kind, Kind::GitLab);
        assert_eq!(repo.host, "code.example.com");
        assert_eq!(repo.owner, "team/sub");
        // A host's own name still wins over an account's say-so.
        let github = Account {
            kind: Kind::GitLab,
            host: "github.com".into(),
            user: "me".into(),
        };
        assert_eq!(
            identify_with("git@github.com:a/b.git", &[github])
                .unwrap()
                .kind,
            Kind::GitHub
        );
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

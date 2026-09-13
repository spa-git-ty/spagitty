// SPDX-License-Identifier: GPL-3.0-or-later

//! The author's real picture, for the graph's nodes (FEAT-079).
//!
//! # This reverses a decision, on purpose
//!
//! [`crate::graph`]'s frontend counterpart — `portrait.ts` — argues at length
//! that Spagitty should *not* fetch author pictures: a request per author on
//! the most performance-sensitive screen in the application, a repository's
//! committer list handed to a third party, and faces that appear only when
//! there is a network. Every one of those arguments is still true, and the
//! generated face is still what the graph draws when this module returns
//! nothing.
//!
//! What changed is that a generated face is not who somebody is. A blob is a
//! good *disambiguator* — "these four rows are the same person" — and a poor
//! *identifier*: it does not answer "which person", which is the question
//! actually being asked when somebody looks at a node. So the picture is
//! fetched, and the three objections are answered rather than dismissed:
//!
//! - **One request per author, ever.** The answer is written to disk, and the
//!   absence of an answer is written too (see [`Cached::Missing`]). A second
//!   launch on the same repository makes no requests at all.
//! - **Nothing is asked that does not have to be.** The GitHub no-reply forms
//!   carry the account in the address itself, so those cost no lookup and leak
//!   nothing. An ordinary address is asked about once — of the repository's own
//!   forge, by one commit (see below) — and only then, if the forge knows no
//!   account for it, of Gravatar, as a hash.
//! - **A missing network is the ordinary case, not a failure.** Every path
//!   here returns `None` rather than an error, and `None` is the generated
//!   face. A repository opened on a train looks exactly like it did before this
//!   module existed.
//!
//! And it is a preference, defaulting to on, that turns the whole thing off.
//!
//! # Asking the forge who wrote a commit (FEAT-081)
//!
//! The first resolver never asked a host who an address belonged to, and the
//! runtime recording showed the cost: most people commit with an ordinary
//! address that has no Gravatar, so a whole repository of real GitHub accounts
//! drew initials. The reference client shows their pictures because it reads
//! the forge's commit metadata, where GitHub answers a commit with the account
//! it is associated with — login and picture URL.
//!
//! So an ordinary address in a repository whose remote is on GitHub is looked
//! up once, by one representative commit, and the account that comes back is
//! remembered per host and address. The request goes to the repository's own
//! host, carrying a commit id and a repository name that host already has; a
//! connected account's token is sent with it and with nothing else. The
//! picture itself is then fetched with no credential at all.
//!
//! # Not remembering a failure as an answer
//!
//! A miss is remembered for ninety days, which is right for "this address has no
//! picture" and badly wrong for "the network was down" — yet the first resolver
//! wrote both the same way, together with the redirect GitHub answers an older
//! no-reply address with, which the transport declines to follow. A single
//! offline launch hid every face for a quarter of a year. Only a definitive
//! not-found is a miss now; everything transient backs off, boundedly, and is
//! asked again. The cache's directory is versioned so the misses the old rule
//! wrote are not believed.
//!
//! # What leaves the machine
//!
//! For a GitHub no-reply address: a request to `avatars.githubusercontent.com`
//! or `github.com` for a picture, carrying an account name that is already
//! written in the repository's commits in plain text.
//!
//! For an ordinary address in a repository on GitHub: one request to that
//! repository's API for one of the author's commits, then a request for the
//! picture URL it names, on an image host from a fixed list.
//!
//! For any other address: a request to `gravatar.com` carrying the SHA-256 of
//! the lowercased address. That is the whole of it — no name, no repository, no
//! path, no other address. Gravatar cannot reverse the hash, but it can confirm
//! an address it already knows, so this is a real disclosure and it is the one
//! the Settings screen names.
//!
//! Addresses that are obviously local — no `@`, or a host with no dot in it —
//! never leave at all.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::forge::http::{self, Fetched, Response};
use crate::forge::{Kind, Repo};

/// The pixel size asked for.
///
/// The graph draws a node at up to about 18 CSS pixels and the author column
/// at the same, so 96 covers a 3x display with room to spare and is small
/// enough that a hundred of them is a few megabytes on disk. Hosts serve
/// whatever square is nearest.
pub const SIZE: u32 = 96;

/// What a picture is allowed to weigh.
///
/// A 96px avatar is a few kilobytes. A quarter of a megabyte is far past
/// anything legitimate and still small enough that a host answering with
/// something absurd cannot matter.
const MAX_BYTES: usize = 256 * 1024;

/// How long an answer stands before it is asked again.
///
/// People change their picture, and a client that never asked again would show
/// the old one until the cache was deleted by hand. Thirty days is long enough
/// that the fetch is invisible and short enough that a change is not
/// permanent.
const FRESH: Duration = Duration::from_secs(60 * 60 * 24 * 30);

/// A missing picture is remembered for longer than a found one.
///
/// The expensive case is an address with no picture anywhere: it costs a
/// request that returns 404, and re-asking it weekly for every author in a
/// large repository is most of the traffic this module could generate. Ninety
/// days, because somebody who has never had a Gravatar is unlikely to acquire
/// one on a schedule.
const FRESH_MISS: Duration = Duration::from_secs(60 * 60 * 24 * 90);

/// Where an author's picture can be found, and what to call them.
///
/// `Default` is "nowhere, and nothing to call them", which is the answer for
/// every address that must not be looked up — see [`source`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    /// The URL to fetch, when there is one worth fetching.
    pub url: Option<String>,
    /// The account name, when the address carries one.
    ///
    /// A GitHub no-reply address *is* the handle, written down. This is the
    /// only place a handle can come from without asking a host who an address
    /// belongs to, which is a question this module will not ask.
    pub handle: Option<String>,
}

/// Where to look for one address's picture.
///
/// Pure, and the part worth reading: everything this module discloses is
/// decided here, so "what does Spagitty send" is answered by one function with
/// a table of tests under it rather than by following a fetch.
pub fn source(email: &str) -> Source {
    let address = email.trim().to_lowercase();

    let Some((user, host)) = address.split_once('@') else {
        // Not an address. git allows it, and there is nowhere to look.
        return Source::default();
    };

    if user.is_empty() || !host.contains('.') {
        // `ada@localhost`, `ada@build-box`: a host with no dot is a name on
        // this network, and asking the internet about it would be telling the
        // internet about it.
        return Source::default();
    }

    // GitHub's no-reply addresses, which are what a person committing through
    // the web interface or with the privacy setting on writes. Both forms
    // carry the account, so both cost nothing to resolve and disclose nothing
    // that is not already in the commit.
    if let Some(handle) = github_noreply(user, host) {
        return Source {
            url: Some(github_avatar(user, &handle)),
            handle: Some(handle),
        };
    }

    // Anything else: Gravatar, by hash, with `d=404` so a "no picture here"
    // is a status rather than a generated image. The generated image is the
    // one thing this must not accept — Spagitty already has a generated image
    // and it is a better one, because it is the same on every machine and
    // needs no network.
    Source {
        url: Some(format!(
            "https://gravatar.com/avatar/{}?s={SIZE}&d=404",
            sha256_hex(address.as_bytes())
        )),
        handle: None,
    }
}

/// The account in a GitHub no-reply address, if that is what this is.
///
/// Two forms, both current: `octocat@users.noreply.github.com` from the older
/// setting, and `1234567+octocat@users.noreply.github.com` from the current
/// one, where the number is the account id.
fn github_noreply(user: &str, host: &str) -> Option<String> {
    if host != "users.noreply.github.com" {
        return None;
    }
    let handle = user.rsplit_once('+').map_or(user, |(_, name)| name);
    (!handle.is_empty()).then(|| handle.to_string())
}

/// The picture URL for a GitHub account.
///
/// By **id** when the address carries one, because an id is permanent and a
/// login is not: somebody who renames their account keeps every commit they
/// ever made, and a URL built from the old login would 404 for the rest of the
/// repository's life. The login form is the fallback for the older addresses
/// that have no id in them.
fn github_avatar(user: &str, handle: &str) -> String {
    match user.split_once('+') {
        Some((id, _)) if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) => {
            format!("https://avatars.githubusercontent.com/u/{id}?s={SIZE}&v=4")
        }
        _ => format!("https://github.com/{handle}.png?size={SIZE}"),
    }
}

/// The cache's record format, as the directory it lives in (FEAT-081).
///
/// Bumped when a record written under the old rules must not be believed. `v2`
/// is the first bump: the resolver before it wrote a ninety-day miss for a
/// redirect, a rate limit and a dropped connection alike, and there is no way
/// to tell those zero-byte files from a real not-found after the fact.
pub const CACHE_VERSION: &str = "v2";

/// The directory the current record format lives in, under the avatar root.
pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(CACHE_VERSION)
}

/// Delete the records the first format wrote straight into the root.
///
/// Not needed for correctness — nothing reads them — but they are a directory
/// full of hashes of email addresses, and leaving that behind after it stopped
/// meaning anything is not tidy. Only plain `.img` and `.miss` files directly
/// in `root` are touched.
pub fn retire_legacy(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let legacy = matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("img" | "miss")
        );
        if legacy && path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// How long the first transient failure waits before the address is asked
/// again, and the most any number of them can make it wait.
///
/// Doubling from five minutes to six hours: soon enough that a laptop coming
/// back online shows faces within the session, and slow enough that a rate
/// limit is not hammered by a repository of three hundred authors.
const BACKOFF_FIRST: Duration = Duration::from_secs(5 * 60);
const BACKOFF_MAX: Duration = Duration::from_secs(6 * 60 * 60);

/// How many redirects a picture request follows, each to a host on
/// [`image_host_allowed`]'s list. GitHub's login-form URL is one hop.
const REDIRECTS: usize = 3;

/// One of the author's commits, on the forge the repository lives on.
pub struct Commit<'a> {
    pub repo: &'a Repo,
    /// The commit's full id.
    pub id: &'a str,
    /// A connected account's token for `repo.host`, if there is one.
    ///
    /// A function rather than a value: reading the keychain is a blocking call
    /// into the platform's secret store, and nearly every lookup is answered
    /// from disk without needing it.
    pub token: &'a dyn Fn() -> Option<String>,
}

/// The requests this module makes, as a seam a test can stand in for.
///
/// Both go through [`http`], the application's one network module; nothing
/// here holds a transport of its own.
pub trait Net {
    /// A forge API read. `token` may be empty, which sends no credential.
    fn json(&self, url: &str, token: &str, host: &str) -> crate::Result<Response>;
    /// A public file, never with a credential.
    fn bytes(&self, url: &str, limit: usize) -> crate::Result<Fetched>;
}

/// The network, for real.
pub struct Live;

impl Net for Live {
    fn json(&self, url: &str, token: &str, host: &str) -> crate::Result<Response> {
        http::get_json(url, token, host)
    }

    fn bytes(&self, url: &str, limit: usize) -> crate::Result<Fetched> {
        http::get_bytes(url, limit)
    }
}

/// What is known about one author.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    /// The picture, as a `data:` URL ready to draw.
    pub picture: Option<String>,
    /// The account name, from the address or from the forge.
    pub handle: Option<String>,
    /// No answer yet, for a reason that may pass — ask again later.
    pub retry: bool,
}

/// What the cache holds for one address.
enum Cached {
    /// A picture, as the bytes that were served.
    Found(Vec<u8>),
    /// Asked, and the host said there is none. Worth remembering: it is what
    /// stops the expensive case from being asked again on every launch.
    Missing,
    /// Asked, and the asking failed in a way that may pass; still backing off.
    Waiting,
    /// Never asked, or asked long enough ago that the answer has expired.
    Unknown,
}

/// The three ways one step of a lookup can end.
enum Step<T> {
    Got(T),
    /// The authority said there is nothing — a 404, a commit with no account.
    Nothing,
    /// No answer that can be trusted: offline, rate limited, a server error, a
    /// redirect somewhere not on the list, a body that is not an image.
    Transient,
}

/// One author's picture, as a `data:` URL, fetching it if it is not held.
///
/// `data:` rather than a file path because that is what the webview's content
/// policy already allows for images, and because it makes the picture a value
/// the frontend owns rather than a second path into the cache directory that
/// would need its own scope and its own escaping rules.
///
/// `commit` is one commit by this author in the open repository, when there is
/// one; it is what lets a forge say which account an ordinary address is.
///
/// Never an error. A definitive absence is `picture: None`, which the caller
/// draws the generated face for; a transient one also sets `retry`.
pub fn picture(cache: &Path, email: &str, commit: Option<&Commit<'_>>, net: &dyn Net) -> Answer {
    let address = email.trim().to_lowercase();
    let source = source(&address);
    let mut handle = source.handle.clone();

    // An address that must not leave the machine does not, whatever forge the
    // repository is on: the rule is about the address, not the request.
    let Some(fallback) = source.url.clone() else {
        return Answer {
            handle,
            ..Answer::default()
        };
    };

    let forge = commit.filter(|commit| commit.repo.kind == Kind::GitHub);
    if let Some(who) = forge.and_then(|commit| read_who(cache, &commit.repo.host, &address)) {
        handle = handle.or(Some(who.0));
    }

    match read_cache(cache, &address) {
        Cached::Found(bytes) => {
            return Answer {
                picture: data_url(&bytes),
                handle,
                retry: false,
            }
        }
        Cached::Missing => {
            return Answer {
                handle,
                ..Answer::default()
            }
        }
        Cached::Waiting => {
            return Answer {
                handle,
                retry: true,
                ..Answer::default()
            }
        }
        Cached::Unknown => {}
    }

    let url = if fallback.starts_with("https://avatars.githubusercontent.com/u/") {
        // A numeric no-reply address names its account permanently: the cheap
        // first path, with nothing to ask anybody.
        fallback
    } else {
        match forge.map_or(Step::Nothing, |commit| {
            forge_account(cache, &address, commit, net)
        }) {
            Step::Got((login, url)) => {
                handle = handle.or(Some(login));
                url
            }
            Step::Nothing => fallback,
            Step::Transient => return waiting(cache, &address, handle),
        }
    };

    let forge_host = forge.map(|commit| commit.repo.host.as_str());
    match fetch_image(&url, forge_host, net) {
        Step::Got(bytes) => {
            write_cache(cache, &address, Some(&bytes));
            Answer {
                picture: data_url(&bytes),
                handle,
                retry: false,
            }
        }
        Step::Nothing => {
            write_cache(cache, &address, None);
            Answer {
                handle,
                ..Answer::default()
            }
        }
        Step::Transient => waiting(cache, &address, handle),
    }
}

/// A picture already on disk for an address, asking nothing.
///
/// For a lookup that should not reach the network on its own — an ordinary
/// address in a repository on GitHub, asked without one of its commits. Sending
/// it to Gravatar there would write a miss that later hides the account the
/// forge would have named (FEAT-081).
pub fn cached(cache: &Path, email: &str) -> Option<String> {
    match read_cache(cache, &email.trim().to_lowercase()) {
        Cached::Found(bytes) => data_url(&bytes),
        _ => None,
    }
}

/// Record a transient failure and say so.
fn waiting(cache: &Path, address: &str, handle: Option<String>) -> Answer {
    back_off(cache, address);
    Answer {
        handle,
        retry: true,
        ..Answer::default()
    }
}

fn data_url(bytes: &[u8]) -> Option<String> {
    let mime = kind(bytes)?;
    Some(format!("data:{mime};base64,{}", base64(bytes)))
}

/// The account the forge associates with one of this author's commits.
///
/// Remembered per host and address, so a repository makes one of these per
/// author rather than one per row — and the next repository on the same host
/// by the same person makes none.
fn forge_account(
    cache: &Path,
    address: &str,
    commit: &Commit<'_>,
    net: &dyn Net,
) -> Step<(String, String)> {
    let repo = commit.repo;
    if let Some(who) = read_who(cache, &repo.host, address) {
        return Step::Got(who);
    }

    // An id is interpolated into a URL; anything that is not one is not sent.
    let id = commit.id.trim();
    if id.len() < 7 || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Step::Nothing;
    }

    let url = format!(
        "{}/repos/{}/{}/commits/{id}",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name
    );
    let token = (commit.token)().unwrap_or_default();
    let Ok(response) = net.json(&url, &token, &repo.host) else {
        return Step::Transient;
    };

    match response.status {
        200..=299 => match account_of(&response.body) {
            Some((login, avatar)) => {
                let url = sized(&avatar);
                write_who(cache, &repo.host, address, &login, &url);
                Step::Got((login, url))
            }
            // A commit GitHub has no account for: the address is not on any
            // account, and Gravatar is the next place to look.
            None if response.body.trim_start().starts_with('{') => Step::Nothing,
            None => Step::Transient,
        },
        // No such commit here (not pushed, or a private repository this token
        // cannot see), or an id GitHub will not parse: nothing to learn from
        // the forge, which is not the same as the network failing.
        401 | 404 | 422 => Step::Nothing,
        _ => Step::Transient,
    }
}

/// `author.login` and `author.avatar_url` out of a commit response.
fn account_of(body: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let author = value.get("author")?;
    let login = author.get("login")?.as_str()?.trim();
    let avatar = author.get("avatar_url")?.as_str()?.trim();
    (!login.is_empty() && avatar.starts_with("https://"))
        .then(|| (login.to_string(), avatar.to_string()))
}

/// A picture URL from the forge, asking for [`SIZE`].
fn sized(url: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}s={SIZE}")
}

/// Fetch a picture, following a few redirects to hosts on the list.
///
/// A separate path from the forge's JSON client, which follows no redirect at
/// all and never will: that client carries a token. This one never does, so
/// following GitHub's `github.com/<login>.png` redirect to its image host
/// costs nothing — but only to hosts that serve avatars, so a redirect cannot
/// turn a picture request into a request to anywhere.
fn fetch_image(url: &str, forge_host: Option<&str>, net: &dyn Net) -> Step<Vec<u8>> {
    let mut url = url.to_string();
    for _ in 0..=REDIRECTS {
        if !image_host_allowed(&url, forge_host) {
            return Step::Transient;
        }
        let Ok(fetched) = net.bytes(&url, MAX_BYTES) else {
            return Step::Transient;
        };
        match fetched.status {
            200..=299 if kind(&fetched.body).is_some() => return Step::Got(fetched.body),
            // The authority on this address says it has no picture.
            404 | 410 => return Step::Nothing,
            301 | 302 | 303 | 307 | 308 => match fetched.location {
                Some(next) => url = absolute(&url, &next),
                None => return Step::Transient,
            },
            _ => return Step::Transient,
        }
    }
    Step::Transient
}

/// A `Location` resolved against the URL that sent it.
fn absolute(from: &str, location: &str) -> String {
    if location.starts_with("https://") || location.starts_with("http://") {
        return location.to_string();
    }
    let origin_end = from["https://".len()..]
        .find('/')
        .map_or(from.len(), |at| at + "https://".len());
    if location.starts_with('/') {
        format!("{}{location}", &from[..origin_end])
    } else {
        // Nothing GitHub or Gravatar sends; not guessed at.
        String::new()
    }
}

/// Whether a picture may be fetched from this URL.
///
/// HTTPS, no credentials in the authority, no port, and a host that serves
/// avatars: GitHub's two, Gravatar's, and — for GitHub Enterprise, which
/// serves pictures from itself — the repository's own forge host.
fn image_host_allowed(url: &str, forge_host: Option<&str>) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return false;
    }
    let host = authority.to_ascii_lowercase();
    const HOSTS: &[&str] = &[
        "avatars.githubusercontent.com",
        "github.com",
        "gravatar.com",
        "www.gravatar.com",
        "secure.gravatar.com",
    ];
    HOSTS.contains(&host.as_str())
        || forge_host.is_some_and(|forge| forge.eq_ignore_ascii_case(&host))
}

/// What an image's first bytes say it is.
///
/// The magic number rather than the `Content-Type` header, because the header
/// is the host's claim and the bytes are the fact — and because a `data:` URL
/// with the wrong type in it renders as nothing at all, silently. An
/// unrecognised body is not an image; a host that answered a picture request
/// with an HTML error page is the common way that happens.
fn kind(bytes: &[u8]) -> Option<&'static str> {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    const JPEG: &[u8] = &[0xff, 0xd8, 0xff];
    const GIF: &[u8] = b"GIF8";

    if bytes.starts_with(PNG) {
        Some("image/png")
    } else if bytes.starts_with(JPEG) {
        Some("image/jpeg")
    } else if bytes.starts_with(GIF) {
        Some("image/gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

/// The file an address's answer is kept in.
///
/// Named by the hash of the address, not by the address: a cache directory is
/// a place people look, and a directory listing that is a list of every email
/// address in every repository somebody has opened is a worse thing to leave
/// on a disk than the pictures are worth.
fn cache_file(cache: &Path, key: &str, extension: &str) -> PathBuf {
    let name = sha256_hex(key.trim().to_lowercase().as_bytes());
    cache.join(format!("{name}.{extension}"))
}

fn read_cache(cache: &Path, email: &str) -> Cached {
    let found = cache_file(cache, email, "img");
    if let Ok(meta) = std::fs::metadata(&found) {
        if fresh(&meta, FRESH) {
            if let Ok(bytes) = std::fs::read(&found) {
                return Cached::Found(bytes);
            }
        }
    }

    let missing = cache_file(cache, email, "miss");
    if let Ok(meta) = std::fs::metadata(&missing) {
        if fresh(&meta, FRESH_MISS) {
            return Cached::Missing;
        }
    }

    let wait = cache_file(cache, email, "wait");
    if let Ok(meta) = std::fs::metadata(&wait) {
        if fresh(&meta, backoff(attempts(&wait))) {
            return Cached::Waiting;
        }
    }

    Cached::Unknown
}

/// How many transient failures in a row a `.wait` record has counted.
fn attempts(wait: &Path) -> u32 {
    std::fs::read_to_string(wait)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// The wait after `attempts` failures: doubling, and bounded.
fn backoff(attempts: u32) -> Duration {
    let doublings = attempts.saturating_sub(1).min(16);
    BACKOFF_FIRST
        .saturating_mul(1 << doublings)
        .min(BACKOFF_MAX)
}

/// Count one more transient failure for an address.
fn back_off(cache: &Path, email: &str) {
    if std::fs::create_dir_all(cache).is_err() {
        return;
    }
    let wait = cache_file(cache, email, "wait");
    let next = attempts(&wait).saturating_add(1);
    let _ = std::fs::write(&wait, next.to_string());
}

/// The account remembered for an address on a forge host, while fresh.
fn read_who(cache: &Path, host: &str, address: &str) -> Option<(String, String)> {
    let file = cache_file(cache, &format!("{host}\n{address}"), "who");
    let meta = std::fs::metadata(&file).ok()?;
    if !fresh(&meta, FRESH) {
        return None;
    }
    let text = std::fs::read_to_string(&file).ok()?;
    let (login, url) = text.split_once('\n')?;
    (!login.is_empty() && url.starts_with("https://")).then(|| (login.to_string(), url.to_string()))
}

fn write_who(cache: &Path, host: &str, address: &str, login: &str, url: &str) {
    if std::fs::create_dir_all(cache).is_err() || login.contains('\n') || url.contains('\n') {
        return;
    }
    let file = cache_file(cache, &format!("{host}\n{address}"), "who");
    let _ = std::fs::write(file, format!("{login}\n{url}"));
}

/// Whether a cached answer is still young enough to use.
///
/// A file whose modification time cannot be read, or is in the future because
/// a clock moved, counts as stale: asking again costs one request and trusting
/// a nonsense timestamp costs a picture that never updates.
fn fresh(meta: &std::fs::Metadata, within: Duration) -> bool {
    meta.modified()
        .ok()
        .and_then(|at| SystemTime::now().duration_since(at).ok())
        .is_some_and(|age| age < within)
}

/// Record an answer, or the lack of one. A cache that cannot be written is not
/// an error — it costs a request next time, and nothing else.
fn write_cache(cache: &Path, email: &str, bytes: Option<&[u8]>) {
    if std::fs::create_dir_all(cache).is_err() {
        return;
    }

    // The records are exclusive: finding a picture for an address that had
    // none must not leave the miss behind to be found first next time, and a
    // settled answer ends whatever backing off came before it.
    let found = cache_file(cache, email, "img");
    let missing = cache_file(cache, email, "miss");
    let _ = std::fs::remove_file(cache_file(cache, email, "wait"));

    match bytes {
        Some(bytes) => {
            let _ = std::fs::remove_file(&missing);
            let _ = std::fs::write(&found, bytes);
        }
        None => {
            let _ = std::fs::remove_file(&found);
            let _ = std::fs::write(&missing, b"");
        }
    }
}

/// Forget every cached picture.
///
/// What "Clear" on the Settings screen does, and what turning the preference
/// off does — a picture that was fetched before somebody opted out should not
/// go on being shown from disk afterwards.
pub fn forget(cache: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(cache) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Standard base64, no line breaks, for a `data:` URL.
///
/// Written here rather than taken as a dependency: it is twenty lines with no
/// decisions in it, the crate it would replace is already in the tree only
/// because something else wanted it, and a `data:` URL is the only thing in
/// Spagitty that needs one.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let triple = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);

        for shift in [18, 12, 6, 0] {
            out.push(ALPHABET[(triple >> shift) as usize & 0x3f] as char);
        }

        // Pad back the characters that stood for bytes the chunk did not have.
        let padding = 3 - chunk.len();
        out.truncate(out.len() - padding);
        for _ in 0..padding {
            out.push('=');
        }
    }
    out
}

/// SHA-256, hex, lowercase.
///
/// Two callers: the Gravatar identifier, whose format is not ours to choose,
/// and the cache file name, which wants a hash for a different reason — so
/// that a directory listing is not a list of everybody's email address.
///
/// Written out rather than depended on for the same reason as [`base64`]: it
/// is one function with a published test vector, and the alternative is a
/// dependency on the application's own attack surface for something a
/// repository walker never needed.
fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // The padded message: the input, a 1 bit, zeroes, and the bit length.
    let mut message = input.to_vec();
    let bits = (input.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());

    for block in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, word) in block.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut v = h;
        for index in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let choose = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let temp1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let majority = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let temp2 = s0.wrapping_add(majority);

            v = [
                temp1.wrapping_add(temp2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(temp1),
                v[4],
                v[5],
                v[6],
            ];
        }

        for (slot, value) in h.iter_mut().zip(v) {
            *slot = slot.wrapping_add(value);
        }
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_published_vectors() {
        // The two every implementation is checked against, so a hand-written
        // one is checked against them too.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_handles_a_message_that_needs_a_second_block() {
        // 56 bytes is the length at which the padding no longer fits in the
        // block the message ends in, which is where a hand-written one breaks.
        assert_eq!(
            sha256_hex(&b"a".repeat(56)),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
    }

    #[test]
    fn base64_pads_every_remainder() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_github_noreply_address_resolves_by_id_and_carries_its_handle() {
        let source = source("1234567+octocat@users.noreply.github.com");

        assert_eq!(source.handle.as_deref(), Some("octocat"));
        assert_eq!(
            source.url.as_deref(),
            Some("https://avatars.githubusercontent.com/u/1234567?s=96&v=4")
        );
    }

    #[test]
    fn an_older_noreply_address_resolves_by_login() {
        // No id in the address, so there is nothing permanent to use and the
        // login is the only thing there is.
        let source = source("octocat@users.noreply.github.com");

        assert_eq!(source.handle.as_deref(), Some("octocat"));
        assert_eq!(
            source.url.as_deref(),
            Some("https://github.com/octocat.png?size=96")
        );
    }

    #[test]
    fn an_ordinary_address_goes_to_gravatar_as_a_hash_and_nothing_else() {
        let source = source("Ada@Example.COM");

        assert_eq!(source.handle, None, "no host was asked who this is");
        let url = source
            .url
            .expect("an ordinary address has somewhere to look");

        // Lowercased before hashing, which is what makes two spellings of one
        // address one person.
        assert!(url.contains(&sha256_hex(b"ada@example.com")), "sent {url}");
        assert!(
            !url.contains("ada"),
            "the address itself must not be in {url}"
        );
        // `d=404` is what keeps a generated Gravatar out: Spagitty has its own
        // generated face and it is the better one.
        assert!(url.contains("d=404"), "sent {url}");
    }

    #[test]
    fn an_address_that_names_no_public_host_is_never_looked_up() {
        for local in [
            "ada",           // git allows a name with no host at all
            "ada@localhost", // a host with no dot is on this machine
            "ada@build-box", // or on this network
            "@example.com",  // no user
            "",
        ] {
            assert_eq!(source(local), Source::default(), "for {local:?}");
        }
    }

    use std::cell::RefCell;
    use std::collections::HashMap;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, b'x'];

    /// A network that answers from a table and remembers what it was asked.
    #[derive(Default)]
    struct Fake {
        json: HashMap<String, Option<(u16, String)>>,
        bytes: HashMap<String, Option<Fetched>>,
        asked: RefCell<Vec<String>>,
        tokens: RefCell<Vec<String>>,
    }

    impl Fake {
        fn json(mut self, url: &str, answer: Option<(u16, &str)>) -> Self {
            self.json.insert(
                url.into(),
                answer.map(|(status, body)| (status, body.into())),
            );
            self
        }

        fn image(mut self, url: &str, status: u16, body: &[u8], location: Option<&str>) -> Self {
            self.bytes.insert(
                url.into(),
                Some(Fetched {
                    status,
                    body: body.to_vec(),
                    location: location.map(str::to_string),
                }),
            );
            self
        }

        fn offline(mut self, url: &str) -> Self {
            self.bytes.insert(url.into(), None);
            self
        }

        fn asked(&self) -> Vec<String> {
            self.asked.borrow().clone()
        }
    }

    impl Net for Fake {
        fn json(&self, url: &str, token: &str, host: &str) -> crate::Result<Response> {
            self.asked.borrow_mut().push(url.into());
            self.tokens.borrow_mut().push(token.into());
            match self.json.get(url) {
                Some(Some((status, body))) => Ok(Response {
                    status: *status,
                    body: body.clone(),
                    retry_after: None,
                }),
                _ => Err(crate::Error::ForgeOffline {
                    host: host.into(),
                    detail: "offline".into(),
                }),
            }
        }

        fn bytes(&self, url: &str, _limit: usize) -> crate::Result<Fetched> {
            self.asked.borrow_mut().push(url.into());
            match self.bytes.get(url) {
                Some(Some(fetched)) => Ok(fetched.clone()),
                _ => Err(crate::Error::ForgeOffline {
                    host: String::new(),
                    detail: "offline".into(),
                }),
            }
        }
    }

    fn flea() -> Repo {
        Repo {
            kind: Kind::GitHub,
            host: "github.com".into(),
            owner: "thisisgm".into(),
            name: "flea".into(),
        }
    }

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const COMMIT_API: &str =
        "https://api.github.com/repos/thisisgm/flea/commits/0123456789abcdef0123456789abcdef01234567";
    const FORGE_PICTURE: &str = "https://avatars.githubusercontent.com/u/30683038?v=4&s=96";
    const ASSOCIATED: &str = r#"{"sha":"x","author":{"login":"thisisgm","avatar_url":"https://avatars.githubusercontent.com/u/30683038?v=4"}}"#;

    fn gravatar(address: &str) -> String {
        format!(
            "https://gravatar.com/avatar/{}?s=96&d=404",
            sha256_hex(address.as_bytes())
        )
    }

    fn no_token() -> Option<String> {
        None
    }

    #[test]
    fn an_ordinary_address_resolves_through_the_forge_and_is_reused() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let net = Fake::default()
            .json(COMMIT_API, Some((200, ASSOCIATED)))
            .image(FORGE_PICTURE, 200, PNG, None);
        let repo = flea();
        let commit = Commit {
            repo: &repo,
            id: SHA,
            token: &no_token,
        };

        let answer = picture(
            dir.path(),
            "gianmarcomorales@icloud.com",
            Some(&commit),
            &net,
        );
        assert_eq!(answer.handle.as_deref(), Some("thisisgm"));
        assert_eq!(answer.picture, data_url(PNG));
        assert!(!answer.retry);
        assert_eq!(
            net.asked(),
            vec![COMMIT_API.to_string(), FORGE_PICTURE.to_string()]
        );
        assert!(
            !net.asked().iter().any(|url| url.contains("gravatar")),
            "an address the forge knows is never sent to Gravatar"
        );

        // The next commit by the same person: nothing asked at all.
        let later = Commit {
            repo: &repo,
            id: "fedcba9876543210fedcba9876543210fedcba98",
            token: &no_token,
        };
        let again = picture(
            dir.path(),
            "GianmarcoMorales@icloud.com ",
            Some(&later),
            &net,
        );
        assert_eq!(again.picture, answer.picture);
        assert_eq!(again.handle.as_deref(), Some("thisisgm"));
        assert_eq!(net.asked().len(), 2);
    }

    #[test]
    fn a_connected_token_goes_to_the_forge_api_and_nowhere_else() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let net = Fake::default()
            .json(COMMIT_API, Some((200, ASSOCIATED)))
            .image(FORGE_PICTURE, 200, PNG, None);
        let repo = flea();
        let token = || Some("ghp_secret".to_string());
        let commit = Commit {
            repo: &repo,
            id: SHA,
            token: &token,
        };

        picture(dir.path(), "gm@icloud.com", Some(&commit), &net);
        // `Net::bytes` has no token parameter at all; the one JSON call had it.
        assert_eq!(net.tokens.borrow().as_slice(), ["ghp_secret"]);
    }

    #[test]
    fn a_commit_with_no_account_falls_back_to_gravatar() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let address = "ada@example.com";
        let net = Fake::default()
            .json(COMMIT_API, Some((200, r#"{"sha":"x","author":null}"#)))
            .image(&gravatar(address), 200, PNG, None);
        let repo = flea();
        let commit = Commit {
            repo: &repo,
            id: SHA,
            token: &no_token,
        };

        let answer = picture(dir.path(), address, Some(&commit), &net);
        assert_eq!(answer.picture, data_url(PNG));
        assert_eq!(answer.handle, None);
    }

    #[test]
    fn a_numeric_noreply_address_uses_the_direct_image_and_asks_no_api() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let direct = "https://avatars.githubusercontent.com/u/62413?s=96&v=4";
        let net = Fake::default().image(direct, 200, PNG, None);
        let repo = flea();
        let commit = Commit {
            repo: &repo,
            id: SHA,
            token: &no_token,
        };

        let answer = picture(
            dir.path(),
            "62413+cmyk@users.noreply.github.com",
            Some(&commit),
            &net,
        );
        assert_eq!(answer.handle.as_deref(), Some("cmyk"));
        assert_eq!(answer.picture, data_url(PNG));
        assert_eq!(net.asked(), vec![direct.to_string()]);
    }

    #[test]
    fn a_handle_only_noreply_address_follows_githubs_redirect_to_its_image_host() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let login = "https://github.com/octocat.png?size=96";
        let image = "https://avatars.githubusercontent.com/u/583231?v=4&s=96";
        let net = Fake::default()
            .image(login, 302, b"", Some(image))
            .image(image, 200, PNG, None);

        let answer = picture(dir.path(), "octocat@users.noreply.github.com", None, &net);
        assert_eq!(answer.picture, data_url(PNG));
        assert_eq!(answer.handle.as_deref(), Some("octocat"));
    }

    #[test]
    fn a_redirect_off_the_image_hosts_is_not_followed_and_not_remembered_as_a_miss() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let login = "https://github.com/octocat.png?size=96";
        let net = Fake::default().image(login, 302, b"", Some("https://evil.example/x.png"));

        let answer = picture(dir.path(), "octocat@users.noreply.github.com", None, &net);
        assert!(answer.retry);
        assert_eq!(net.asked(), vec![login.to_string()]);
        assert!(!cache_file(dir.path(), "octocat@users.noreply.github.com", "miss").exists());
    }

    #[test]
    fn only_a_definitive_not_found_is_remembered_as_a_miss() {
        let address = "ada@example.com";
        let url = gravatar(address);

        let dir = tempfile::tempdir().expect("a temporary directory");
        let net = Fake::default().image(&url, 404, b"", None);
        let answer = picture(dir.path(), address, None, &net);
        assert!(!answer.retry);
        assert!(cache_file(dir.path(), address, "miss").exists());
        // And it is believed: asked again, nothing is sent.
        assert_eq!(picture(dir.path(), address, None, &net), Answer::default());
        assert_eq!(net.asked().len(), 1);

        for (label, net) in [
            ("429", Fake::default().image(&url, 429, b"", None)),
            ("503", Fake::default().image(&url, 503, b"", None)),
            ("offline", Fake::default().offline(&url)),
            (
                "html",
                Fake::default().image(&url, 200, b"<!doctype html>", None),
            ),
        ] {
            let dir = tempfile::tempdir().expect("a temporary directory");
            let answer = picture(dir.path(), address, None, &net);
            assert!(answer.retry, "{label} should be retried");
            assert!(
                !cache_file(dir.path(), address, "miss").exists(),
                "{label} was cached as a miss"
            );
        }
    }

    #[test]
    fn a_rate_limited_forge_is_retried_rather_than_skipped_to_gravatar() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let net = Fake::default().json(COMMIT_API, Some((403, "")));
        let repo = flea();
        let commit = Commit {
            repo: &repo,
            id: SHA,
            token: &no_token,
        };

        let answer = picture(dir.path(), "gm@icloud.com", Some(&commit), &net);
        assert!(answer.retry);
        assert_eq!(net.asked(), vec![COMMIT_API.to_string()]);
    }

    #[test]
    fn a_transient_failure_backs_off_boundedly_and_is_asked_again_after() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let address = "ada@example.com";
        let net = Fake::default().offline(&gravatar(address));

        assert!(picture(dir.path(), address, None, &net).retry);
        // Within the wait: answered from disk, still a retry, nothing sent.
        assert!(picture(dir.path(), address, None, &net).retry);
        assert_eq!(net.asked().len(), 1);

        assert_eq!(backoff(1), BACKOFF_FIRST);
        assert_eq!(backoff(2), BACKOFF_FIRST * 2);
        assert_eq!(backoff(40), BACKOFF_MAX);
    }

    #[test]
    fn misses_written_by_the_old_format_do_not_suppress_the_new_resolver() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let address = "ada@example.com";
        // Where the first format kept its records, and what it wrote for every
        // failure alike.
        let legacy = root
            .path()
            .join(format!("{}.miss", sha256_hex(address.as_bytes())));
        std::fs::write(&legacy, b"").expect("a legacy record");

        let net = Fake::default().image(&gravatar(address), 200, PNG, None);
        let answer = picture(&cache_dir(root.path()), address, None, &net);
        assert_eq!(answer.picture, data_url(PNG));

        retire_legacy(root.path());
        assert!(!legacy.exists());
        assert!(
            cache_dir(root.path()).is_dir(),
            "the current format is left alone"
        );
    }

    #[test]
    fn an_answer_is_read_back_from_the_cache_rather_than_asked_for_again() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "ada@example.com", Some(PNG));

        let net = Fake::default();
        let answer = picture(dir.path(), "ada@example.com", None, &net);
        assert_eq!(answer.picture, data_url(PNG));
        assert!(net.asked().is_empty());
    }

    #[test]
    fn a_remembered_miss_is_answered_without_a_request() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "ada@example.com", None);

        let net = Fake::default();
        assert_eq!(
            picture(dir.path(), "ada@example.com", None, &net),
            Answer::default()
        );
        assert!(net.asked().is_empty());
    }

    #[test]
    fn finding_a_picture_clears_the_miss_that_was_remembered_for_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        write_cache(dir.path(), "ada@example.com", None);
        write_cache(dir.path(), "ada@example.com", Some(PNG));

        assert!(!cache_file(dir.path(), "ada@example.com", "miss").exists());
        assert!(
            picture(dir.path(), "ada@example.com", None, &Fake::default())
                .picture
                .is_some()
        );
    }

    #[test]
    fn the_cache_names_no_address_it_holds() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "ada@example.com", None);

        let name = cache_file(dir.path(), "ada@example.com", "miss");
        let name = name.file_name().expect("a file name").to_string_lossy();
        assert!(!name.contains("ada"), "listed as {name}");
        assert!(!name.contains("example"), "listed as {name}");
    }

    #[test]
    fn an_address_is_cached_under_one_name_however_it_is_written() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "  Ada@Example.com ", None);

        let net = Fake::default();
        assert_eq!(
            picture(dir.path(), "ada@example.com", None, &net),
            Answer::default()
        );
        assert!(net.asked().is_empty());
    }

    #[test]
    fn a_cached_only_read_never_asks_and_only_returns_a_picture() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(cached(dir.path(), "ada@example.com"), None);
        write_cache(dir.path(), "ada@example.com", None);
        assert_eq!(cached(dir.path(), "ada@example.com"), None);
        write_cache(dir.path(), "ada@example.com", Some(PNG));
        assert_eq!(cached(dir.path(), " Ada@example.com"), data_url(PNG));
    }

    #[test]
    fn the_image_hosts_are_a_fixed_list() {
        assert!(image_host_allowed(
            "https://avatars.githubusercontent.com/u/1",
            None
        ));
        assert!(image_host_allowed("https://gravatar.com/avatar/x", None));
        assert!(image_host_allowed(
            "https://ghe.corp.example/avatars/u/1",
            Some("ghe.corp.example")
        ));
        assert!(!image_host_allowed(
            "https://ghe.corp.example/avatars/u/1",
            None
        ));
        assert!(!image_host_allowed(
            "http://avatars.githubusercontent.com/u/1",
            None
        ));
        assert!(!image_host_allowed("https://user@github.com/x", None));
        assert!(!image_host_allowed("https://github.com:8443/x", None));
        assert!(!image_host_allowed(
            "https://github.com.evil.example/x",
            None
        ));
    }

    #[test]
    fn a_commit_id_that_is_not_one_is_never_put_in_a_url() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let address = "ada@example.com";
        let net = Fake::default().image(&gravatar(address), 404, b"", None);
        let repo = flea();
        let commit = Commit {
            repo: &repo,
            id: "../../user",
            token: &no_token,
        };

        picture(dir.path(), address, Some(&commit), &net);
        assert!(!net.asked().iter().any(|url| url.contains("api.github.com")));
    }

    #[test]
    fn a_body_that_is_not_an_image_is_not_offered_as_one() {
        // The usual shape of this is a host answering a picture request with
        // an HTML error page and a 200.
        assert_eq!(kind(b"<!doctype html>"), None);
        assert_eq!(
            kind(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            Some("image/png")
        );
        assert_eq!(kind(&[0xff, 0xd8, 0xff, 0xe0]), Some("image/jpeg"));
    }

    #[test]
    fn forgetting_removes_every_cached_answer_and_survives_there_being_none() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let cache = dir.path().join("avatars");

        assert!(forget(&cache).is_ok(), "nothing cached is not a failure");

        write_cache(&cache, "ada@example.com", None);
        assert!(forget(&cache).is_ok());
        assert!(!cache.exists());
    }
}

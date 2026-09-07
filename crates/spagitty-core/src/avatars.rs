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
//! - **Nothing is asked that does not have to be.** No API, no token, no query
//!   for who an address belongs to. The GitHub no-reply forms carry the account
//!   in the address itself, so those cost no lookup and leak nothing. Only an
//!   ordinary address reaches Gravatar, and only as a hash.
//! - **A missing network is the ordinary case, not a failure.** Every path
//!   here returns `None` rather than an error, and `None` is the generated
//!   face. A repository opened on a train looks exactly like it did before this
//!   module existed.
//!
//! And it is a preference, defaulting to on, that turns the whole thing off.
//!
//! # What leaves the machine
//!
//! For a GitHub no-reply address: a request to `avatars.githubusercontent.com`
//! or `github.com` for a picture, carrying an account name that is already
//! written in the repository's commits in plain text.
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

use crate::forge::http;

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

/// What the cache holds for one address.
enum Cached {
    /// A picture, as the bytes that were served.
    Found(Vec<u8>),
    /// Asked, and the host said there is none. Worth remembering: it is what
    /// stops the expensive case from being asked again on every launch.
    Missing,
    /// Never asked, or asked long enough ago that the answer has expired.
    Unknown,
}

/// One author's picture, as a `data:` URL, fetching it if it is not held.
///
/// `data:` rather than a file path because that is what the webview's content
/// policy already allows for images, and because it makes the picture a value
/// the frontend owns rather than a second path into the cache directory that
/// would need its own scope and its own escaping rules.
///
/// Returns `None` for every ordinary failure — no network, no picture for this
/// address, a host answering with something that is not an image. The caller
/// draws the generated face, which is what it drew before this existed.
pub fn picture(cache: &Path, email: &str) -> Option<String> {
    let source = source(email);
    let url = source.url?;
    let held = read_cache(cache, email);

    let bytes = match held {
        Cached::Found(bytes) => bytes,
        Cached::Missing => return None,
        Cached::Unknown => {
            let (status, body) = http::get_bytes(&url, MAX_BYTES).ok()?;

            // Anything that is not a picture is a miss, including a redirect
            // we declined to follow and a rate limit. Remembering the miss is
            // right for all of them: the retry is the next expiry, not the
            // next scroll.
            let ok = (200..300).contains(&status) && kind(&body).is_some();
            write_cache(cache, email, if ok { Some(&body) } else { None });
            if !ok {
                return None;
            }
            body
        }
    };

    let mime = kind(&bytes)?;
    Some(format!("data:{mime};base64,{}", base64(&bytes)))
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
fn cache_file(cache: &Path, email: &str, missing: bool) -> PathBuf {
    let name = sha256_hex(email.trim().to_lowercase().as_bytes());
    let extension = if missing { "miss" } else { "img" };
    cache.join(format!("{name}.{extension}"))
}

fn read_cache(cache: &Path, email: &str) -> Cached {
    let found = cache_file(cache, email, false);
    if let Ok(meta) = std::fs::metadata(&found) {
        if fresh(&meta, FRESH) {
            if let Ok(bytes) = std::fs::read(&found) {
                return Cached::Found(bytes);
            }
        }
    }

    let missing = cache_file(cache, email, true);
    if let Ok(meta) = std::fs::metadata(&missing) {
        if fresh(&meta, FRESH_MISS) {
            return Cached::Missing;
        }
    }

    Cached::Unknown
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

    // The two files are exclusive: finding a picture for an address that had
    // none must not leave the miss behind to be found first next time.
    let found = cache_file(cache, email, false);
    let missing = cache_file(cache, email, true);

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

    #[test]
    fn an_answer_is_read_back_from_the_cache_rather_than_asked_for_again() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let png = [
            &[0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a][..],
            b"body",
        ]
        .concat();

        write_cache(dir.path(), "ada@example.com", Some(&png));

        // No network is reachable in a test, so anything this returns came off
        // the disk.
        let picture = picture(dir.path(), "ada@example.com");
        assert_eq!(
            picture.as_deref(),
            Some(format!("data:image/png;base64,{}", base64(&png)).as_str())
        );
    }

    #[test]
    fn a_remembered_miss_is_answered_without_a_request() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "ada@example.com", None);

        assert_eq!(picture(dir.path(), "ada@example.com"), None);
    }

    #[test]
    fn finding_a_picture_clears_the_miss_that_was_remembered_for_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let png = [
            &[0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a][..],
            b"x",
        ]
        .concat();

        write_cache(dir.path(), "ada@example.com", None);
        write_cache(dir.path(), "ada@example.com", Some(&png));

        assert!(!cache_file(dir.path(), "ada@example.com", true).exists());
        assert!(picture(dir.path(), "ada@example.com").is_some());
    }

    #[test]
    fn the_cache_names_no_address_it_holds() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "ada@example.com", None);

        let name = cache_file(dir.path(), "ada@example.com", true);
        let name = name.file_name().expect("a file name").to_string_lossy();
        assert!(!name.contains("ada"), "listed as {name}");
        assert!(!name.contains("example"), "listed as {name}");
    }

    #[test]
    fn an_address_is_cached_under_one_name_however_it_is_written() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        write_cache(dir.path(), "  Ada@Example.com ", None);

        assert_eq!(picture(dir.path(), "ada@example.com"), None);
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

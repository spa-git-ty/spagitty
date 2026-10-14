// SPDX-License-Identifier: GPL-3.0-or-later

//! Who last touched each line.
//!
//! Read from `git blame --line-porcelain` rather than computed here, and that
//! is a documented exception to the `shell.rs` rule rather than a shortcut:
//! `gix::blame` 0.16 — the newest published version — panics on an ordinary
//! history shape, a file blamed at a merge commit whose history contains an
//! intervening commit that left the file alone. The exception, its reason and
//! its end condition are recorded on `shell::blame`.
//!
//! What this module owns is the part git does not do: deciding what is worth
//! blaming at all, and turning the porcelain stream into rows.
//!
//! A refusal is reported rather than returned as nothing. A binary file, a path
//! that is not in that revision, and a directory each come back saying which —
//! an empty list would read as a file nobody has ever touched, which is a
//! different and much stranger claim.

use serde::Serialize;

use std::path::Path;

use crate::error::{Error, Result};
use crate::shell;

/// How far into a blob we look for a NUL byte before calling it binary. Git's
/// own rule, so the same files are called binary here as everywhere else in the
/// application.
const BINARY_SNIFF_BYTES: usize = 8000;

/// Above this, blaming is refused rather than attempted. Blame reads every
/// revision of the file it walks through, so the cost is the file size times
/// the depth of its history.
const MAX_BLOB_BYTES: usize = 4 * 1024 * 1024;

/// One line of the file, and the commit that last changed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    /// 1-based line number in the blamed file.
    pub line: u32,
    pub text: String,
    pub commit: String,
    pub short: String,
    pub summary: String,
    pub author_name: String,
    /// Author time, seconds since the unix epoch.
    pub time: i64,
    /// Where this line lived before, when the commit that introduced it also
    /// renamed the file. `None` for the overwhelmingly common case.
    pub source_path: Option<String>,
}

/// Why there is nothing to blame.
///
/// Not an error: these are ordinary states of ordinary files, and the strip
/// says which one rather than failing the screen around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotBlamable {
    Binary,
    TooLarge,
    /// No such path in that revision, or the path is a directory or submodule.
    NotAFile,
    /// A file with no lines has nothing to attribute.
    Empty,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Blame {
    pub path: String,
    /// The revision blamed, resolved to a full id.
    pub revision: String,
    pub lines: Vec<BlameLine>,
    /// Set when `lines` is empty for a reason worth stating.
    pub refused: Option<NotBlamable>,
}

/// One commit in a file's history (FEAT-063).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHistoryEntry {
    pub commit: String,
    pub short: String,
    pub author_name: String,
    pub author_email: String,
    pub time: i64,
    pub summary: String,
}

/// Parse null-delimited log entries into `FileHistoryEntry`.
pub fn parse_file_history(raw: &str) -> Vec<FileHistoryEntry> {
    let mut entries = Vec::new();
    for line in raw.lines() {
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.len() >= 6 {
            let commit = parts[0].to_string();
            let short = parts[1].to_string();
            let author_name = parts[2].to_string();
            let author_email = parts[3].to_string();
            let time = parts[4].parse::<i64>().unwrap_or(0);
            let summary = parts[5].to_string();

            entries.push(FileHistoryEntry {
                commit,
                short,
                author_name,
                author_email,
                time,
                summary,
            });
        }
    }
    entries
}

/// Read the commit evolution history of a single file path (FEAT-063).
pub fn history(repo: &gix::Repository, path: &str, limit: usize) -> Result<Vec<FileHistoryEntry>> {
    let dir = workdir(repo)?;
    let raw = shell::file_history(dir, path, limit)?;
    Ok(parse_file_history(&raw))
}

/// Blame `path` as of `revision`.
///
/// `revision` is anything `git rev-parse` would take — a branch name, a tag, a
/// short id, `HEAD`. An empty string means `HEAD`.
pub fn file(repo: &gix::Repository, path: &str, revision: &str) -> Result<Blame> {
    let spec = if revision.trim().is_empty() {
        "HEAD"
    } else {
        revision.trim()
    };

    // Peeled, because an annotated tag resolves to the tag object and a blame
    // is about the commit it points at.
    let suspect = repo
        .rev_parse_single(spec)
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .object()
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .peel_to_commit()
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .id;

    let blamed = Blame {
        path: path.to_string(),
        revision: suspect.to_string(),
        lines: Vec::new(),
        refused: None,
    };

    // Read the blob first. Refusing a binary or oversized file here costs one
    // lookup; discovering it inside the blame walk costs the whole walk.
    let Some(contents) = blob_at(repo, suspect, path)? else {
        return Ok(Blame {
            refused: Some(NotBlamable::NotAFile),
            ..blamed
        });
    };
    if is_binary(&contents) {
        return Ok(Blame {
            refused: Some(NotBlamable::Binary),
            ..blamed
        });
    }
    if contents.len() > MAX_BLOB_BYTES {
        return Ok(Blame {
            refused: Some(NotBlamable::TooLarge),
            ..blamed
        });
    }
    let text = String::from_utf8_lossy(&contents).into_owned();
    let lines: Vec<&str> = split_lines(&text);
    if lines.is_empty() {
        return Ok(Blame {
            refused: Some(NotBlamable::Empty),
            ..blamed
        });
    }

    let porcelain = shell::blame(workdir(repo)?, spec, path)?;

    Ok(Blame {
        lines: parse(&porcelain, path),
        ..blamed
    })
}

/// One commit a blamed file's lines come from (FEAT-131).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameCommit {
    pub id: String,
    pub short: String,
    pub summary: String,
    pub author_name: String,
    /// The author's address, which is what a portrait is found by.
    pub author_email: String,
    /// Author time, seconds since the unix epoch.
    pub time: i64,
    /// The agent a `Co-authored-by` trailer names, by the rule gate 1 applies
    /// ([`crate::attribution`]). `None` for a commit a person wrote alone.
    pub agent: Option<String>,
    /// The path the lines were written under at this commit.
    pub path: String,
    /// The commit and path this one's version of the file was changed from:
    /// what "blame before this" blames, and what the peek's "was" line is
    /// read from. `None` when this commit added the file.
    pub previous: Option<String>,
    pub previous_path: Option<String>,
}

/// A whole file's blame, with each commit said once.
///
/// [`Blame`] repeats the commit on every line and carries the text again;
/// that is right for a strip beside a diff and wrong for a file of twenty
/// thousand lines the screen already has. Here a line is two numbers.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactBlame {
    pub path: String,
    /// The revision blamed, resolved to a full id.
    pub revision: String,
    pub commits: Vec<BlameCommit>,
    /// For each line of the file, in order: an index into `commits`.
    pub lines: Vec<u32>,
    /// For each line: its number in the commit that wrote it.
    pub origin: Vec<u32>,
    /// Set when there are no lines for a reason worth stating.
    pub refused: Option<NotBlamable>,
}

/// Blame `path` as of `revision`, compactly (FEAT-131).
///
/// The same refusals as [`file`], and the same `git blame`. Works in a bare
/// repository: a revision is given, so git reads objects and no checkout.
pub fn compact(repo: &gix::Repository, path: &str, revision: &str) -> Result<CompactBlame> {
    let spec = if revision.trim().is_empty() {
        "HEAD"
    } else {
        revision.trim()
    };
    let suspect = repo
        .rev_parse_single(spec)
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .object()
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .peel_to_commit()
        .map_err(|_| Error::UnknownCommit(spec.to_string()))?
        .id;

    let empty = |refused| CompactBlame {
        path: path.to_string(),
        revision: suspect.to_string(),
        commits: Vec::new(),
        lines: Vec::new(),
        origin: Vec::new(),
        refused,
    };

    let Some(contents) = blob_at(repo, suspect, path)? else {
        return Ok(empty(Some(NotBlamable::NotAFile)));
    };
    if is_binary(&contents) {
        return Ok(empty(Some(NotBlamable::Binary)));
    }
    if contents.len() > MAX_BLOB_BYTES {
        return Ok(empty(Some(NotBlamable::TooLarge)));
    }
    if contents.is_empty() {
        return Ok(empty(Some(NotBlamable::Empty)));
    }

    let dir = repo.workdir().unwrap_or_else(|| repo.git_dir());
    // The id rather than the name it was asked by: a branch can move between
    // the read above and the blame below, and the two must be of one commit.
    let porcelain = shell::blame_porcelain(dir, &suspect.to_string(), path)?;
    let mut blamed = parse_compact(&porcelain, path);

    for commit in &mut blamed.commits {
        let Ok(id) = gix::ObjectId::from_hex(commit.id.as_bytes()) else {
            continue;
        };
        if let Ok(found) = repo.find_commit(id) {
            commit.agent = crate::attribution::agent_in(&found.message_raw_sloppy().to_string());
        }
    }

    Ok(CompactBlame {
        commits: blamed.commits,
        lines: blamed.lines,
        origin: blamed.origin,
        ..empty(None)
    })
}

struct Compact {
    commits: Vec<BlameCommit>,
    lines: Vec<u32>,
    origin: Vec<u32>,
}

/// Turn `--porcelain` into commits and a commit per line.
///
/// A group starts with `<sha> <orig> <final> [count]`. The first time a commit
/// appears its headers follow; after that only the header line does, so a
/// commit is filled in once and looked up by id from then on. Content arrives
/// on a line with a leading tab, and ends the entry for that line.
fn parse_compact(porcelain: &str, asked_for: &str) -> Compact {
    let mut commits: Vec<BlameCommit> = Vec::new();
    let mut index: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut lines = Vec::new();
    let mut origin = Vec::new();
    // The commit and original line the content line that follows belongs to.
    let mut at: Option<(u32, u32)> = None;

    for raw in porcelain.lines() {
        if raw.starts_with('\t') {
            if let Some((commit, line)) = at.take() {
                lines.push(commit);
                origin.push(line);
            }
            continue;
        }

        let (key, value) = raw.split_once(' ').unwrap_or((raw, ""));
        if is_object_id(key) {
            let original = value
                .split(' ')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            let commit = *index.entry(key.to_string()).or_insert_with(|| {
                commits.push(BlameCommit {
                    id: key.to_string(),
                    short: key.chars().take(7).collect(),
                    summary: String::new(),
                    author_name: String::new(),
                    author_email: String::new(),
                    time: 0,
                    agent: None,
                    path: asked_for.to_string(),
                    previous: None,
                    previous_path: None,
                });
                (commits.len() - 1) as u32
            });
            at = Some((commit, original));
            continue;
        }

        let Some((commit, _)) = at else { continue };
        let commit = &mut commits[commit as usize];
        match key {
            "author" => commit.author_name = value.to_string(),
            // `<ada@example.com>`: the address without its brackets.
            "author-mail" => {
                commit.author_email = value
                    .trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string();
            }
            "author-time" => commit.time = value.parse().unwrap_or(0),
            "summary" => commit.summary = value.to_string(),
            "filename" => commit.path = value.to_string(),
            // `previous <sha> <path>`, the path being everything after the id.
            "previous" => {
                if let Some((id, path)) = value.split_once(' ') {
                    commit.previous = Some(id.to_string());
                    commit.previous_path = Some(path.to_string());
                }
            }
            _ => {}
        }
    }

    Compact {
        commits,
        lines,
        origin,
    }
}

/// The working directory to run `git` in.
///
/// A bare repository has no checkout, and `git blame` wants one. Blaming a bare
/// repository is not something this screen can be asked for, since it has no
/// file list to ask from.
fn workdir(repo: &gix::Repository) -> Result<&Path> {
    repo.workdir()
        .ok_or_else(|| Error::NotStageable("a bare repository has no working copy".into()))
}

/// Turn `--line-porcelain` into rows.
///
/// `--line-porcelain` repeats every header on every line, so this is a loop
/// over lines rather than a state machine over hunks. A line is complete when
/// its content arrives, which porcelain marks with a leading tab.
///
/// Unknown headers are skipped rather than refused: git adds fields over time,
/// and a blame that stops working because a future git said something new would
/// be a poor trade for strictness we do not need.
///
/// `asked_for` is the path the caller blamed. Git's `filename` header names the
/// path *at the commit that introduced the line*, so a line whose filename is
/// not the one we asked about is a line that arrived under another name.
fn parse(porcelain: &str, asked_for: &str) -> Vec<BlameLine> {
    let mut out = Vec::new();
    let mut current = Partial::default();

    for raw in porcelain.lines() {
        if let Some(content) = raw.strip_prefix('\t') {
            if let Some(line) = current.finish(content, asked_for) {
                out.push(line);
            }
            current = Partial::default();
            continue;
        }

        let (key, value) = match raw.split_once(' ') {
            Some(pair) => pair,
            None => (raw, ""),
        };

        match key {
            "author" => current.author_name = Some(value.to_string()),
            "author-time" => current.time = value.parse().ok(),
            "summary" => current.summary = Some(value.to_string()),
            "filename" => current.filename = Some(value.to_string()),
            _ => {
                // A header line, which is "<sha> <orig> <final> [count]".
                if is_object_id(key) {
                    current.commit = Some(key.to_string());
                    current.line = value.split(' ').nth(1).and_then(|n| n.parse().ok());
                }
            }
        }
    }

    out
}

/// One line's headers, accumulated until its content arrives.
#[derive(Default)]
struct Partial {
    commit: Option<String>,
    line: Option<u32>,
    author_name: Option<String>,
    time: Option<i64>,
    summary: Option<String>,
    filename: Option<String>,
}

impl Partial {
    /// A row, or nothing when the headers were incomplete — which would mean
    /// git printed content without a header, and dropping that line is better
    /// than attributing it to whatever came before.
    fn finish(self, text: &str, asked_for: &str) -> Option<BlameLine> {
        let commit = self.commit?;
        Some(BlameLine {
            line: self.line?,
            text: text.to_string(),
            short: commit.chars().take(7).collect(),
            commit,
            summary: self.summary.unwrap_or_default(),
            author_name: self.author_name.unwrap_or_default(),
            time: self.time.unwrap_or(0),
            // Only when the line arrived under another name; the same name is
            // not a rename.
            source_path: self.filename.filter(|name| name != asked_for),
        })
    }
}

fn is_object_id(text: &str) -> bool {
    text.len() == 40 && text.chars().all(|c| c.is_ascii_hexdigit())
}

fn blob_at(repo: &gix::Repository, commit: gix::ObjectId, path: &str) -> Result<Option<Vec<u8>>> {
    let tree = repo
        .find_commit(commit)
        .map_err(|e| Error::UnknownCommit(e.to_string()))?
        .tree()
        .map_err(|e| Error::Diff(e.to_string()))?;

    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(path))
        .map_err(|e| Error::Diff(e.to_string()))?;

    let Some(entry) = entry.filter(|entry| entry.mode().is_blob_or_symlink()) else {
        return Ok(None);
    };

    let object = repo
        .find_object(entry.object_id())
        .map_err(|e| Error::Diff(e.to_string()))?;
    Ok(Some(object.detach().data))
}

/// Lines the way a file viewer counts them: a trailing newline ends the last
/// line rather than starting an empty one.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

fn is_binary(data: &[u8]) -> bool {
    data[..data.len().min(BINARY_SNIFF_BYTES)].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// `git blame --porcelain`'s commit-per-line answer, as `(line, commit)`.
    fn git_blame(fixture: &Fixture, path: &str, revision: &str) -> Vec<(u32, String)> {
        let out = fixture.git(&["blame", "--line-porcelain", revision, "--", path]);

        let mut pairs = Vec::new();
        for line in out.lines() {
            // A porcelain header is "<sha> <orig-line> <final-line> [count]".
            let mut parts = line.split(' ');
            let (Some(sha), Some(_orig), Some(final_line)) =
                (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let Ok(number) = final_line.parse::<u32>() else {
                continue;
            };
            pairs.push((number, sha.to_string()));
        }
        pairs
    }

    #[test]
    fn every_line_is_attributed_to_the_commit_git_blames_it_on() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "core.txt", "HEAD").expect("blame");
        let mine: Vec<(u32, String)> = blame
            .lines
            .iter()
            .map(|line| (line.line, line.commit.clone()))
            .collect();

        assert_eq!(mine, git_blame(&fixture, "core.txt", "HEAD"));
    }

    #[test]
    fn the_text_of_each_line_is_the_file_itself() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "notes.md", "HEAD").expect("blame");
        let from_disk = std::fs::read_to_string(fixture.path().join("notes.md")).expect("file");

        assert_eq!(
            blame
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            from_disk.lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn lines_arrive_in_line_order_rather_than_hunk_order() {
        // gix produces entries per hunk, which is not top-to-bottom, and the
        // strip is read top to bottom.
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "core.txt", "HEAD").expect("blame");

        assert_eq!(
            blame.lines.iter().map(|l| l.line).collect::<Vec<_>>(),
            (1..=blame.lines.len() as u32).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_row_carries_who_and_when_and_what_the_commit_said() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "core.txt", "HEAD").expect("blame");
        let first = &blame.lines[0];

        assert_eq!(first.author_name, "Ada Lovelace");
        assert_eq!(first.short.len(), 7);
        assert!(!first.summary.is_empty());
        assert!(first.time > 0);
    }

    #[test]
    fn blaming_an_older_revision_answers_for_that_revision() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let older = file(&repo, "core.txt", "v0.1.0").expect("blame");
        let now = file(&repo, "core.txt", "HEAD").expect("blame");

        assert_eq!(older.revision, fixture.rev("v0.1.0^{commit}"));
        assert_ne!(
            older.lines.iter().map(|l| &l.commit).collect::<Vec<_>>(),
            now.lines.iter().map(|l| &l.commit).collect::<Vec<_>>(),
            "the rewrites happened after v0.1.0"
        );
    }

    #[test]
    fn an_empty_revision_means_head() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert_eq!(
            file(&repo, "core.txt", "").expect("blame").revision,
            file(&repo, "core.txt", "HEAD").expect("blame").revision
        );
    }

    #[test]
    fn a_binary_file_says_so_rather_than_rendering_empty() {
        // An empty list would read as a file nobody has ever touched, which is
        // a different and much stranger claim.
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "logo.bin", "HEAD").expect("blame");

        assert_eq!(blame.refused, Some(NotBlamable::Binary));
        assert!(blame.lines.is_empty());
    }

    #[test]
    fn a_path_that_is_not_in_that_revision_says_so() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert_eq!(
            file(&repo, "no/such/file.txt", "HEAD")
                .expect("blame")
                .refused,
            Some(NotBlamable::NotAFile)
        );
        assert_eq!(
            file(&repo, "split.txt", "v0.1.0").expect("blame").refused,
            Some(NotBlamable::NotAFile),
            "it did not exist yet at that tag"
        );
    }

    #[test]
    fn a_directory_is_not_a_file_to_blame() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert_eq!(
            file(&repo, "src/deep", "HEAD").expect("blame").refused,
            Some(NotBlamable::NotAFile)
        );
    }

    #[test]
    fn a_file_with_no_lines_has_nothing_to_attribute() {
        let fixture = Fixture::woven();
        fixture.write("hollow.txt", "");
        fixture.git(&["add", "hollow.txt"]);
        fixture.commit("Add an empty file");
        let repo = fixture.open();

        let blame = file(&repo, "hollow.txt", "HEAD").expect("blame");

        assert_eq!(blame.refused, Some(NotBlamable::Empty));
    }

    #[test]
    fn an_unknown_revision_is_an_error_rather_than_a_refusal() {
        // A refusal says "this file cannot be blamed". A revision that does not
        // exist is a different problem, and one the caller got wrong.
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert!(file(&repo, "core.txt", "no-such-branch").is_err());
    }

    #[test]
    fn a_renamed_file_carries_where_the_line_lived_before() {
        let fixture = Fixture::woven();
        fixture.git(&["mv", "notes.md", "journal.md"]);
        fixture.commit_all("Rename notes to journal");
        let repo = fixture.open();

        let blame = file(&repo, "journal.md", "HEAD").expect("blame");

        assert!(
            blame
                .lines
                .iter()
                .any(|line| line.source_path.as_deref() == Some("notes.md")),
            "the lines that predate the rename came from notes.md"
        );
    }

    #[test]
    fn the_same_filename_is_not_a_rename() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = file(&repo, "core.txt", "HEAD").expect("blame");

        assert!(blame.lines.iter().all(|line| line.source_path.is_none()));
    }

    #[test]
    fn a_header_the_parser_does_not_know_is_skipped_rather_than_refused() {
        // Git adds fields over time. A blame that stopped working because a
        // future git said something new would be a poor trade for strictness.
        let porcelain = concat!(
            "1111111111111111111111111111111111111111 1 1 1\n",
            "author Ada Lovelace\n",
            "author-time 1700000000\n",
            "some-future-header whatever it says\n",
            "summary A commit\n",
            "filename a.txt\n",
            "\tthe line\n"
        );

        let lines = parse(porcelain, "a.txt");

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].author_name, "Ada Lovelace");
        assert_eq!(lines[0].text, "the line");
        assert_eq!(lines[0].line, 1);
    }

    #[test]
    fn content_arriving_without_a_header_is_dropped_rather_than_misattributed() {
        let lines = parse("\torphaned content\n", "a.txt");

        assert!(lines.is_empty());
    }

    #[test]
    fn splitting_lines_treats_a_trailing_newline_as_ending_the_last_line() {
        assert_eq!(split_lines(""), Vec::<&str>::new());
        assert_eq!(split_lines("one\n"), vec!["one"]);
        assert_eq!(split_lines("one\ntwo"), vec!["one", "two"]);
        assert_eq!(split_lines("one\n\n"), vec!["one", ""]);
    }

    #[test]
    fn a_nul_byte_after_the_sniff_window_is_still_text() {
        let mut late = vec![b'a'; BINARY_SNIFF_BYTES + 10];
        late[BINARY_SNIFF_BYTES + 5] = 0;

        assert!(!is_binary(&late));
        assert!(is_binary(b"\0start"));
    }

    #[test]
    fn parses_null_delimited_file_history_stream() {
        let sample = "1a2b3c4d\x001a2b3c4\x00Ada Lovelace\x00ada@example.com\x001700000000\x00Initial commit\n2b3c4d5e\x002b3c4d5\x00Charles Babbage\x00charles@example.com\x001700000100\x00Second commit\n";
        let entries = parse_file_history(sample);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].commit, "1a2b3c4d");
        assert_eq!(entries[0].author_name, "Ada Lovelace");
        assert_eq!(entries[0].time, 1700000000);
        assert_eq!(entries[0].summary, "Initial commit");
        assert_eq!(entries[1].short, "2b3c4d5");
    }

    #[test]
    fn file_history_walks_fixture_commits() {
        let fixture = Fixture::woven();
        let repo = fixture.open();
        let entries = history(&repo, "core.txt", 10).expect("file history");
        assert!(!entries.is_empty());
        assert_eq!(entries[0].author_name, "Ada Lovelace");
    }

    // --- The compact form Files reads (FEAT-131) ---------------------------

    #[test]
    fn the_compact_blame_agrees_with_the_one_that_repeats_itself() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let long = file(&repo, "core.txt", "HEAD").expect("blame");
        let short = compact(&repo, "core.txt", "HEAD").expect("blame");

        assert_eq!(short.lines.len(), long.lines.len());
        assert_eq!(short.origin.len(), short.lines.len());
        for (line, &commit) in long.lines.iter().zip(&short.lines) {
            let commit = &short.commits[commit as usize];
            assert_eq!(commit.id, line.commit, "line {}", line.line);
            assert_eq!(commit.summary, line.summary);
            assert_eq!(commit.author_name, line.author_name);
            assert_eq!(commit.time, line.time);
        }
        assert_eq!(short.revision, fixture.head());
        assert_eq!(short.refused, None);
    }

    #[test]
    fn each_commit_is_said_once_with_who_wrote_it() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = compact(&repo, "core.txt", "HEAD").expect("blame");
        let ids: std::collections::HashSet<&str> =
            blame.commits.iter().map(|c| c.id.as_str()).collect();

        // Three commits wrote core.txt: the import and the two rewrites.
        assert_eq!(blame.commits.len(), 3);
        assert_eq!(ids.len(), 3);
        for commit in &blame.commits {
            assert_eq!(commit.author_email, "ada@example.com");
            assert_eq!(commit.short, commit.id[..7]);
            assert_eq!(commit.path, "core.txt");
            assert_eq!(commit.agent, None);
        }
    }

    #[test]
    fn a_commit_that_changed_the_file_names_what_it_changed_it_from() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let blame = compact(&repo, "core.txt", "HEAD").expect("blame");
        let import = fixture.rev("HEAD~2");
        let rewrite = blame
            .commits
            .iter()
            .find(|c| c.summary == "Rewrite line 38")
            .expect("the rewrite");
        let added = blame
            .commits
            .iter()
            .find(|c| c.summary == "Initial import")
            .expect("the import");

        // Line 38 is line 38 of the commit that rewrote it, changed from the
        // import; the import added the file and has no before.
        assert_eq!(blame.origin[37], 38);
        assert_eq!(blame.commits[blame.lines[37] as usize].id, rewrite.id);
        assert_eq!(rewrite.previous.as_deref(), Some(import.as_str()));
        assert_eq!(rewrite.previous_path.as_deref(), Some("core.txt"));
        assert_eq!(added.previous, None);
    }

    #[test]
    fn a_line_from_an_agents_commit_is_marked_and_a_namesakes_is_not() {
        let fixture = Fixture::empty();
        fixture.write("a.rs", "one\ntwo\nthree\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Add");
        fixture.write("a.rs", "one\nTWO\nthree\n");
        fixture.commit_all("Cache on disk\n\nCo-authored-by: Codex <noreply@example.com>");
        fixture.write("a.rs", "one\nTWO\nTHREE\n");
        fixture.commit_all("Tidy\n\nCo-authored-by: Claude Shannon <claude@bell-labs.example>");
        let repo = fixture.open();

        let blame = compact(&repo, "a.rs", "HEAD").expect("blame");
        let agent_of = |line: usize| blame.commits[blame.lines[line] as usize].agent.as_deref();

        assert_eq!(agent_of(0), None);
        assert_eq!(agent_of(1), Some("Codex"));
        assert_eq!(agent_of(2), None, "a person called Claude is a person");
    }

    #[test]
    fn a_renamed_files_lines_keep_the_name_they_were_written_under() {
        let fixture = Fixture::empty();
        fixture.write("old.rs", "alpha\nbeta\ngamma\ndelta\nepsilon\n");
        fixture.git(&["add", "-A"]);
        let first = fixture.commit("Add");
        fixture.git(&["mv", "old.rs", "new.rs"]);
        fixture.write("new.rs", "alpha\nBETA\ngamma\ndelta\nepsilon\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Rename and edit");
        let repo = fixture.open();

        let blame = compact(&repo, "new.rs", "HEAD").expect("blame");
        let of = |line: usize| &blame.commits[blame.lines[line] as usize];

        assert_eq!(of(0).path, "old.rs");
        assert_eq!(of(1).path, "new.rs");
        assert_eq!(of(1).previous.as_deref(), Some(first.as_str()));
        assert_eq!(of(1).previous_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn the_compact_blame_refuses_what_the_long_one_refuses() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert_eq!(
            compact(&repo, "logo.bin", "HEAD").expect("blame").refused,
            Some(NotBlamable::Binary)
        );
        assert_eq!(
            compact(&repo, "nope.txt", "HEAD").expect("blame").refused,
            Some(NotBlamable::NotAFile)
        );
        assert_eq!(
            compact(&repo, "src", "HEAD").expect("blame").refused,
            Some(NotBlamable::NotAFile)
        );
        assert!(compact(&repo, "core.txt", "no-such-rev").is_err());
    }

    #[test]
    fn an_older_revision_is_blamed_as_it_was() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let then = compact(&repo, "core.txt", "v0.1.0").expect("blame");

        // At the first tag nothing had been rewritten yet.
        assert_eq!(then.commits.len(), 1);
        assert_eq!(then.commits[0].summary, "Initial import");
        assert_eq!(then.lines.len(), 40);
    }

    #[test]
    fn a_bare_repository_is_blamed_from_its_objects() {
        let fixture = Fixture::woven();
        let bare = tempfile::tempdir().expect("temp dir");
        let target = bare.path().join("bare.git");
        fixture.git(&[
            "clone",
            "-q",
            "--bare",
            ".",
            target.to_str().expect("a utf-8 path"),
        ]);
        let repo = gix::open(&target).expect("open the bare clone");

        let blame = compact(&repo, "core.txt", "main").expect("blame");

        assert_eq!(blame.lines.len(), 40);
        assert_eq!(blame.refused, None);
    }

    #[test]
    fn porcelain_with_a_path_that_has_spaces_keeps_the_whole_path() {
        let id = "a".repeat(40);
        let before = "b".repeat(40);
        let porcelain = format!(
            "{id} 3 1 1\nauthor Ada Lovelace\nauthor-mail <ada@example.com>\nauthor-time 1700000000\nsummary Edit\nprevious {before} my docs/old name.md\nfilename my docs/new name.md\n\thello\n{id} 9 2\n\tworld\n"
        );

        let parsed = parse_compact(&porcelain, "my docs/new name.md");

        assert_eq!(parsed.commits.len(), 1);
        assert_eq!(parsed.lines, [0, 0]);
        assert_eq!(parsed.origin, [3, 9]);
        assert_eq!(
            parsed.commits[0].previous_path.as_deref(),
            Some("my docs/old name.md")
        );
        assert_eq!(parsed.commits[0].path, "my docs/new name.md");
        assert_eq!(parsed.commits[0].author_email, "ada@example.com");
    }
}

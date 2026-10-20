// SPDX-License-Identifier: GPL-3.0-or-later

//! A repository's files at a revision, read from git's objects (FEAT-131).
//!
//! Files shows any file at any branch, tag or commit, and it cannot write.
//! That is not a switch: everything here takes a revision and reads trees and
//! blobs from the object database. There is no path in this module to a file
//! on disk — it never asks the repository where its working copy is — so there
//! is nothing to disable and nothing a future change could leave enabled. A
//! test reads this file's source to keep it that way, and another reads a file
//! after its working copy has been changed and then deleted.
//!
//! A bare repository reads the same, having objects and nothing else.

use serde::Serialize;

use crate::diff::{self, LineOrigin};
use crate::error::{Error, Result};
use crate::highlight;

/// Above this a file is not opened, and the screen says how big it is.
pub const OPEN_MAX_BYTES: u64 = 50 * 1024 * 1024;

/// An image larger than this is shown as binary: it crosses to the webview as
/// base64, the same limit the Diff screen's image viewer keeps.
const IMAGE_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// How far into a blob we look for a NUL byte before calling it binary. Git's
/// own rule.
const BINARY_SNIFF_BYTES: usize = 8000;

/// A Git LFS pointer is a few lines of text; the specification caps it.
const LFS_POINTER_MAX_BYTES: usize = 1024;
const LFS_VERSION: &str = "version https://git-lfs.github.com/spec/v1";

/// How many tree entries *Find a file* reads before it stops looking.
const FIND_WALK_MAX: usize = 300_000;

/// A branch, tag or commit, resolved to the commit it means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub id: String,
    pub short: String,
}

/// What a tree entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    Folder,
    File,
    /// A symbolic link. Its blob is the path it points at.
    Link,
    /// Another repository's commit. Not followed into.
    Submodule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    /// From the repository's root, with `/` between the parts.
    pub path: String,
    pub kind: EntryKind,
}

/// What a file is, and its text when it has any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    Text {
        text: String,
    },
    /// Drawn by the image viewer.
    Image {
        mime: String,
        base64: String,
    },
    Binary,
    /// A Git LFS pointer, shown as one: the object it stands for and its size.
    #[serde(rename_all = "camelCase")]
    Lfs {
        oid: String,
        object_size: u64,
    },
    /// Over [`OPEN_MAX_BYTES`]. Not read at all.
    TooLarge,
    /// Nothing at that path in that revision, or a folder or submodule.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRead {
    pub path: String,
    /// The commit read from, in full.
    pub revision: String,
    /// The blob's id: what colour is cached by. Empty when nothing is there.
    pub blob: String,
    /// The blob's size in bytes.
    pub size: u64,
    /// The tree-sitter grammar that reads this path, when one is compiled in.
    pub grammar: Option<String>,
    /// False over [`highlight::COLOUR_MAX_BYTES`]: shown, and not coloured.
    pub colour: bool,
    pub content: Content,
}

/// What *Find a file* found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub paths: Vec<String>,
    /// More matched than were returned, or the walk stopped at its limit.
    pub more: bool,
}

fn commit_at<'repo>(repo: &'repo gix::Repository, rev: &str) -> Result<gix::Commit<'repo>> {
    let spec = if rev.trim().is_empty() {
        "HEAD"
    } else {
        rev.trim()
    };
    let unknown = || Error::UnknownCommit(spec.to_string());
    // Peeled: an annotated tag resolves to the tag object, and the files are
    // the commit's.
    repo.rev_parse_single(spec)
        .map_err(|_| unknown())?
        .object()
        .map_err(|_| unknown())?
        .peel_to_commit()
        .map_err(|_| unknown())
}

fn tree_at<'repo>(repo: &'repo gix::Repository, rev: &str) -> Result<gix::Tree<'repo>> {
    commit_at(repo, rev)?
        .tree()
        .map_err(|error| Error::Diff(error.to_string()))
}

/// The commit `rev` means. An empty `rev` is `HEAD`.
pub fn resolve(repo: &gix::Repository, rev: &str) -> Result<Revision> {
    let id = commit_at(repo, rev)?.id;
    let full = id.to_string();
    Ok(Revision {
        short: full.chars().take(7).collect(),
        id: full,
    })
}

fn kind_of(mode: gix::objs::tree::EntryMode) -> EntryKind {
    if mode.is_tree() {
        EntryKind::Folder
    } else if mode.is_commit() {
        EntryKind::Submodule
    } else if mode.is_link() {
        EntryKind::Link
    } else {
        EntryKind::File
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn entries(tree: &gix::Tree<'_>, dir: &str) -> Result<Vec<TreeEntry>> {
    let mut out = Vec::new();
    for entry in tree.iter() {
        let entry = entry.map_err(|error| Error::Diff(error.to_string()))?;
        let name = entry.filename().to_string();
        out.push(TreeEntry {
            path: join(dir, &name),
            name,
            kind: kind_of(entry.mode()),
        });
    }
    // Folders first, then by name without regard to case: how a file tree is
    // read, and not the byte order git stores.
    out.sort_by(|a, b| {
        (a.kind != EntryKind::Folder)
            .cmp(&(b.kind != EntryKind::Folder))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

/// The tree at `dir` in `tree`, or nothing when `dir` is not a folder there.
fn subtree<'repo>(
    repo: &'repo gix::Repository,
    tree: gix::Tree<'repo>,
    dir: &str,
) -> Result<Option<gix::Tree<'repo>>> {
    let dir = dir.trim_matches('/');
    if dir.is_empty() {
        return Ok(Some(tree));
    }
    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(dir))
        .map_err(|error| Error::Diff(error.to_string()))?;
    let Some(entry) = entry.filter(|entry| entry.mode().is_tree()) else {
        return Ok(None);
    };
    repo.find_tree(entry.object_id())
        .map(Some)
        .map_err(|error| Error::Diff(error.to_string()))
}

/// One folder of the repository at `rev`. `dir` is empty for the root.
///
/// One level, read when the folder is opened: a repository of a hundred
/// thousand files costs what the folders somebody looked into cost.
pub fn tree(repo: &gix::Repository, rev: &str, dir: &str) -> Result<Vec<TreeEntry>> {
    let dir = dir.trim_matches('/');
    match subtree(repo, tree_at(repo, rev)?, dir)? {
        Some(tree) => entries(&tree, dir),
        None => Err(Error::UnknownPath(dir.to_string())),
    }
}

/// The object a Git LFS pointer stands for, when `data` is one.
fn lfs_pointer(data: &[u8]) -> Option<(String, u64)> {
    if data.len() > LFS_POINTER_MAX_BYTES {
        return None;
    }
    let text = std::str::from_utf8(data).ok()?;
    let mut lines = text.lines();
    if lines.next()? != LFS_VERSION {
        return None;
    }
    let mut oid = None;
    let mut size = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("oid ") {
            oid = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("size ") {
            size = value.trim().parse::<u64>().ok();
        }
    }
    Some((oid?, size?))
}

/// `path` as it is in `rev`.
///
/// A path that is not a file there is an answer, not an error: a tab kept from
/// another revision is the ordinary way to ask for one.
pub fn read(repo: &gix::Repository, rev: &str, path: &str) -> Result<FileRead> {
    let commit = commit_at(repo, rev)?;
    let revision = commit.id.to_string();
    let tree = commit
        .tree()
        .map_err(|error| Error::Diff(error.to_string()))?;
    let path = path.trim_matches('/');

    let nothing = |content: Content, blob: String, size: u64| FileRead {
        path: path.to_string(),
        revision: revision.clone(),
        blob,
        size,
        grammar: None,
        colour: false,
        content,
    };

    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(path))
        .map_err(|error| Error::Diff(error.to_string()))?;
    let Some(entry) = entry.filter(|entry| entry.mode().is_blob_or_symlink()) else {
        return Ok(nothing(Content::Missing, String::new(), 0));
    };

    let id = entry.object_id();
    let blob = id.to_string();
    let size = repo
        .find_header(id)
        .map_err(|error| Error::Diff(error.to_string()))?
        .size();
    if size > OPEN_MAX_BYTES {
        return Ok(nothing(Content::TooLarge, blob, size));
    }

    let data = repo
        .find_object(id)
        .map_err(|error| Error::Diff(error.to_string()))?
        .detach()
        .data;

    if let Some((oid, object_size)) = lfs_pointer(&data) {
        return Ok(nothing(Content::Lfs { oid, object_size }, blob, size));
    }

    let (mime, is_image) = diff::detect_mime(path);
    // An SVG is text and an image both. It is read here, where its lines can
    // be blamed; a picture of it is the Diff screen's business.
    if is_image && mime != "image/svg+xml" {
        if size > IMAGE_MAX_BYTES {
            return Ok(nothing(Content::Binary, blob, size));
        }
        let base64 = diff::encode_base64(&data);
        return Ok(nothing(Content::Image { mime, base64 }, blob, size));
    }

    if data[..data.len().min(BINARY_SNIFF_BYTES)].contains(&0) {
        return Ok(nothing(Content::Binary, blob, size));
    }

    let colour = size <= highlight::COLOUR_MAX_BYTES;
    let grammar = highlight::grammar_for(path)
        .filter(|grammar| colour && highlight::has_grammar(grammar))
        .map(str::to_string);

    Ok(FileRead {
        path: path.to_string(),
        revision,
        blob,
        size,
        grammar,
        colour,
        content: Content::Text {
            text: String::from_utf8_lossy(&data).into_owned(),
        },
    })
}

/// How well `path` answers `words`: lower is better, `None` is no match.
///
/// Every word has to be in the path. A file whose own name starts with the
/// first word comes first, then one whose name holds it, then the rest.
fn rank(path: &str, words: &[String]) -> Option<u8> {
    let lower = path.to_lowercase();
    if !words.iter().all(|word| lower.contains(word.as_str())) {
        return None;
    }
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    let first = words.first()?;
    Some(if name.starts_with(first.as_str()) {
        0
    } else if name.contains(first.as_str()) {
        1
    } else {
        2
    })
}

/// The files of `rev` whose paths hold every word of `query`, best first.
pub fn find(repo: &gix::Repository, rev: &str, query: &str, limit: usize) -> Result<Found> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() || limit == 0 {
        return Ok(Found {
            paths: Vec::new(),
            more: false,
        });
    }

    let mut matched: Vec<(u8, String)> = Vec::new();
    let mut walked = 0usize;
    let mut cut = false;
    let mut pending = vec![(String::new(), tree_at(repo, rev)?.id)];

    'walk: while let Some((dir, id)) = pending.pop() {
        let tree = repo
            .find_tree(id)
            .map_err(|error| Error::Diff(error.to_string()))?;
        for entry in tree.iter() {
            let entry = entry.map_err(|error| Error::Diff(error.to_string()))?;
            walked += 1;
            if walked > FIND_WALK_MAX {
                cut = true;
                break 'walk;
            }
            let path = join(&dir, &entry.filename().to_string());
            if entry.mode().is_tree() {
                pending.push((path, entry.object_id()));
            } else if entry.mode().is_blob_or_symlink() {
                if let Some(score) = rank(&path, &words) {
                    matched.push((score, path));
                }
            }
        }
    }

    matched.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.len().cmp(&b.1.len()))
            .then_with(|| a.1.cmp(&b.1))
    });
    let more = cut || matched.len() > limit;
    matched.truncate(limit);
    Ok(Found {
        paths: matched.into_iter().map(|(_, path)| path).collect(),
        more,
    })
}

/// How *Search in files* and the search in a file read what was typed
/// (FEAT-151).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchOptions {
    /// A regular expression rather than the text as typed.
    pub regex: bool,
    /// Only where it stands as a word of its own.
    pub word: bool,
    /// Letter case counts. Without it, case counts only when what was typed
    /// has a capital — escapes such as `\S` aside.
    pub case: bool,
}

/// Does `query` ask for its case to count, read as `options` read it?
pub fn case_counts(query: &str, options: SearchOptions) -> bool {
    if options.case {
        return true;
    }
    let mut plain = String::with_capacity(query.len());
    let mut chars = query.chars();
    while let Some(c) = chars.next() {
        if c == '\\' && options.regex {
            chars.next();
        } else {
            plain.push(c);
        }
    }
    plain.chars().any(char::is_uppercase)
}

/// What was typed, as the expression it is matched with. A pattern that is
/// not a regular expression says why.
pub fn pattern(query: &str, options: SearchOptions) -> Result<regex::Regex> {
    let body = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let body = if options.word {
        format!(r"\b(?:{body})\b")
    } else {
        body
    };
    regex::RegexBuilder::new(&body)
        .case_insensitive(!case_counts(query, options))
        .size_limit(1 << 20)
        .build()
        .map_err(|error| Error::Pattern(error.to_string()))
}

/// Above this a file is not searched: a search reads every blob it walks.
const GREP_MAX_BYTES: u64 = 2 * 1024 * 1024;
/// How much of a long line is sent around its match.
const HIT_BEFORE: usize = 80;
const HIT_AFTER: usize = 200;

/// One line *Search in files* found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    /// One-based.
    pub line: u32,
    /// Where the match is in the whole line, in UTF-16 units as the webview
    /// counts them.
    pub column: u32,
    pub length: u32,
    /// The line, or the part of a long one around the match.
    pub text: String,
    /// The match within `text`, in UTF-16 units.
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Searched {
    pub hits: Vec<Hit>,
    /// How many files held a match.
    pub files: u32,
    /// More matched than were returned, or the walk stopped at its limit.
    pub more: bool,
}

fn utf16(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// The match at bytes `start..end` of `line`, as a hit, with a long line
/// cut around it. `number` is zero-based.
pub(crate) fn hit_on(path: &str, number: usize, line: &str, start: usize, end: usize) -> Hit {
    let (mut from, mut to) = (0, line.len());
    if line.len() > HIT_BEFORE + HIT_AFTER {
        from = start.saturating_sub(HIT_BEFORE);
        to = (end + HIT_AFTER).min(line.len());
        while !line.is_char_boundary(from) {
            from -= 1;
        }
        while !line.is_char_boundary(to) {
            to += 1;
        }
    }
    let lead = if from > 0 { "…" } else { "" };
    let text = format!("{lead}{}", &line[from..to]);
    let found = &line[start..end];
    let begins = utf16(lead) + utf16(&line[from..start]);
    Hit {
        path: path.to_string(),
        line: number as u32 + 1,
        column: utf16(&line[..start]),
        length: utf16(found),
        start: begins,
        end: begins + utf16(found),
        text,
    }
}

/// Every blob at `rev`, in path order, and whether the walk stopped at its
/// limit before the end.
pub(crate) fn blobs_at(
    repo: &gix::Repository,
    rev: &str,
) -> Result<(Vec<(String, gix::ObjectId)>, bool)> {
    let mut blobs: Vec<(String, gix::ObjectId)> = Vec::new();
    let mut walked = 0usize;
    let mut cut = false;
    let mut pending = vec![(String::new(), tree_at(repo, rev)?.id)];
    'walk: while let Some((dir, id)) = pending.pop() {
        let tree = repo
            .find_tree(id)
            .map_err(|error| Error::Diff(error.to_string()))?;
        for entry in tree.iter() {
            let entry = entry.map_err(|error| Error::Diff(error.to_string()))?;
            walked += 1;
            if walked > FIND_WALK_MAX {
                cut = true;
                break 'walk;
            }
            let path = join(&dir, &entry.filename().to_string());
            if entry.mode().is_tree() {
                pending.push((path, entry.object_id()));
            } else if entry.mode().is_blob() {
                blobs.push((path, entry.object_id().to_owned()));
            }
        }
    }
    blobs.sort();
    Ok((blobs, cut))
}

/// A blob's text, or `None` for one a search leaves out: binary, a Git LFS
/// pointer, or over 2 MB.
pub(crate) fn searchable_text(repo: &gix::Repository, id: gix::ObjectId) -> Result<Option<String>> {
    let size = repo
        .find_header(id)
        .map_err(|error| Error::Diff(error.to_string()))?
        .size();
    if size > GREP_MAX_BYTES {
        return Ok(None);
    }
    let data = repo
        .find_object(id)
        .map_err(|error| Error::Diff(error.to_string()))?
        .detach()
        .data;
    if data[..data.len().min(BINARY_SNIFF_BYTES)].contains(&0) || lfs_pointer(&data).is_some() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&data).into_owned()))
}

/// Every line of every text file at `rev` that `query` matches, the first
/// match on each, in path order (FEAT-151). Binary files, Git LFS pointers
/// and files over 2 MB are left out. Reads blobs only, like the rest of this
/// module.
pub fn search(
    repo: &gix::Repository,
    rev: &str,
    query: &str,
    options: SearchOptions,
    limit: usize,
) -> Result<Searched> {
    let empty = Searched {
        hits: Vec::new(),
        files: 0,
        more: false,
    };
    if query.is_empty() || limit == 0 {
        return Ok(empty);
    }
    let matcher = pattern(query, options)?;
    let (blobs, mut cut) = blobs_at(repo, rev)?;

    let mut hits = Vec::new();
    let mut files = 0u32;
    'files: for (path, id) in blobs {
        let Some(text) = searchable_text(repo, id)? else {
            continue;
        };
        let mut any = false;
        for (number, line) in text.lines().enumerate() {
            if let Some(found) = matcher.find(line) {
                if hits.len() == limit {
                    cut = true;
                    break 'files;
                }
                any = true;
                hits.push(hit_on(&path, number, line, found.start(), found.end()));
            }
        }
        files += u32::from(any);
    }
    Ok(Searched {
        hits,
        files,
        more: cut,
    })
}

/// Which of what a terminal printed is real (FEAT-130).
///
/// A hex word is a commit when it resolves to one in this repository, and a
/// path is a file when it is one at `HEAD`. The Terminal screen underlines
/// only what comes back from here: a link is checked, never guessed.
///
/// Returns the words that are commits, each with the full id it means, and
/// the paths that are files.
pub fn known(
    repo: &gix::Repository,
    words: &[String],
    paths: &[String],
) -> (Vec<(String, String)>, Vec<String>) {
    let commits = words
        .iter()
        .filter(|word| {
            (7..=40).contains(&word.len()) && word.bytes().all(|b| b.is_ascii_hexdigit())
        })
        .filter_map(|word| {
            let id = repo
                .rev_parse_single(word.as_str())
                .ok()?
                .object()
                .ok()?
                .try_into_commit()
                .ok()?
                .id;
            Some((word.clone(), id.to_string()))
        })
        .collect();

    let tree = tree_at(repo, "HEAD").ok();
    let files = paths
        .iter()
        .filter(|path| {
            let path = path.trim_start_matches("./");
            !path.is_empty()
                && tree.as_ref().is_some_and(|tree| {
                    tree.lookup_entry_by_path(std::path::Path::new(path))
                        .ok()
                        .flatten()
                        .is_some_and(|entry| entry.mode().is_blob_or_symlink())
                })
        })
        .cloned()
        .collect();

    (commits, files)
}

fn bytes_at(repo: &gix::Repository, commit: &str, path: &str) -> Result<Option<Vec<u8>>> {
    let tree = tree_at(repo, commit)?;
    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(path))
        .map_err(|error| Error::Diff(error.to_string()))?;
    let Some(entry) = entry.filter(|entry| entry.mode().is_blob_or_symlink()) else {
        return Ok(None);
    };
    let object = repo
        .find_object(entry.object_id())
        .map_err(|error| Error::Diff(error.to_string()))?;
    Ok(Some(object.detach().data))
}

/// What line `line` of `path` was before `commit` wrote it (FEAT-131's peek).
///
/// `previous` is the commit and path blame says the line's file came from —
/// the parent the change is measured against. With none, the commit added the
/// file and there is no "before".
///
/// The answer is the removed line the commit's hunk replaced: the line's place
/// among the lines its change added is the removed line's place among the ones
/// it took away. A change that only added, or added more than it removed past
/// that place, replaced nothing, and the answer is `None`.
pub fn before(
    repo: &gix::Repository,
    commit: &str,
    path: &str,
    line: u32,
    previous: Option<(&str, &str)>,
) -> Result<Option<String>> {
    let Some((previous_commit, previous_path)) = previous else {
        return Ok(None);
    };
    let Some(new) = bytes_at(repo, commit, path)? else {
        return Ok(None);
    };
    let Some(old) = bytes_at(repo, previous_commit, previous_path)? else {
        return Ok(None);
    };
    if old.len() as u64 > highlight::COLOUR_MAX_BYTES
        || new.len() as u64 > highlight::COLOUR_MAX_BYTES
    {
        return Ok(None);
    }

    for hunk in diff::hunks(&old, &new) {
        let Some(at) = hunk
            .lines
            .iter()
            .position(|row| row.origin == LineOrigin::Added && row.new == Some(line))
        else {
            continue;
        };
        // Back to the start of this run of added lines, then to the run of
        // removed lines immediately before it.
        let mut first_added = at;
        while first_added > 0 && hunk.lines[first_added - 1].origin == LineOrigin::Added {
            first_added -= 1;
        }
        let mut first_removed = first_added;
        while first_removed > 0 && hunk.lines[first_removed - 1].origin == LineOrigin::Removed {
            first_removed -= 1;
        }
        let place = at - first_added;
        let removed = first_added - first_removed;
        return Ok((place < removed).then(|| hunk.lines[first_removed + place].text.clone()));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// Stage everything, new files included, and commit.
    fn save(fixture: &Fixture, message: &str) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit(message)
    }

    fn text_of(read: &FileRead) -> &str {
        match &read.content {
            Content::Text { text } => text,
            other => panic!("not text: {other:?}"),
        }
    }

    fn names(entries: &[TreeEntry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    fn a_tree() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("README.md", "# Thing\n");
        fixture.write("Cargo.toml", "[package]\nname = \"thing\"\n");
        fixture.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
        fixture.write("src/Zed.rs", "// zed\n");
        fixture.write("src/deep/avatars.rs", "pub fn avatar() {}\n");
        fixture.write("docs/guide.md", "Guide\n");
        save(&fixture, "Add a tree");
        fixture
    }

    /// The module's whole claim. Nothing in it can reach a file on disk.
    #[test]
    fn this_module_has_no_way_to_the_working_tree() {
        let source = include_str!("files.rs");
        let code = source
            .split("#[cfg(test)]")
            .next()
            .expect("the code before the tests");

        for forbidden in [
            "std::fs",
            "workdir",
            "work_dir",
            "File::open",
            "read_to_string",
            "shell::",
            "Command",
        ] {
            let found: Vec<&str> = code
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .filter(|line| line.contains(forbidden))
                .collect();
            assert!(found.is_empty(), "{forbidden} is used: {found:?}");
        }
    }

    #[test]
    fn a_file_is_read_from_the_commit_whatever_the_working_copy_holds() {
        let fixture = a_tree();
        let repo = fixture.open();

        // Changed on disk, and not committed.
        fixture.write("src/lib.rs", "something else entirely\n");
        let read_changed = read(&repo, "HEAD", "src/lib.rs").expect("read");
        assert_eq!(text_of(&read_changed), "pub fn one() -> u32 {\n    1\n}\n");

        // Gone from disk.
        fixture.remove("src/lib.rs");
        let read_removed = read(&repo, "HEAD", "src/lib.rs").expect("read");
        assert_eq!(text_of(&read_removed), "pub fn one() -> u32 {\n    1\n}\n");
        assert_eq!(
            names(&tree(&repo, "HEAD", "src").expect("tree")),
            ["deep", "lib.rs", "Zed.rs"]
        );

        // And a file only on disk is not in the tree.
        fixture.write("src/untracked.rs", "fn nope() {}\n");
        assert!(!names(&tree(&repo, "HEAD", "src").expect("tree")).contains(&"untracked.rs"));
        assert_eq!(
            read(&repo, "HEAD", "src/untracked.rs")
                .expect("read")
                .content,
            Content::Missing
        );
    }

    #[test]
    fn a_bare_repository_reads_the_same() {
        let fixture = a_tree();
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
        assert!(repo.is_bare());

        assert_eq!(
            names(&tree(&repo, "main", "").expect("tree")),
            ["docs", "src", "Cargo.toml", "README.md"]
        );
        assert_eq!(
            text_of(&read(&repo, "main", "README.md").expect("read")),
            "# Thing\n"
        );
    }

    #[test]
    fn a_folder_lists_folders_first_then_files_by_name() {
        let fixture = a_tree();
        let repo = fixture.open();

        let root = tree(&repo, "HEAD", "").expect("tree");
        assert_eq!(names(&root), ["docs", "src", "Cargo.toml", "README.md"]);
        assert_eq!(root[0].kind, EntryKind::Folder);
        assert_eq!(root[2].kind, EntryKind::File);

        let src = tree(&repo, "HEAD", "src/").expect("tree");
        // Without regard to case: `lib.rs` before `Zed.rs`.
        assert_eq!(names(&src), ["deep", "lib.rs", "Zed.rs"]);
        assert_eq!(src[1].path, "src/lib.rs");
        assert_eq!(
            tree(&repo, "HEAD", "src/deep").expect("tree")[0].path,
            "src/deep/avatars.rs"
        );
    }

    #[test]
    fn a_folder_that_is_not_there_is_an_error_and_a_file_is_not_a_folder() {
        let fixture = a_tree();
        let repo = fixture.open();

        assert!(tree(&repo, "HEAD", "nowhere").is_err());
        assert!(tree(&repo, "HEAD", "README.md").is_err());
        assert!(tree(&repo, "no-such-branch", "").is_err());
    }

    #[test]
    fn any_revision_reads_the_file_as_it_was_then() {
        let fixture = a_tree();
        let first = fixture.head();
        fixture.git(&["tag", "-a", "v1", "-m", "one"]);
        fixture.write("src/lib.rs", "pub fn one() -> u32 {\n    2\n}\n");
        save(&fixture, "Change one");
        fixture.git(&["checkout", "-q", "-b", "side", &first]);
        fixture.write("only-on-side.txt", "side\n");
        save(&fixture, "Add on side");
        let repo = fixture.open();

        assert!(text_of(&read(&repo, "main", "src/lib.rs").expect("read")).contains("    2"));
        assert!(text_of(&read(&repo, &first, "src/lib.rs").expect("read")).contains("    1"));
        // An annotated tag is peeled to its commit.
        let tagged = read(&repo, "v1", "src/lib.rs").expect("read");
        assert!(text_of(&tagged).contains("    1"));
        assert_eq!(tagged.revision, first);
        assert_eq!(
            read(&repo, "main", "only-on-side.txt")
                .expect("read")
                .content,
            Content::Missing
        );
        assert_eq!(
            text_of(&read(&repo, "side", "only-on-side.txt").expect("read")),
            "side\n"
        );

        let resolved = resolve(&repo, "v1").expect("resolve");
        assert_eq!(resolved.id, first);
        assert_eq!(resolved.short, first[..7]);
        assert_eq!(resolve(&repo, "").expect("resolve").id, fixture.head());
        assert!(resolve(&repo, "nope").is_err());
    }

    #[test]
    fn a_text_file_names_its_blob_its_size_and_its_grammar() {
        let fixture = a_tree();
        let repo = fixture.open();

        let read_rs = read(&repo, "HEAD", "src/lib.rs").expect("read");
        assert_eq!(read_rs.blob, fixture.rev("HEAD:src/lib.rs"));
        assert_eq!(read_rs.size, 30);
        assert_eq!(read_rs.grammar.as_deref(), Some("rust"));
        assert!(read_rs.colour);
        assert_eq!(read_rs.revision, fixture.head());

        // No grammar compiled in: the frontend's highlighter takes it.
        fixture.write("Main.kt", "fun main() {}\n");
        fixture.write("LICENSE", "words\n");
        save(&fixture, "Add more");
        assert_eq!(read(&repo, "HEAD", "Main.kt").expect("read").grammar, None);
        assert_eq!(read(&repo, "HEAD", "LICENSE").expect("read").grammar, None);
    }

    #[test]
    fn a_folder_or_nothing_at_the_path_is_missing() {
        let fixture = a_tree();
        let repo = fixture.open();

        for path in ["src", "nope.rs", "src/nope/deeper.rs", ""] {
            let read = read(&repo, "HEAD", path).expect("read");
            assert_eq!(read.content, Content::Missing, "{path}");
            assert_eq!(read.blob, "");
        }
    }

    #[test]
    fn a_binary_file_and_an_image_say_what_they_are() {
        let fixture = Fixture::empty();
        fixture.write_bytes("data.bin", &[1, 2, 0, 3, 4]);
        fixture.write_bytes("dot.png", &[0x89, b'P', b'N', b'G', 0, 1, 2]);
        fixture.write("logo.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n");
        save(&fixture, "Add binaries");
        let repo = fixture.open();

        let binary = read(&repo, "HEAD", "data.bin").expect("read");
        assert_eq!(binary.content, Content::Binary);
        assert_eq!(binary.size, 5);

        match read(&repo, "HEAD", "dot.png").expect("read").content {
            Content::Image { mime, base64 } => {
                assert_eq!(mime, "image/png");
                assert_eq!(
                    base64,
                    diff::encode_base64(&[0x89, b'P', b'N', b'G', 0, 1, 2])
                );
            }
            other => panic!("not an image: {other:?}"),
        }

        // An SVG is text: its lines can be read and blamed.
        assert!(text_of(&read(&repo, "HEAD", "logo.svg").expect("read")).starts_with("<svg"));
    }

    #[test]
    fn a_git_lfs_pointer_is_shown_as_a_pointer() {
        let fixture = Fixture::empty();
        let oid = "sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";
        fixture.write(
            "big.psd",
            &format!("{LFS_VERSION}\noid {oid}\nsize 12345678\n"),
        );
        fixture.write("not-a-pointer.txt", "version 2\noid none\n");
        save(&fixture, "Add a pointer");
        let repo = fixture.open();

        assert_eq!(
            read(&repo, "HEAD", "big.psd").expect("read").content,
            Content::Lfs {
                oid: oid.to_string(),
                object_size: 12_345_678
            }
        );
        assert!(matches!(
            read(&repo, "HEAD", "not-a-pointer.txt")
                .expect("read")
                .content,
            Content::Text { .. }
        ));
        assert_eq!(lfs_pointer(LFS_VERSION.as_bytes()), None);
        assert_eq!(lfs_pointer(&vec![b'a'; LFS_POINTER_MAX_BYTES + 1]), None);
    }

    #[test]
    fn the_limits_are_the_ones_the_item_names() {
        assert_eq!(highlight::COLOUR_MAX_BYTES, 5 * 1024 * 1024);
        assert_eq!(OPEN_MAX_BYTES, 50 * 1024 * 1024);
    }

    #[test]
    fn a_file_over_the_colour_limit_is_read_without_colour() {
        let fixture = Fixture::empty();
        let line = "let x = 1; // padding padding padding padding padding padding\n";
        let big = line.repeat((highlight::COLOUR_MAX_BYTES as usize / line.len()) + 2);
        fixture.write("big.rs", &big);
        save(&fixture, "Add a big file");
        let repo = fixture.open();

        let read_big = read(&repo, "HEAD", "big.rs").expect("read");
        assert!(read_big.size > highlight::COLOUR_MAX_BYTES);
        assert!(!read_big.colour);
        assert_eq!(read_big.grammar, None);
        assert_eq!(text_of(&read_big).len(), big.len());
        // And asking for its colour anyway answers with none.
        assert!(highlight::spans(&repo, &read_big.blob, "rust", 0, 10)
            .expect("spans")
            .is_empty());
    }

    fn searched(repo: &gix::Repository, query: &str, options: SearchOptions) -> Vec<String> {
        search(repo, "HEAD", query, options, 100)
            .expect("search")
            .hits
            .iter()
            .map(|hit| format!("{}:{}", hit.path, hit.line))
            .collect()
    }

    #[test]
    fn search_finds_text_in_every_file_at_the_revision() {
        let fixture = a_tree();
        let repo = fixture.open();
        let plain = SearchOptions::default();
        assert_eq!(
            searched(&repo, "pub fn", plain),
            ["src/deep/avatars.rs:1", "src/lib.rs:1"]
        );
        // Smart case: lower case finds any case, a capital asks for its own.
        assert_eq!(searched(&repo, "guide", plain), ["docs/guide.md:1"]);
        assert!(searched(&repo, "GUIDE", plain).is_empty());
        let counted = search(&repo, "HEAD", "pub fn", plain, 100).expect("search");
        assert_eq!((counted.files, counted.more), (2, false));
    }

    #[test]
    fn search_reads_a_regex_a_whole_word_and_case() {
        let fixture = a_tree();
        let repo = fixture.open();
        let regex = SearchOptions {
            regex: true,
            ..SearchOptions::default()
        };
        assert_eq!(
            searched(&repo, r"fn \w+\(\) -> u32", regex),
            ["src/lib.rs:1"]
        );
        // An escape is not a capital asking for case.
        assert_eq!(searched(&repo, r"\S+ thing", regex), ["README.md:1"]);
        let word = SearchOptions {
            word: true,
            ..SearchOptions::default()
        };
        assert_eq!(searched(&repo, "avatar", word), ["src/deep/avatars.rs:1"]);
        assert!(searched(&repo, "avat", word).is_empty());
        let case = SearchOptions {
            case: true,
            ..SearchOptions::default()
        };
        assert!(searched(&repo, "thing", case)
            .iter()
            .all(|hit| hit != "README.md:1"));
        assert_eq!(searched(&repo, "Thing", case), ["README.md:1"]);
    }

    #[test]
    fn a_pattern_that_is_not_a_regex_says_so() {
        let fixture = a_tree();
        let repo = fixture.open();
        let regex = SearchOptions {
            regex: true,
            ..SearchOptions::default()
        };
        let refused = search(&repo, "HEAD", "fn (", regex, 10).unwrap_err();
        assert!(
            refused.to_string().starts_with("not a pattern:"),
            "{refused}"
        );
        // As plain text the same characters are only characters.
        assert!(search(&repo, "HEAD", "fn (", SearchOptions::default(), 10).is_ok());
    }

    #[test]
    fn search_leaves_binary_files_out_and_says_when_there_is_more() {
        let fixture = a_tree();
        fixture.write("bin/blob.bin", "pub fn\0hidden\n");
        fixture.write("many.txt", &"needle\n".repeat(10));
        save(&fixture, "Binary and many");
        let repo = fixture.open();
        assert!(!searched(&repo, "pub fn", SearchOptions::default())
            .iter()
            .any(|hit| hit.starts_with("bin/")));
        let few = search(&repo, "HEAD", "needle", SearchOptions::default(), 4).expect("search");
        assert_eq!((few.hits.len(), few.more), (4, true));
    }

    #[test]
    fn a_hit_says_where_its_match_is_in_utf16_units() {
        let fixture = a_tree();
        let long = format!("é{}needle{}", "x".repeat(200), "y".repeat(300));
        fixture.write("wide.txt", &format!("café needle\n{long}\n"));
        save(&fixture, "Wide");
        let repo = fixture.open();
        let hits = search(&repo, "HEAD", "needle", SearchOptions::default(), 10)
            .expect("search")
            .hits;
        let short = &hits[0];
        assert_eq!((short.line, short.column, short.length), (1, 5, 6));
        assert_eq!(&short.text, "café needle");
        assert_eq!((short.start, short.end), (5, 11));
        // A long line is cut around its match, and the match is still marked.
        let cut = &hits[1];
        assert_eq!(cut.column, 201);
        assert!(cut.text.starts_with('…'));
        let marked: String = cut
            .text
            .encode_utf16()
            .skip(cut.start as usize)
            .take((cut.end - cut.start) as usize)
            .map(|unit| char::from_u32(unit as u32).unwrap())
            .collect();
        assert_eq!(marked, "needle");
    }

    #[test]
    fn find_ranks_a_name_that_starts_with_the_word_first() {
        let fixture = a_tree();
        fixture.write("src/deep/lib_helpers.rs", "\n");
        fixture.write("tools/calibrate.rs", "\n");
        save(&fixture, "More files");
        let repo = fixture.open();

        let found = find(&repo, "HEAD", "lib", 10).expect("find");
        assert_eq!(
            found.paths,
            [
                "src/lib.rs",
                "src/deep/lib_helpers.rs",
                "tools/calibrate.rs"
            ]
        );
        assert!(!found.more);

        // Every word has to be there, in any order and any case.
        assert_eq!(
            find(&repo, "HEAD", "DEEP avatars", 10).expect("find").paths,
            ["src/deep/avatars.rs"]
        );
        assert!(find(&repo, "HEAD", "zzz", 10)
            .expect("find")
            .paths
            .is_empty());
        assert!(find(&repo, "HEAD", "   ", 10)
            .expect("find")
            .paths
            .is_empty());

        let capped = find(&repo, "HEAD", "rs", 2).expect("find");
        assert_eq!(capped.paths.len(), 2);
        assert!(capped.more);
    }

    #[test]
    fn before_is_the_line_the_commit_replaced() {
        let fixture = Fixture::empty();
        fixture.write("a.rs", "one\ntwo\nthree\nfour\n");
        let first = save(&fixture, "Add");
        fixture.write("a.rs", "one\nTWO\nTHREE\nextra\nfour\nfive\n");
        let second = save(&fixture, "Rewrite the middle");
        let repo = fixture.open();
        let previous = Some((first.as_str(), "a.rs"));

        assert_eq!(
            before(&repo, &second, "a.rs", 2, previous).expect("before"),
            Some("two".to_string())
        );
        assert_eq!(
            before(&repo, &second, "a.rs", 3, previous).expect("before"),
            Some("three".to_string())
        );
        // Added past what was removed: it replaced nothing.
        assert_eq!(
            before(&repo, &second, "a.rs", 4, previous).expect("before"),
            None
        );
        // A line added at the end, with nothing removed.
        assert_eq!(
            before(&repo, &second, "a.rs", 6, previous).expect("before"),
            None
        );
        // A line the commit did not touch.
        assert_eq!(
            before(&repo, &second, "a.rs", 1, previous).expect("before"),
            None
        );
        // The commit that added the file has no before.
        assert_eq!(
            before(&repo, &first, "a.rs", 1, None).expect("before"),
            None
        );
    }

    /// The terminal underlines only what is real (FEAT-130).
    #[test]
    fn a_hex_word_is_a_commit_only_when_it_resolves_and_a_path_only_when_it_is_a_file() {
        let fixture = a_tree();
        let head = fixture.head();
        let repo = fixture.open();
        let own = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };

        let (commits, files) = known(
            &repo,
            &own(&[&head[..7], &head, "deadbee", "abc", "nothexx", &head[..6]]),
            &own(&[
                "src/lib.rs",
                "./README.md",
                "src",
                "nope.rs",
                "",
                "src/deep/avatars.rs",
            ]),
        );

        assert_eq!(
            commits,
            [
                (head[..7].to_string(), head.clone()),
                (head.clone(), head.clone())
            ]
        );
        // A folder is not a file, and neither is a path that is not there.
        assert_eq!(files, ["src/lib.rs", "./README.md", "src/deep/avatars.rs"]);
    }

    #[test]
    fn a_hex_word_that_names_a_blob_is_not_a_commit() {
        let fixture = a_tree();
        let blob = fixture.rev("HEAD:README.md");
        let repo = fixture.open();

        let (commits, _) = known(&repo, &[blob[..10].to_string()], &[]);

        assert!(commits.is_empty());
    }

    #[test]
    fn before_follows_the_file_to_the_name_it_had() {
        let fixture = Fixture::empty();
        fixture.write("old.rs", "alpha\nbeta\ngamma\ndelta\nepsilon\n");
        let first = save(&fixture, "Add");
        fixture.git(&["mv", "old.rs", "new.rs"]);
        fixture.write("new.rs", "alpha\nBETA\ngamma\ndelta\nepsilon\n");
        let second = save(&fixture, "Rename and edit");
        let repo = fixture.open();

        assert_eq!(
            before(
                &repo,
                &second,
                "new.rs",
                2,
                Some((first.as_str(), "old.rs"))
            )
            .expect("before"),
            Some("beta".to_string())
        );
        // Asked against a path that was not there, there is nothing to say.
        assert_eq!(
            before(
                &repo,
                &second,
                "new.rs",
                2,
                Some((first.as_str(), "new.rs"))
            )
            .expect("before"),
            None
        );
    }
}

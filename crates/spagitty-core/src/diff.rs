// SPDX-License-Identifier: GPL-3.0-or-later

//! What a commit changed.
//!
//! Three levels, because they cost different amounts:
//!
//! - [`commit_detail`] — message, people, parents, and the list of paths. This
//!   is what fills the 270px detail panel on the Graph screen. No blob is read.
//! - [`commit_diff`] — the same list plus `+n −m` per file, which means reading
//!   and line-diffing every changed blob. The Diff screen needs it for its file
//!   list and its header count.
//! - [`file_diff`] — the hunks of a single file, fetched as that file is
//!   opened. A 400-file commit therefore does not build 400 files' worth of
//!   hunks to show the first one.

use gix::diff::blob;
use gix::ObjectId;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::graph::short_id;

/// Context lines kept either side of a change. Three is git's default, and the
/// number people are used to reading.
const CONTEXT: u32 = 3;

/// Above this, a blob is reported as too large rather than diffed. Two copies
/// of the file plus the interner's line index all live in memory at once, so
/// the real cost of a diff is several times the file size; a repository with a
/// 200MB asset in it should not be able to freeze the window.
const MAX_BLOB_BYTES: usize = 8 * 1024 * 1024;

/// How far into a blob we look for a NUL byte before calling it text. This is
/// git's rule (`xdiff`'s `FIRST_FEW_BYTES`), so Spagitty calls the same files
/// binary that `git diff` does.
const BINARY_SNIFF_BYTES: usize = 8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    /// Present in the working tree and not in the index. Only ever produced by
    /// the working-copy status walk; a commit cannot contain one.
    Untracked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub id: String,
    pub short: String,
    /// First line of the message.
    pub summary: String,
    /// Everything after the first line, trimmed. Often empty.
    pub body: String,
    pub author_name: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub commit_time: i64,
    /// The commit carries a signature. See [`crate::signing`] — present, not
    /// verified.
    pub signed: bool,
    pub parents: Vec<String>,
    pub files: Vec<ChangedFile>,
}

/// Where a line came from. `Context` lines appear in both versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineOrigin {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub origin: LineOrigin,
    /// 1-based line number in the old version. `None` for an added line.
    pub old: Option<u32>,
    /// 1-based line number in the new version. `None` for a removed line.
    pub new: Option<u32>,
    /// The line's text, without its terminator.
    pub text: String,
}

/// One run of changes plus its surrounding context — a `@@` block.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// `@@ -12,7 +12,9 @@`, for the hunk header row.
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// One file's entry in the Diff screen's file list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: FileStatus,
    /// No line diff exists for this file: it is binary, or one side was over
    /// [`MAX_BLOB_BYTES`]. `added` and `removed` are 0 in both cases, which is
    /// the truth rather than a guess.
    pub binary: bool,
    pub too_large: bool,
    pub added: u32,
    pub removed: u32,
}

/// Everything the Diff screen's header and file list need.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDiff {
    pub id: String,
    pub short: String,
    pub summary: String,
    pub files: Vec<FileChange>,
    /// Totals across every file, for the header's `+n −m`.
    pub added: u32,
    pub removed: u32,
}

/// One file's hunks, fetched when that file is opened.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub binary: bool,
    pub too_large: bool,
    pub added: u32,
    pub removed: u32,
    pub hunks: Vec<Hunk>,
}

/// Everything the detail panel shows for one commit.
///
/// The file list is the diff against the **first** parent. For a merge that is
/// the conventional view — the changes the merge brought onto the branch it
/// landed on — and it is what `git show` does by default.
pub fn commit_detail(repo: &gix::Repository, id: &str) -> Result<CommitDetail> {
    let commit = find_commit(repo, id)?;

    let message = commit.message().map_err(|e| Error::Diff(e.to_string()))?;
    let summary = message.summary().to_string();
    let body = message
        .body()
        .map(|b| b.to_string())
        .unwrap_or_default()
        .trim()
        .to_string();

    let author = commit.author().map_err(|e| Error::Diff(e.to_string()))?;
    let committer = commit.committer().map_err(|e| Error::Diff(e.to_string()))?;

    let parents: Vec<ObjectId> = commit.parent_ids().map(|id| id.detach()).collect();
    let files = tree_changes(repo, &commit, parents.first().copied())?
        .into_iter()
        .map(|change| ChangedFile {
            path: change.path,
            status: change.status,
        })
        .collect();

    Ok(CommitDetail {
        id: commit.id().to_string(),
        short: short_id(&commit.id().detach()),
        summary,
        body,
        author_name: author.name.to_string(),
        author_email: author.email.to_string(),
        author_time: author.time().map(|t| t.seconds).unwrap_or(0),
        committer_name: committer.name.to_string(),
        committer_email: committer.email.to_string(),
        commit_time: committer.time().map(|t| t.seconds).unwrap_or(0),
        signed: crate::signing::signed(&commit),
        parents: parents.iter().map(ObjectId::to_string).collect(),
        files,
    })
}

/// The file list with per-file line counts.
///
/// Every changed blob is read and line-diffed here, because `+n −m` cannot be
/// known any other way. That is the price of the counts the design asks for,
/// and it is paid once when the screen opens rather than per keystroke.
pub fn commit_diff(repo: &gix::Repository, id: &str) -> Result<CommitDiff> {
    let commit = find_commit(repo, id)?;
    let summary = commit
        .message()
        .map_err(|e| Error::Diff(e.to_string()))?
        .summary()
        .to_string();

    let parent = commit.parent_ids().next().map(|id| id.detach());
    let changes = tree_changes(repo, &commit, parent)?;

    let mut files = Vec::with_capacity(changes.len());
    let mut added = 0;
    let mut removed = 0;

    for change in changes {
        let old = blob_bytes(repo, change.old)?;
        let new = blob_bytes(repo, change.new)?;
        let stats = line_stats(old.as_deref(), new.as_deref());

        added += stats.added;
        removed += stats.removed;

        files.push(FileChange {
            path: change.path,
            status: change.status,
            binary: stats.binary,
            too_large: stats.too_large,
            added: stats.added,
            removed: stats.removed,
        });
    }

    Ok(CommitDiff {
        id: commit.id().to_string(),
        short: short_id(&commit.id().detach()),
        summary,
        files,
        added,
        removed,
    })
}

/// The hunks for one path in one commit, against its first parent.
///
/// The path is looked up directly in both trees rather than by walking the
/// whole diff again: opening the twentieth file of a commit should not cost
/// what opening the whole commit did.
pub fn file_diff(repo: &gix::Repository, id: &str, path: &str) -> Result<FileDiff> {
    let commit = find_commit(repo, id)?;
    let parent = commit.parent_ids().next().map(|id| id.detach());

    let tree = commit.tree().map_err(|e| Error::Diff(e.to_string()))?;
    let new = blob_at(&tree, path)?;

    let old = match parent {
        Some(pid) => {
            let parent = repo
                .find_commit(pid)
                .map_err(|e| Error::Diff(e.to_string()))?;
            let parent_tree = parent.tree().map_err(|e| Error::Diff(e.to_string()))?;
            blob_at(&parent_tree, path)?
        }
        None => None,
    };

    let status = match (old, new) {
        (None, None) => return Err(Error::UnknownPath(path.to_string())),
        (None, Some(_)) => FileStatus::Added,
        (Some(_), None) => FileStatus::Deleted,
        (Some(_), Some(_)) => FileStatus::Modified,
    };

    let old = blob_bytes(repo, old)?;
    let new = blob_bytes(repo, new)?;
    let stats = line_stats(old.as_deref(), new.as_deref());

    let hunks = if stats.binary || stats.too_large {
        Vec::new()
    } else {
        hunks(
            old.as_deref().unwrap_or_default(),
            new.as_deref().unwrap_or_default(),
        )
    };

    Ok(FileDiff {
        path: path.to_string(),
        status,
        binary: stats.binary,
        too_large: stats.too_large,
        added: stats.added,
        removed: stats.removed,
        hunks,
    })
}

// --- The working copy -----------------------------------------------------

/// Which two sides of the working copy a diff compares.
///
/// The Working copy screen shows both at once — what is staged, and what is
/// not — and they are the same diff machinery pointed at different pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// `HEAD` against the index: what a commit right now would contain.
    Staged,
    /// The index against the working tree: what a commit right now would leave
    /// behind.
    Unstaged,
}

/// One file's hunks between two sides of the working copy.
///
/// The same [`FileDiff`] the Diff screen renders, so the hunk pane is one
/// component rather than two.
pub fn working_file_diff(repo: &gix::Repository, path: &str, side: Side) -> Result<FileDiff> {
    let (old, new) = match side {
        Side::Staged => (head_blob(repo, path)?, index_bytes(repo, path)?),
        Side::Unstaged => (index_bytes(repo, path)?, worktree_bytes(repo, path)?),
    };

    let status = match (old.is_some(), new.is_some()) {
        (false, false) => return Err(Error::UnknownPath(path.to_string())),
        (false, true) => FileStatus::Added,
        (true, false) => FileStatus::Deleted,
        (true, true) => FileStatus::Modified,
    };

    let stats = line_stats(old.as_deref(), new.as_deref());
    let hunks = if stats.binary || stats.too_large {
        Vec::new()
    } else {
        hunks(
            old.as_deref().unwrap_or_default(),
            new.as_deref().unwrap_or_default(),
        )
    };

    Ok(FileDiff {
        path: path.to_string(),
        status,
        binary: stats.binary,
        too_large: stats.too_large,
        added: stats.added,
        removed: stats.removed,
        hunks,
    })
}

/// A patch containing exactly one hunk of one file, ready for `git apply`.
///
/// The hunk is identified by its position and its header rather than sent as
/// text from the UI, for two reasons. The patch has to be built from the bytes
/// as they are *now*, so a file that changed under an open screen cannot be
/// half-applied from a stale view. And the header check is what turns that
/// staleness into a plain refusal instead of a wrong result.
pub fn working_hunk_patch(
    repo: &gix::Repository,
    path: &str,
    side: Side,
    index: usize,
    expect_header: &str,
) -> Result<String> {
    let (old, new) = match side {
        Side::Staged => (head_blob(repo, path)?, index_bytes(repo, path)?),
        Side::Unstaged => (index_bytes(repo, path)?, worktree_bytes(repo, path)?),
    };

    let old_bytes = old.as_deref().unwrap_or_default();
    let new_bytes = new.as_deref().unwrap_or_default();

    if is_binary(old_bytes) || is_binary(new_bytes) {
        return Err(Error::NotStageable(
            "a binary file has no hunks to stage".into(),
        ));
    }

    let hunks = hunks(old_bytes, new_bytes);
    let Some(hunk) = hunks.get(index) else {
        return Err(Error::Stale(path.to_string()));
    };
    if hunk.header != expect_header {
        return Err(Error::Stale(path.to_string()));
    }

    Ok(format_patch(path, old, new, hunk))
}

/// Render one hunk as a unified diff `git apply` will accept.
fn format_patch(path: &str, old: Option<Vec<u8>>, new: Option<Vec<u8>>, hunk: &Hunk) -> String {
    let mut out = String::new();
    out.push_str(&format!("diff --git a/{path} b/{path}\n"));
    out.push_str(&match old {
        Some(_) => format!("--- a/{path}\n"),
        None => "--- /dev/null\n".to_string(),
    });
    out.push_str(&match new {
        Some(_) => format!("+++ b/{path}\n"),
        None => "+++ /dev/null\n".to_string(),
    });
    out.push_str(&hunk.header);
    out.push('\n');

    let old_bytes = old.as_deref().unwrap_or_default();
    let new_bytes = new.as_deref().unwrap_or_default();
    // A file whose last line has no terminator has to say so, or applying the
    // patch would quietly add one — a content change nobody asked for.
    let old_unterminated = last_line_of(old_bytes);
    let new_unterminated = last_line_of(new_bytes);

    for line in &hunk.lines {
        let sign = match line.origin {
            LineOrigin::Context => ' ',
            LineOrigin::Added => '+',
            LineOrigin::Removed => '-',
        };
        out.push(sign);
        out.push_str(&line.text);
        out.push('\n');

        let ends_a_file = match line.origin {
            LineOrigin::Added => line.new == new_unterminated,
            LineOrigin::Removed => line.old == old_unterminated,
            // A context line is the same line on both sides; it only lacks a
            // terminator if both sides end there.
            LineOrigin::Context => line.old == old_unterminated && line.new == new_unterminated,
        };
        if ends_a_file {
            out.push_str("\\ No newline at end of file\n");
        }
    }

    out
}

/// The 1-based number of the final line when the data does not end in a
/// newline, or `None` when it does (or is empty).
fn last_line_of(data: &[u8]) -> Option<u32> {
    if data.is_empty() || data.ends_with(b"\n") {
        return None;
    }
    Some(data.iter().filter(|b| **b == b'\n').count() as u32 + 1)
}

/// The blob `HEAD` has at `path`. `None` in an unborn repository, which is a
/// repository where everything staged is an addition.
fn head_blob(repo: &gix::Repository, path: &str) -> Result<Option<Vec<u8>>> {
    let Ok(id) = repo.head_id() else {
        return Ok(None);
    };
    let commit = repo
        .find_commit(id.detach())
        .map_err(|e| Error::Diff(e.to_string()))?;
    let tree = commit.tree().map_err(|e| Error::Diff(e.to_string()))?;
    blob_bytes(repo, blob_at(&tree, path)?)
}

/// The blob the index holds at `path`.
///
/// Stage 0 only: a conflicted path has stages 1 to 3 and no single "index
/// version", which is the Conflicts screen's problem rather than this one's.
fn index_bytes(repo: &gix::Repository, path: &str) -> Result<Option<Vec<u8>>> {
    let index = repo
        .index_or_empty()
        .map_err(|e| Error::Diff(e.to_string()))?;

    let Some(entry) = index.entry_by_path(path.into()) else {
        return Ok(None);
    };
    // A submodule or a sparse directory entry has no line diff, the same way a
    // tree entry that is not a blob does not.
    if entry.mode.is_submodule() || entry.mode.is_sparse() {
        return Ok(None);
    }
    blob_bytes(repo, Some(entry.id))
}

/// The file on disk at `path`, as git would store it.
///
/// A path that is not there reads as `None` — a deletion, not an error — and so
/// does a directory, since a directory has no line diff.
///
/// **Converted, not raw** (BUG-033). The other side of every unstaged diff is
/// a blob, and a blob holds what `git add` would have written: line endings
/// normalised by `core.autocrlf` and `.gitattributes`, and any clean filter
/// applied. Git runs the same conversion on the working file before it diffs.
/// Compared raw, a CRLF checkout — the default on Windows — differs from its
/// blob on every line, and one changed line showed as the whole file deleted
/// and written again. The hunks built from this diff are applied by
/// `git apply`, which reads the working file through the same conversion, so
/// staging and discarding a hunk agree with what is shown.
fn worktree_bytes(repo: &gix::Repository, path: &str) -> Result<Option<Vec<u8>>> {
    let Some(workdir) = repo.workdir() else {
        return Ok(None);
    };
    let full = workdir.join(path);
    // Checked before reading rather than by matching the error afterwards:
    // `ErrorKind::IsADirectory` is newer than this crate's MSRV.
    if full.is_dir() {
        return Ok(None);
    }

    match std::fs::read(&full) {
        Ok(bytes) => to_git(repo, path, bytes).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io(e)),
    }
}

/// `bytes`, read from the working file at `path`, converted to what the
/// repository would store for it.
fn to_git(repo: &gix::Repository, path: &str, bytes: Vec<u8>) -> Result<Vec<u8>> {
    use gix::filter::plumbing::pipeline::convert::ToGitOutcome;
    use std::io::Read;

    let (mut pipeline, index) = repo
        .filter_pipeline(None)
        .map_err(|e| Error::Diff(e.to_string()))?;
    let converted = match pipeline
        .convert_to_git(&bytes[..], std::path::Path::new(path), &index)
        .map_err(|e| Error::Diff(e.to_string()))?
    {
        ToGitOutcome::Unchanged(_) => None,
        ToGitOutcome::Buffer(buffer) => Some(buffer.to_vec()),
        ToGitOutcome::Process(mut stream) => {
            let mut out = Vec::new();
            stream.read_to_end(&mut out).map_err(Error::Io)?;
            Some(out)
        }
    };
    Ok(converted.unwrap_or(bytes))
}

// --- Tree walking ---------------------------------------------------------

/// One changed path with both sides' blob ids, which is what a text diff needs
/// and what the public [`ChangedFile`] deliberately does not carry.
struct RawChange {
    path: String,
    status: FileStatus,
    /// `None` when the file did not exist on that side.
    old: Option<ObjectId>,
    new: Option<ObjectId>,
}

/// The files that differ between two commits — `base` to `head`, as
/// `git diff --name-status base head` would list them (FEAT-096).
///
/// A review of a committed range is described by this list before anything
/// is sent anywhere, so it is read in-process like every other diff here.
pub fn range_files(repo: &gix::Repository, base: &str, head: &str) -> Result<Vec<ChangedFile>> {
    let old = find_commit(repo, base)?
        .tree()
        .map_err(|e| Error::Diff(e.to_string()))?;
    let new = find_commit(repo, head)?
        .tree()
        .map_err(|e| Error::Diff(e.to_string()))?;
    Ok(trees_changes(&old, &new)?
        .into_iter()
        .map(|change| ChangedFile {
            path: change.path,
            status: change.status,
        })
        .collect())
}

fn find_commit<'repo>(repo: &'repo gix::Repository, id: &str) -> Result<gix::Commit<'repo>> {
    let oid =
        ObjectId::from_hex(id.as_bytes()).map_err(|_| Error::UnknownCommit(id.to_string()))?;
    repo.find_commit(oid)
        .map_err(|_| Error::UnknownCommit(id.to_string()))
}

/// Tree diff of `commit` against `parent`, or against the empty tree for a root
/// commit — which is how a root commit correctly reads as "added everything".
fn tree_changes(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    parent: Option<ObjectId>,
) -> Result<Vec<RawChange>> {
    let tree = commit.tree().map_err(|e| Error::Diff(e.to_string()))?;
    let parent_tree = match parent {
        Some(pid) => {
            let parent = repo
                .find_commit(pid)
                .map_err(|e| Error::Diff(e.to_string()))?;
            parent.tree().map_err(|e| Error::Diff(e.to_string()))?
        }
        None => repo.empty_tree(),
    };
    trees_changes(&parent_tree, &tree)
}

/// Every changed file between two trees, the older first.
fn trees_changes(old: &gix::Tree<'_>, new: &gix::Tree<'_>) -> Result<Vec<RawChange>> {
    use gix::object::tree::diff::Change;

    let mut changes: Vec<RawChange> = Vec::new();

    old
        .changes()
        .map_err(|e| Error::Diff(e.to_string()))?
        .for_each_to_obtain_tree(new, |change| {
            // A changed subtree is reported alongside the blobs inside it. Only
            // the blobs are files; a directory row in the file list would be a
            // row nothing can be done with.
            let mode = match &change {
                Change::Addition { entry_mode, .. }
                | Change::Deletion { entry_mode, .. }
                | Change::Modification { entry_mode, .. }
                | Change::Rewrite { entry_mode, .. } => *entry_mode,
            };
            if !mode.is_blob_or_symlink() {
                return Ok(std::ops::ControlFlow::Continue(()));
            }

            changes.push(match change {
                Change::Addition { location, id, .. } => RawChange {
                    path: location.to_string(),
                    status: FileStatus::Added,
                    old: None,
                    new: Some(id.detach()),
                },
                Change::Deletion { location, id, .. } => RawChange {
                    path: location.to_string(),
                    status: FileStatus::Deleted,
                    old: Some(id.detach()),
                    new: None,
                },
                Change::Modification {
                    location,
                    previous_id,
                    id,
                    ..
                } => RawChange {
                    path: location.to_string(),
                    status: FileStatus::Modified,
                    old: Some(previous_id.detach()),
                    new: Some(id.detach()),
                },
                Change::Rewrite {
                    location,
                    source_id,
                    id,
                    ..
                } => RawChange {
                    path: location.to_string(),
                    status: FileStatus::Renamed,
                    old: Some(source_id.detach()),
                    new: Some(id.detach()),
                },
            });
            Ok::<_, std::convert::Infallible>(std::ops::ControlFlow::Continue(()))
        })
        .map_err(|e| Error::Diff(e.to_string()))?;

    changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(changes)
}

/// The blob id at `path`, or `None` when nothing is there. A directory or a
/// submodule at that path is also `None`: neither has a line diff.
fn blob_at(tree: &gix::Tree<'_>, path: &str) -> Result<Option<ObjectId>> {
    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(path))
        .map_err(|e| Error::Diff(e.to_string()))?;

    Ok(entry
        .filter(|entry| entry.mode().is_blob_or_symlink())
        .map(|entry| entry.object_id()))
}

/// A missing side reads as empty, which is what makes an addition diff against
/// nothing rather than needing its own code path.
fn blob_bytes(repo: &gix::Repository, id: Option<ObjectId>) -> Result<Option<Vec<u8>>> {
    let Some(id) = id else { return Ok(None) };
    let object = repo
        .find_object(id)
        .map_err(|e| Error::Diff(e.to_string()))?;
    Ok(Some(object.detach().data))
}

// --- Text diff ------------------------------------------------------------

struct Stats {
    added: u32,
    removed: u32,
    binary: bool,
    too_large: bool,
}

/// Git's rule: a NUL byte near the start means binary.
fn is_binary(data: &[u8]) -> bool {
    data[..data.len().min(BINARY_SNIFF_BYTES)].contains(&0)
}

/// Line counts for one file, and the two reasons there may not be any.
fn line_stats(old: Option<&[u8]>, new: Option<&[u8]>) -> Stats {
    let old = old.unwrap_or_default();
    let new = new.unwrap_or_default();

    if is_binary(old) || is_binary(new) {
        return Stats {
            added: 0,
            removed: 0,
            binary: true,
            too_large: false,
        };
    }
    if old.len() > MAX_BLOB_BYTES || new.len() > MAX_BLOB_BYTES {
        return Stats {
            added: 0,
            removed: 0,
            binary: false,
            too_large: true,
        };
    }

    let input = blob::InternedInput::new(
        blob::sources::byte_lines(old),
        blob::sources::byte_lines(new),
    );
    let diff = blob::Diff::compute(blob::Algorithm::Histogram, &input);

    Stats {
        added: diff.count_additions(),
        removed: diff.count_removals(),
        binary: false,
        too_large: false,
    }
}

/// Group the raw changed ranges into `@@` blocks with context around them.
///
/// Two changes close enough that their context would overlap belong in one
/// hunk; that is what keeps a diff from splitting into a block per line.
fn hunks(old: &[u8], new: &[u8]) -> Vec<Hunk> {
    let input = blob::InternedInput::new(
        blob::sources::byte_lines(old),
        blob::sources::byte_lines(new),
    );
    let diff = blob::diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

    let old_total = input.before.len() as u32;
    let new_total = input.after.len() as u32;

    let mut groups: Vec<Vec<blob::Hunk>> = Vec::new();
    for change in diff.hunks() {
        match groups.last_mut() {
            Some(group)
                if change.before.start
                    <= group.last().expect("group is never empty").before.end + CONTEXT * 2 =>
            {
                group.push(change);
            }
            _ => groups.push(vec![change]),
        }
    }

    // The terminator is not part of the line. A CR is dropped with it, so a
    // CRLF file does not render a control character at the end of every row.
    // The diff itself is computed over the raw bytes, so a commit that only
    // changes line endings still shows its lines as changed — they just look
    // the same, which is the truth about what changed.
    let text = |token: blob::Token| {
        let line: &[u8] = input.interner[token];
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        String::from_utf8_lossy(line).into_owned()
    };

    let mut out = Vec::with_capacity(groups.len());

    for group in groups {
        let first = group.first().expect("group is never empty");
        let last = group.last().expect("group is never empty");

        // Leading context exists in both versions, so the same number of lines
        // is taken off each side's start.
        let old_start = first.before.start.saturating_sub(CONTEXT);
        let lead = first.before.start - old_start;
        let new_start = first.after.start.saturating_sub(lead);

        let old_end = (last.before.end + CONTEXT).min(old_total);
        let trail = old_end - last.before.end;
        let new_end = (last.after.end + trail).min(new_total);

        let mut lines = Vec::new();
        let mut old_at = old_start;
        let mut new_at = new_start;

        for change in &group {
            while old_at < change.before.start {
                lines.push(DiffLine {
                    origin: LineOrigin::Context,
                    old: Some(old_at + 1),
                    new: Some(new_at + 1),
                    text: text(input.before[old_at as usize]),
                });
                old_at += 1;
                new_at += 1;
            }
            for index in change.before.clone() {
                lines.push(DiffLine {
                    origin: LineOrigin::Removed,
                    old: Some(index + 1),
                    new: None,
                    text: text(input.before[index as usize]),
                });
            }
            for index in change.after.clone() {
                lines.push(DiffLine {
                    origin: LineOrigin::Added,
                    old: None,
                    new: Some(index + 1),
                    text: text(input.after[index as usize]),
                });
            }
            old_at = change.before.end;
            new_at = change.after.end;
        }

        while old_at < old_end {
            lines.push(DiffLine {
                origin: LineOrigin::Context,
                old: Some(old_at + 1),
                new: Some(new_at + 1),
                text: text(input.before[old_at as usize]),
            });
            old_at += 1;
            new_at += 1;
        }

        let old_lines = old_end - old_start;
        let new_lines = new_end - new_start;
        out.push(Hunk {
            old_start: header_start(old_start, old_lines),
            old_lines,
            new_start: header_start(new_start, new_lines),
            new_lines,
            header: format!(
                "@@ -{},{} +{},{} @@",
                header_start(old_start, old_lines),
                old_lines,
                header_start(new_start, new_lines),
                new_lines
            ),
            lines,
        });
    }

    out
}

/// Hunk headers count from 1, except for an empty range, which git writes as
/// the line it would follow.
fn header_start(start: u32, lines: u32) -> u32 {
    if lines == 0 {
        start
    } else {
        start + 1
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Pure-Rust standard Base64 encoder (FEAT-065).
pub fn encode_base64(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(BASE64_ALPHABET[(triple >> 18) & 0x3F] as char);
        result.push(BASE64_ALPHABET[(triple >> 12) & 0x3F] as char);

        if chunk.len() > 1 {
            result.push(BASE64_ALPHABET[(triple >> 6) & 0x3F] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_ALPHABET[triple & 0x3F] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Detect MIME type and image status from path extension (FEAT-065).
pub fn detect_mime(path: &str) -> (String, bool) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "png" => ("image/png".into(), true),
        "jpg" | "jpeg" => ("image/jpeg".into(), true),
        "gif" => ("image/gif".into(), true),
        "webp" => ("image/webp".into(), true),
        "svg" => ("image/svg+xml".into(), true),
        "ico" => ("image/x-icon".into(), true),
        "avif" => ("image/avif".into(), true),
        "bmp" => ("image/bmp".into(), true),
        "pdf" => ("application/pdf".into(), false),
        "zip" => ("application/zip".into(), false),
        "wasm" => ("application/wasm".into(), false),
        "tar" | "gz" | "tgz" => ("application/gzip".into(), false),
        _ => ("application/octet-stream".into(), false),
    }
}

/// Detailed binary or image diff metadata (FEAT-065).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryDiff {
    pub path: String,
    pub is_image: bool,
    pub mime: String,
    pub old_size: Option<usize>,
    pub new_size: Option<usize>,
    pub old_base64: Option<String>,
    pub new_base64: Option<String>,
}

/// Maximum payload size for base64 data transfer (10 MB).
const MAX_IMAGE_BASE64_BYTES: usize = 10 * 1024 * 1024;

/// Read binary / image diff for a single file in a commit (FEAT-065).
pub fn binary_file_diff(repo: &gix::Repository, commit_id: &str, path: &str) -> Result<BinaryDiff> {
    let (mime, is_image) = detect_mime(path);
    let commit = repo
        .rev_parse_single(commit_id)
        .map_err(|_| Error::UnknownCommit(commit_id.to_string()))?
        .object()
        .map_err(|_| Error::UnknownCommit(commit_id.to_string()))?
        .peel_to_commit()
        .map_err(|_| Error::UnknownCommit(commit_id.to_string()))?;

    let tree = commit.tree().map_err(|e| Error::Diff(e.to_string()))?;
    let new_blob = blob_at(&tree, path)?;
    let new_bytes = blob_bytes(repo, new_blob)?;

    let old_bytes = match commit.parent_ids().next() {
        Some(parent_id) => {
            let parent = repo
                .find_object(parent_id)
                .map_err(|e| Error::Diff(e.to_string()))?
                .peel_to_commit()
                .map_err(|e| Error::Diff(e.to_string()))?;
            let parent_tree = parent.tree().map_err(|e| Error::Diff(e.to_string()))?;
            let old_blob = blob_at(&parent_tree, path)?;
            blob_bytes(repo, old_blob)?
        }
        None => None,
    };

    let old_size = old_bytes.as_ref().map(|b| b.len());
    let new_size = new_bytes.as_ref().map(|b| b.len());

    let old_base64 = if is_image {
        old_bytes
            .filter(|b| b.len() <= MAX_IMAGE_BASE64_BYTES)
            .map(|b| encode_base64(&b))
    } else {
        None
    };

    let new_base64 = if is_image {
        new_bytes
            .filter(|b| b.len() <= MAX_IMAGE_BASE64_BYTES)
            .map(|b| encode_base64(&b))
    } else {
        None
    };

    Ok(BinaryDiff {
        path: path.to_string(),
        is_image,
        mime,
        old_size,
        new_size,
        old_base64,
        new_base64,
    })
}

/// Read binary / image diff for working copy (FEAT-065).
pub fn binary_working_diff(repo: &gix::Repository, path: &str, side: Side) -> Result<BinaryDiff> {
    let (mime, is_image) = detect_mime(path);
    let (old_bytes, new_bytes) = match side {
        Side::Staged => (head_blob(repo, path)?, index_bytes(repo, path)?),
        Side::Unstaged => (index_bytes(repo, path)?, worktree_bytes(repo, path)?),
    };

    let old_size = old_bytes.as_ref().map(|b| b.len());
    let new_size = new_bytes.as_ref().map(|b| b.len());

    let old_base64 = if is_image {
        old_bytes
            .filter(|b| b.len() <= MAX_IMAGE_BASE64_BYTES)
            .map(|b| encode_base64(&b))
    } else {
        None
    };

    let new_base64 = if is_image {
        new_bytes
            .filter(|b| b.len() <= MAX_IMAGE_BASE64_BYTES)
            .map(|b| encode_base64(&b))
    } else {
        None
    };

    Ok(BinaryDiff {
        path: path.to_string(),
        is_image,
        mime,
        old_size,
        new_size,
        old_base64,
        new_base64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_sniffing_matches_gits_rule() {
        assert!(!is_binary(b"plain text\n"));
        assert!(is_binary(b"has a \0 byte"));
        // A NUL past the sniff window is not looked at, same as git.
        let mut late = vec![b'a'; BINARY_SNIFF_BYTES + 10];
        late[BINARY_SNIFF_BYTES + 5] = 0;
        assert!(!is_binary(&late));
    }

    #[test]
    fn empty_range_headers_use_gits_convention() {
        assert_eq!(header_start(0, 0), 0);
        assert_eq!(header_start(0, 3), 1);
        assert_eq!(header_start(11, 7), 12);
    }

    #[test]
    fn one_changed_line_gets_context_either_side() {
        let old = b"a\nb\nc\nd\ne\nf\ng\nh\n";
        let new = b"a\nb\nc\nD\ne\nf\ng\nh\n";

        let hunks = hunks(old, new);
        assert_eq!(hunks.len(), 1);

        let hunk = &hunks[0];
        assert_eq!(hunk.header, "@@ -1,7 +1,7 @@");
        assert_eq!(hunk.lines.len(), 8); // 3 context, -d, +D, 3 context

        assert_eq!(hunk.lines[3].origin, LineOrigin::Removed);
        assert_eq!(hunk.lines[3].text, "d");
        assert_eq!(hunk.lines[3].old, Some(4));
        assert_eq!(hunk.lines[3].new, None);

        assert_eq!(hunk.lines[4].origin, LineOrigin::Added);
        assert_eq!(hunk.lines[4].text, "D");
        assert_eq!(hunk.lines[4].old, None);
        assert_eq!(hunk.lines[4].new, Some(4));

        // Context after the change keeps counting on both sides.
        assert_eq!(hunk.lines[5].old, Some(5));
        assert_eq!(hunk.lines[5].new, Some(5));
    }

    #[test]
    fn distant_changes_split_into_separate_hunks() {
        let old: String = (0..40).map(|n| format!("line {n}\n")).collect();
        // Two changes far enough apart that their context cannot meet.
        let new = old
            .replace("line 1\n", "LINE 1\n")
            .replace("line 38\n", "LINE 38\n");

        let hunks = hunks(old.as_bytes(), new.as_bytes());
        assert_eq!(hunks.len(), 2);
        assert!(hunks[0].old_start < hunks[1].old_start);
    }

    #[test]
    fn an_addition_diffs_against_nothing() {
        let hunks = hunks(b"", b"one\ntwo\n");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].header, "@@ -0,0 +1,2 @@");
        assert!(hunks[0].lines.iter().all(|l| l.origin == LineOrigin::Added));
    }

    #[test]
    fn crlf_terminators_are_not_shown() {
        let hunks = hunks(b"a\r\n", b"b\r\n");
        assert_eq!(hunks[0].lines[0].text, "a");
        assert_eq!(hunks[0].lines[1].text, "b");
    }

    #[test]
    fn a_binary_side_reports_no_lines() {
        let stats = line_stats(Some(b"text\n"), Some(b"bin\0ary"));
        assert!(stats.binary);
        assert_eq!((stats.added, stats.removed), (0, 0));
    }

    #[test]
    fn base64_encoder_produces_standard_output() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn detect_mime_identifies_images_and_binaries() {
        assert_eq!(detect_mime("icon.png"), ("image/png".into(), true));
        assert_eq!(detect_mime("photo.jpg"), ("image/jpeg".into(), true));
        assert_eq!(detect_mime("vector.svg"), ("image/svg+xml".into(), true));
        assert_eq!(detect_mime("data.pdf"), ("application/pdf".into(), false));
        assert_eq!(detect_mime("lib.wasm"), ("application/wasm".into(), false));
    }

    #[test]
    fn binary_diff_extracts_base64_for_images() {
        let fixture = crate::fixture::Fixture::woven();
        fixture.write_bytes(
            "image.png",
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        );
        fixture.git(&["add", "-A"]);
        let id = fixture.commit("Add test image");
        let diff = binary_file_diff(&fixture.open(), &id, "image.png").expect("binary diff");
        assert!(diff.is_image);
        assert_eq!(diff.mime, "image/png");
        assert_eq!(diff.new_size, Some(8));
        assert!(diff.new_base64.is_some());
    }
}

#[cfg(test)]
mod repository_tests {
    use super::*;
    use crate::fixture::Fixture;

    fn find<'a>(files: &'a [FileChange], path: &str) -> &'a FileChange {
        files.iter().find(|f| f.path == path).unwrap_or_else(|| {
            panic!(
                "no {path} in {:?}",
                files.iter().map(|f| &f.path).collect::<Vec<_>>()
            )
        })
    }

    #[test]
    fn commit_detail_reads_both_signatures_and_the_message() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "a\n");
        fixture.git(&["add", "-A"]);
        fixture.git(&["commit", "-q", "-m", "A subject", "-m", "A body paragraph."]);
        let id = fixture.head();

        let detail = commit_detail(&fixture.open(), &id).expect("detail");

        assert_eq!(detail.id, id);
        assert_eq!(detail.short, id[..7]);
        assert_eq!(detail.summary, "A subject");
        assert_eq!(detail.body, "A body paragraph.");
        assert_eq!(detail.author_name, "Ada Lovelace");
        assert_eq!(detail.author_email, "ada@example.com");
        assert_eq!(detail.committer_name, "Ada Lovelace");
        assert!(detail.parents.is_empty(), "the first commit has no parent");
        assert_eq!(detail.files.len(), 1);
    }

    #[test]
    fn a_commit_with_no_body_reports_an_empty_body_rather_than_repeating_the_subject() {
        let fixture = Fixture::woven();
        let detail = commit_detail(&fixture.open(), &fixture.rev("HEAD^2~1")).expect("detail");

        assert_eq!(detail.summary, "Rewrite line 3");
        assert_eq!(detail.body, "");
    }

    #[test]
    fn an_unknown_commit_says_which_one() {
        let fixture = Fixture::woven();
        let missing = "0".repeat(40);

        let error = commit_detail(&fixture.open(), &missing).unwrap_err();

        assert!(matches!(error, Error::UnknownCommit(id) if id == missing));
    }

    #[test]
    fn commit_diff_totals_match_the_files_it_lists() {
        let fixture = Fixture::woven();
        let diff = commit_diff(&fixture.open(), &fixture.rev("HEAD^2~1")).expect("diff");

        assert_eq!(diff.files.len(), 1);
        let file = find(&diff.files, "core.txt");
        assert_eq!((file.added, file.removed), (1, 1));
        assert_eq!((diff.added, diff.removed), (1, 1));
    }

    #[test]
    fn an_added_file_is_reported_as_added() {
        let fixture = Fixture::woven();
        let diff = commit_diff(&fixture.open(), &fixture.rev("HEAD^2")).expect("diff");

        let file = find(&diff.files, "split.txt");
        assert_eq!(file.status, FileStatus::Added);
        assert_eq!(file.removed, 0);
        assert!(file.added > 0);
    }

    #[test]
    fn a_deleted_file_is_reported_as_deleted() {
        let fixture = Fixture::woven();
        fixture.remove("notes.md");
        fixture.git(&["add", "-A"]);
        let id = fixture.commit("Remove the notes");

        let diff = commit_diff(&fixture.open(), &id).expect("diff");

        let file = find(&diff.files, "notes.md");
        assert_eq!(file.status, FileStatus::Deleted);
        assert_eq!(file.added, 0);
        assert!(file.removed > 0);
    }

    #[test]
    fn the_first_commit_diffs_against_nothing() {
        let fixture = Fixture::woven();
        let root = fixture
            .git(&["rev-list", "--max-parents=0", "HEAD"])
            .trim()
            .to_string();

        let diff = commit_diff(&fixture.open(), &root).expect("diff");

        assert!(
            diff.files.len() >= 4,
            "every file in the initial import is an addition"
        );
        assert!(diff.files.iter().all(|f| f.status == FileStatus::Added));
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn a_merge_is_diffed_against_its_first_parent() {
        // Otherwise a merge would appear to change everything both sides did.
        let fixture = Fixture::woven();
        let diff = commit_diff(&fixture.open(), &fixture.head()).expect("diff");

        let paths: Vec<&str> = diff.files.iter().map(|f| f.path.as_str()).collect();
        assert!(
            paths.contains(&"split.txt"),
            "brought in by the merged branch: {paths:?}"
        );
    }

    #[test]
    fn a_binary_file_reports_no_line_counts() {
        let fixture = Fixture::woven();
        fixture.write_bytes("logo.bin", &[0x00, 0xff, 0x00, 0xfe]);
        let id = fixture.commit_all("Change the binary");

        let diff = commit_diff(&fixture.open(), &id).expect("diff");
        let file = find(&diff.files, "logo.bin");

        assert!(file.binary);
        assert_eq!(
            (file.added, file.removed),
            (0, 0),
            "a guess would read as a real count"
        );
    }

    #[test]
    fn file_diff_returns_the_hunks_of_one_file_only() {
        let fixture = Fixture::woven();
        let id = fixture.rev("HEAD^2~1");

        let file = file_diff(&fixture.open(), &id, "core.txt").expect("file diff");

        assert_eq!(file.path, "core.txt");
        assert_eq!(file.status, FileStatus::Modified);
        assert!(!file.binary);
        assert_eq!(file.hunks.len(), 1);

        let hunk = &file.hunks[0];
        assert!(hunk.header.starts_with("@@ "));
        assert!(hunk
            .lines
            .iter()
            .any(|l| l.origin == LineOrigin::Removed && l.text == "line 3"));
        assert!(hunk
            .lines
            .iter()
            .any(|l| l.origin == LineOrigin::Added && l.text == "LINE THREE"));
    }

    #[test]
    fn line_numbers_agree_with_git() {
        let fixture = Fixture::woven();
        let id = fixture.rev("HEAD^2~1");

        let file = file_diff(&fixture.open(), &id, "core.txt").expect("file diff");
        let hunk = &file.hunks[0];

        let expected = fixture.git(&["show", "--unified=3", "--format=", &id]);
        let header = expected
            .lines()
            .find(|l| l.starts_with("@@"))
            .expect("a hunk header from git");

        assert_eq!(header, hunk.header);
    }

    #[test]
    fn a_binary_file_has_no_hunks_and_says_so() {
        let fixture = Fixture::woven();
        fixture.write_bytes("logo.bin", &[0x00, 0xff, 0x00, 0xfe]);
        let id = fixture.commit_all("Change the binary");

        let file = file_diff(&fixture.open(), &id, "logo.bin").expect("file diff");

        assert!(file.binary);
        assert!(file.hunks.is_empty());
    }

    #[test]
    fn a_path_in_neither_side_of_the_commit_says_which_path() {
        let fixture = Fixture::woven();
        let id = fixture.rev("HEAD^2~1");

        let error = file_diff(&fixture.open(), &id, "never/existed.txt").unwrap_err();

        assert!(matches!(error, Error::UnknownPath(p) if p == "never/existed.txt"));
    }

    #[test]
    fn a_path_that_exists_but_was_not_touched_diffs_to_nothing() {
        // Not reachable from the Diff screen, which only offers the paths in
        // the commit's own file list. Recorded so the behaviour is known: the
        // file exists on both sides, so it reads as modified with no hunks —
        // the same shape as a mode-only change.
        let fixture = Fixture::woven();
        let id = fixture.rev("HEAD^2~1");

        let file = file_diff(&fixture.open(), &id, "notes.md").expect("file diff");

        assert_eq!((file.added, file.removed), (0, 0));
        assert!(file.hunks.is_empty());
    }

    #[test]
    fn a_file_diff_for_an_unknown_commit_says_which_commit() {
        let fixture = Fixture::woven();
        let missing = "0".repeat(40);

        let error = file_diff(&fixture.open(), &missing, "core.txt").unwrap_err();

        assert!(matches!(error, Error::UnknownCommit(id) if id == missing));
    }

    #[test]
    fn only_a_full_id_identifies_a_commit() {
        // Every caller has the full id: rows carry it, and the Diff screen is
        // opened with it. Abbreviations are a command-line convenience, and
        // resolving them would mean deciding what to do about an ambiguous
        // prefix — a question nothing here asks.
        let fixture = Fixture::woven();
        let full = fixture.head();
        let short = &full[..7];

        assert!(commit_detail(&fixture.open(), &full).is_ok());
        assert!(matches!(
            commit_detail(&fixture.open(), short).unwrap_err(),
            Error::UnknownCommit(id) if id == short
        ));
    }
}

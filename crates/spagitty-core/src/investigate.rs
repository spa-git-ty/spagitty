// SPDX-License-Identifier: GPL-3.0-or-later

//! The life of a file, a folder, a branch or a line (FEAT-132).
//!
//! Investigate answers one question about each: when was it born and by whom,
//! how did it grow, when was it renamed, moved or rewritten, and who owns it
//! today. A life is a list of [`Entry`], newest first, read a page at a time so
//! the screen paints from the first page while the rest is read.
//!
//! All four targets fill the same shape. What differs is where the entries
//! come from:
//!
//! - a **file** from `git log --follow`, with each commit's status and numstat;
//! - a **folder** from the same log kept to a pathspec;
//! - a **branch** from the commits since it left the default branch, its
//!   reflog, and where it landed;
//! - a **line** from `git log -L`.
//!
//! Following a file across renames and a line through history are git's own
//! work, done commit by commit with its rename detection, so they are asked of
//! `git` (see `shell.rs`). Everything else — sizes, hunks, who owns what — is
//! read from objects here.
//!
//! Nothing in this module writes, and nothing reads the working tree: like
//! Files, it is given a revision and answers from the object database.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::attribution;
use crate::blame;
use crate::diff::{self, Hunk, LineOrigin};
use crate::error::{Error, Result};
use crate::reflog::{self, ReflogQuery};
use crate::shell::{self, LifeLog, LIFE_FIELD, LIFE_RECORD, LIFE_TRAILER};

/// A commit that removed at least this share of the file's lines, as they were
/// before it, is a rewrite and not an edit.
pub const REWRITE_SHARE: f64 = 0.5;

/// How many entries of a life are read before the screen offers *Load more*.
pub const LIFE_MAX: usize = 5000;

/// How many reflog moves of a branch are looked through for rebases and resets.
const REFLOG_LOOK: usize = 500;

/// The names a default branch has, in the order they are tried when the
/// remote does not say.
const DEFAULT_BRANCHES: [&str; 3] = ["main", "master", "trunk"];

/// What a life is the life of.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    File { path: String },
    Folder { path: String },
    Branch { name: String },
    Line { path: String, line: u32 },
}

/// What happened, in one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// The commit that added the file, the first under the folder, the line's
    /// first writing, or a branch's fork point.
    Born,
    /// A new name in the same folder.
    Renamed,
    /// The same file in another folder.
    Moved,
    /// At least [`REWRITE_SHARE`] of the file's lines, as they were, removed.
    Rewrite,
    Edit,
    Deleted,
    /// A folder: files added under it.
    Added,
    /// A folder: files removed from it.
    Removed,
    /// A branch: an ordinary commit.
    Commit,
    /// A branch: another branch merged into it.
    Merge,
    /// A branch: where it landed.
    Merged,
    /// A branch, from its reflog.
    Rebase,
    Reset,
}

/// One thing in a life.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub commit: String,
    pub short: String,
    pub author_name: String,
    pub author_email: String,
    /// Seconds since the unix epoch.
    pub time: i64,
    pub summary: String,
    /// The agent a `Co-authored-by` trailer names, by gate 1's rule.
    pub agent: Option<String>,
    pub kind: Kind,
    pub added: u32,
    pub removed: u32,
    /// A file or a line: the path at this commit.
    pub path: Option<String>,
    /// The path before this commit, when it renamed or moved the file.
    pub old_path: Option<String>,
    /// A file: its lines after this commit. `None` for a binary file.
    pub size: Option<u32>,
    /// A file: the share of its lines, as they were, that this commit removed.
    pub share: Option<f64>,
    /// A folder: how many files this commit added, removed and renamed.
    pub files_added: u32,
    pub files_removed: u32,
    pub files_renamed: u32,
    /// A line: its text as this commit left it.
    pub text: Option<String>,
    /// Said beside the summary: `old → new` for a rename, or what a branch
    /// was merged into.
    pub note: Option<String>,
}

/// A page of a life.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub entries: Vec<Entry>,
    /// Nothing older is left to read.
    pub done: bool,
}

/// How a life is asked for.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub target: Target,
    /// The revision the life is read back from. Empty is `HEAD`. A branch
    /// target ignores it: a branch is read from its own tip.
    #[serde(default)]
    pub rev: String,
    /// A file: follow it across renames.
    #[serde(default = "yes")]
    pub follow: bool,
    /// Nothing older than this, in seconds since the epoch.
    #[serde(default)]
    pub since: Option<i64>,
    #[serde(default)]
    pub skip: usize,
    pub limit: usize,
}

fn yes() -> bool {
    true
}

/// A page of the life `ask` names.
pub fn life(repo: &gix::Repository, ask: &Ask) -> Result<Page> {
    let rev = if ask.rev.trim().is_empty() {
        "HEAD"
    } else {
        ask.rev.trim()
    };
    let limit = ask.limit.clamp(1, LIFE_MAX);
    match &ask.target {
        Target::File { path } => file_life(repo, rev, path, ask.follow, ask.since, ask.skip, limit),
        Target::Folder { path } => folder_life(repo, rev, path, ask.since, ask.skip, limit),
        Target::Branch { name } => branch_life(repo, name, ask.since, ask.skip, limit),
        Target::Line { path, line } => line_life(repo, rev, path, *line),
    }
}

fn dir(repo: &gix::Repository) -> &std::path::Path {
    repo.workdir().unwrap_or_else(|| repo.git_dir())
}

/// One raw status line: `M`, `A`, `D`, or `R` with the path it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    status: char,
    path: String,
    old_path: Option<String>,
}

/// One commit of a log page, before it is read as an entry of some target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Logged {
    commit: String,
    short: String,
    author_name: String,
    author_email: String,
    time: i64,
    parents: Vec<String>,
    summary: String,
    agent: Option<String>,
    changes: Vec<Change>,
    added: u32,
    removed: u32,
    /// Whatever followed the header that was neither status nor numstat: the
    /// patch, for `git log -L`.
    rest: Vec<String>,
}

/// Read `git log`'s records.
///
/// Lenient about what it does not know: a line that is neither a status line
/// nor a numstat line is kept as `rest` rather than refused, which is where a
/// line log's patch goes.
fn parse_log(out: &str) -> Vec<Logged> {
    out.split(LIFE_RECORD)
        .filter(|record| !record.trim().is_empty())
        .filter_map(|record| {
            let mut lines = record.lines();
            let header = lines.next()?;
            let fields: Vec<&str> = header.split(LIFE_FIELD).collect();
            if fields.len() < 7 {
                return None;
            }
            let agent = fields
                .get(7)
                .copied()
                .unwrap_or("")
                .split(LIFE_TRAILER)
                .map(str::trim)
                .find(|who| attribution::names_an_agent(who))
                .map(|who| {
                    let name = who.split('<').next().unwrap_or(who).trim();
                    if name.is_empty() { who } else { name }.to_string()
                });

            let mut logged = Logged {
                commit: fields[0].to_string(),
                short: fields[1].to_string(),
                author_name: fields[2].to_string(),
                author_email: fields[3].to_string(),
                time: fields[4].parse().unwrap_or(0),
                parents: fields[5].split_whitespace().map(str::to_string).collect(),
                summary: fields[6].to_string(),
                agent,
                changes: Vec::new(),
                added: 0,
                removed: 0,
                rest: Vec::new(),
            };

            for line in lines {
                if let Some(change) = raw_line(line) {
                    logged.changes.push(change);
                } else if let Some((added, removed)) = numstat_line(line) {
                    logged.added += added;
                    logged.removed += removed;
                } else {
                    logged.rest.push(line.to_string());
                }
            }
            Some(logged)
        })
        .collect()
}

/// `:100644 100644 <id> <id> R087\told\tnew` as a [`Change`].
fn raw_line(line: &str) -> Option<Change> {
    let line = line.strip_prefix(':')?;
    let mut parts = line.split('\t');
    let status = parts.next()?.split(' ').nth(4)?.chars().next()?;
    let first = parts.next()?.to_string();
    Some(match parts.next() {
        Some(second) => Change {
            status,
            path: second.to_string(),
            old_path: Some(first),
        },
        None => Change {
            status,
            path: first,
            old_path: None,
        },
    })
}

/// `12\t3\tpath` as its two counts. A binary file's `-\t-` counts as nothing.
fn numstat_line(line: &str) -> Option<(u32, u32)> {
    let mut parts = line.splitn(3, '\t');
    let added = parts.next()?;
    let removed = parts.next()?;
    parts.next()?;
    let count = |text: &str| match text {
        "-" => Some(0),
        digits if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
            digits.parse().ok()
        }
        _ => None,
    };
    Some((count(added)?, count(removed)?))
}

fn blank(logged: &Logged, kind: Kind) -> Entry {
    Entry {
        commit: logged.commit.clone(),
        short: logged.short.clone(),
        author_name: logged.author_name.clone(),
        author_email: logged.author_email.clone(),
        time: logged.time,
        summary: logged.summary.clone(),
        agent: logged.agent.clone(),
        kind,
        added: logged.added,
        removed: logged.removed,
        path: None,
        old_path: None,
        size: None,
        share: None,
        files_added: 0,
        files_removed: 0,
        files_renamed: 0,
        text: None,
        note: None,
    }
}

fn blob_at(repo: &gix::Repository, commit: &str, path: &str) -> Option<Vec<u8>> {
    let id = gix::ObjectId::from_hex(commit.as_bytes()).ok()?;
    let tree = repo.find_commit(id).ok()?.tree().ok()?;
    let entry = tree
        .lookup_entry_by_path(std::path::Path::new(path))
        .ok()??;
    if !entry.mode().is_blob_or_symlink() {
        return None;
    }
    Some(repo.find_object(entry.object_id()).ok()?.detach().data)
}

/// A blob's lines, or `None` for a binary one. A trailing newline ends the
/// last line rather than starting an empty one.
fn lines_of(data: &[u8]) -> Option<u32> {
    if data[..data.len().min(8000)].contains(&0) {
        return None;
    }
    if data.is_empty() {
        return Some(0);
    }
    let newlines = data.iter().filter(|byte| **byte == b'\n').count() as u32;
    Some(if data.ends_with(b"\n") {
        newlines
    } else {
        newlines + 1
    })
}

/// The folder a path is in. Empty for a path at the root.
fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// What a commit did to a file, from its status and how much it removed.
///
/// `before` is the file's lines before the commit. Structure comes first: a
/// commit that renamed a file and rewrote it is a rename, because that is the
/// event the next reader has to know to follow the file.
fn file_kind(change: &Change, removed: u32, before: Option<u32>) -> (Kind, Option<f64>) {
    let share = before
        .filter(|lines| *lines > 0)
        .map(|lines| f64::from(removed) / f64::from(lines));
    let kind = match change.status {
        'A' => Kind::Born,
        'D' => Kind::Deleted,
        'R' | 'C' => match &change.old_path {
            Some(old) if parent_of(old) != parent_of(&change.path) => Kind::Moved,
            _ => Kind::Renamed,
        },
        _ if share.is_some_and(|share| share >= REWRITE_SHARE) => Kind::Rewrite,
        _ => Kind::Edit,
    };
    (kind, share)
}

fn file_life(
    repo: &gix::Repository,
    rev: &str,
    path: &str,
    follow: bool,
    since: Option<i64>,
    skip: usize,
    limit: usize,
) -> Result<Page> {
    let out = shell::life_log(
        dir(repo),
        &LifeLog {
            revision: rev,
            path: Some(path),
            follow,
            skip,
            limit,
            since,
        },
    )?;
    let logged = parse_log(&out);
    let done = logged.len() < limit;

    let entries = logged
        .iter()
        .map(|commit| {
            // One file is followed, so one change is its own. A commit that
            // touched nothing by that name still lists as an edit of it.
            let change = commit.changes.first().cloned().unwrap_or(Change {
                status: 'M',
                path: path.to_string(),
                old_path: None,
            });
            let size = match change.status {
                'D' => Some(0),
                _ => blob_at(repo, &commit.commit, &change.path).and_then(|data| lines_of(&data)),
            };
            // Exact for a commit with one parent: what it left, less what it
            // added, plus what it took away.
            let before = size.map(|after| (after + commit.removed).saturating_sub(commit.added));
            let (kind, share) = file_kind(&change, commit.removed, before);
            let note = change
                .old_path
                .as_ref()
                .filter(|_| matches!(kind, Kind::Renamed | Kind::Moved))
                .map(|old| {
                    if kind == Kind::Renamed {
                        format!("{} → {}", name_of(old), name_of(&change.path))
                    } else {
                        format!("{} → {}", parent_of(old), parent_of(&change.path))
                    }
                });
            Entry {
                path: Some(change.path.clone()),
                old_path: change.old_path.clone(),
                size,
                share,
                note,
                ..blank(commit, kind)
            }
        })
        .collect();

    Ok(Page { entries, done })
}

fn folder_life(
    repo: &gix::Repository,
    rev: &str,
    path: &str,
    since: Option<i64>,
    skip: usize,
    limit: usize,
) -> Result<Page> {
    let folder = path.trim_matches('/');
    let out = shell::life_log(
        dir(repo),
        &LifeLog {
            revision: rev,
            // An empty folder is the whole repository.
            path: (!folder.is_empty()).then_some(folder),
            follow: false,
            skip,
            limit,
            since,
        },
    )?;
    let logged = parse_log(&out);
    let done = logged.len() < limit;

    let mut entries: Vec<Entry> = logged
        .iter()
        .map(|commit| {
            let count =
                |status: char| commit.changes.iter().filter(|c| c.status == status).count() as u32;
            let (added, removed, renamed) = (count('A'), count('D'), count('R'));
            // The loudest thing the commit did to the folder names it.
            let kind = if added > 0 && added >= removed {
                Kind::Added
            } else if removed > 0 {
                Kind::Removed
            } else if renamed > 0 {
                Kind::Renamed
            } else {
                Kind::Edit
            };
            Entry {
                files_added: added,
                files_removed: removed,
                files_renamed: renamed,
                ..blank(commit, kind)
            }
        })
        .collect();

    // The first commit that added a file under it is where it was born, and
    // it is the oldest entry there is once nothing older is left. A range
    // ends where the range does, which may be long after the beginning.
    let whole = since.is_none()
        || !goes_further_back(
            repo,
            rev,
            (!folder.is_empty()).then_some(folder),
            skip + entries.len(),
        );
    if done && whole && skip + entries.len() > 0 {
        if let Some(oldest) = entries.last_mut() {
            oldest.kind = Kind::Born;
        }
    }
    Ok(Page { entries, done })
}

/// The branch a repository's history is measured against.
///
/// What the remote says its head is, when it says; otherwise the first of the
/// usual names that exists; otherwise nothing, and a branch's life then goes
/// back to the root.
pub fn default_branch(repo: &gix::Repository) -> Option<String> {
    if let Ok(head) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Some(target) = head.target().try_name() {
            let name = target.shorten().to_string();
            if let Some(branch) = name.strip_prefix("origin/") {
                // The local branch when there is one, the remote's otherwise.
                if repo.find_reference(&format!("refs/heads/{branch}")).is_ok() {
                    return Some(branch.to_string());
                }
                return Some(name);
            }
        }
    }
    DEFAULT_BRANCHES
        .iter()
        .find(|name| repo.find_reference(&format!("refs/heads/{name}")).is_ok())
        .map(|name| name.to_string())
}

fn full_id(repo: &gix::Repository, rev: &str) -> Result<String> {
    Ok(repo
        .rev_parse_single(rev)
        .map_err(|_| Error::UnknownCommit(rev.to_string()))?
        .object()
        .map_err(|_| Error::UnknownCommit(rev.to_string()))?
        .peel_to_commit()
        .map_err(|_| Error::UnknownCommit(rev.to_string()))?
        .id
        .to_string())
}

/// A commit read from its object, as a [`Logged`] with nothing changed.
fn logged_commit(repo: &gix::Repository, id: &str) -> Option<Logged> {
    let object = gix::ObjectId::from_hex(id.as_bytes()).ok()?;
    let commit = repo.find_commit(object).ok()?;
    let author = commit.author().ok()?;
    let message = commit.message_raw_sloppy().to_string();
    Some(Logged {
        commit: id.to_string(),
        short: id.chars().take(7).collect(),
        author_name: author.name.to_string(),
        author_email: author.email.to_string(),
        time: author.time().map(|time| time.seconds).unwrap_or(0),
        parents: commit.parent_ids().map(|id| id.to_string()).collect(),
        summary: message.lines().next().unwrap_or("").to_string(),
        agent: attribution::agent_in(&message),
        changes: Vec::new(),
        added: 0,
        removed: 0,
        rest: Vec::new(),
    })
}

/// What a branch's life is measured against, and how it stands today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchFacts {
    /// The branch it is measured against. `None` when the repository has no
    /// default branch, or the branch is the default one.
    pub against: Option<String>,
    /// Where it left that branch: the commit its first own commit was made on.
    pub fork: Option<String>,
    /// The commit on the other branch that took it in, once it has landed.
    pub landed: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// True when the branch is a local one, and so has a reflog to read.
    pub local: bool,
}

/// A branch, placed against the default branch.
struct Shape {
    tip: String,
    /// What the branch's own commits are counted from: the default branch's
    /// tip, or, once the branch has landed, the commit it landed on.
    base: Option<String>,
    facts: BranchFacts,
}

/// Where `name` stands against the default branch.
///
/// **The fork is not the merge base.** A branch that has had the default
/// branch merged into it shares that branch's newest commit with it, and a
/// branch that has landed shares its own tip — so the merge base of either is
/// far later than where the work began. The fork is found from the branch's
/// own commits instead: the ones on its first-parent line that the other
/// branch lacks, the oldest of which was made on the commit it forked from.
fn shape(repo: &gix::Repository, name: &str) -> Result<Shape> {
    let tip = full_id(repo, name)?;
    let local = repo.find_reference(&format!("refs/heads/{name}")).is_ok();
    let alone = |tip: String| Shape {
        tip,
        base: None,
        facts: BranchFacts {
            against: None,
            fork: None,
            landed: None,
            ahead: 0,
            behind: 0,
            local,
        },
    };
    let Some(default) = default_branch(repo).filter(|default| default != name) else {
        return Ok(alone(tip));
    };
    let other = full_id(repo, &default)?;
    // Nothing in common: two histories in one repository.
    if shell::merge_base(dir(repo), &other, &tip)?.is_none() {
        return Ok(alone(tip));
    }

    let landed = shell::landing(dir(repo), &tip, &other)?;
    // Once landed, every commit of the branch is in the other branch too, and
    // what was its own is what the landing commit's first parent lacks.
    let base = match &landed {
        Some(landing) => first_parent(repo, landing).unwrap_or_else(|| other.clone()),
        None => other.clone(),
    };
    let own = shell::own_commits(dir(repo), &base, &tip)?;
    let fork = match own.last() {
        Some(oldest) => first_parent(repo, oldest),
        // No commit of its own: it is still where it was cut.
        None => Some(tip.clone()),
    };
    let (ahead, behind) = shell::ahead_behind(dir(repo), &other, &tip)?;

    Ok(Shape {
        tip,
        base: Some(base),
        facts: BranchFacts {
            against: Some(default),
            fork,
            landed,
            ahead,
            behind,
            local,
        },
    })
}

/// The branch `name` against the default branch.
pub fn branch_facts(repo: &gix::Repository, name: &str) -> Result<BranchFacts> {
    Ok(shape(repo, name)?.facts)
}

fn branch_life(
    repo: &gix::Repository,
    name: &str,
    since: Option<i64>,
    skip: usize,
    limit: usize,
) -> Result<Page> {
    let Shape { tip, base, facts } = shape(repo, name)?;
    // What the branch has that its base lacks, or the whole history for the
    // default branch and for a branch that shares nothing with it.
    let range = match &base {
        Some(base) if *base != tip && facts.fork.as_deref() != Some(tip.as_str()) => {
            format!("{base}..{tip}")
        }
        Some(_) => String::new(),
        None => tip.clone(),
    };

    let logged = if range.is_empty() {
        Vec::new()
    } else {
        parse_log(&shell::life_log(
            dir(repo),
            &LifeLog {
                revision: &range,
                path: None,
                follow: false,
                skip,
                limit,
                since,
            },
        )?)
    };
    let done = logged.len() < limit;

    let mut entries: Vec<Entry> = logged
        .iter()
        .map(|commit| {
            let kind = if commit.parents.len() > 1 {
                Kind::Merge
            } else {
                Kind::Commit
            };
            blank(commit, kind)
        })
        .collect();

    // What is not a commit on the branch comes with the first page: where it
    // landed, and what its reflog says was done to it.
    if skip == 0 {
        if let (Some(default), Some(landed)) = (&facts.against, &facts.landed) {
            if let Some(commit) = logged_commit(repo, landed) {
                entries.push(Entry {
                    note: Some(format!("into {default}")),
                    ..blank(&commit, Kind::Merged)
                });
            }
        }
        if facts.local {
            entries.extend(reflog_events(repo, name, since));
        }
    }

    if done {
        // Born where it left the default branch. A branch with no fork — the
        // default one, or an unrelated history — was born at its root commit,
        // which is already the oldest entry there is.
        match &facts.fork {
            Some(fork) => {
                // Where it forked is its birth only when that is in the
                // range asked for: an entry older than the range is not
                // part of it.
                if let Some(commit) =
                    logged_commit(repo, fork).filter(|commit| since.is_none_or_older(commit.time))
                {
                    entries.push(Entry {
                        note: facts
                            .against
                            .as_ref()
                            .map(|default| format!("from {default}")),
                        ..blank(&commit, Kind::Born)
                    });
                }
            }
            None => {
                // The oldest commit read is the root only when the range
                // goes back to it.
                let whole = since.is_none()
                    || range.is_empty()
                    || !goes_further_back(repo, &range, None, skip + logged.len());
                if whole {
                    if let Some(root) = entries
                        .iter_mut()
                        .rev()
                        .find(|entry| matches!(entry.kind, Kind::Commit | Kind::Merge))
                    {
                        root.kind = Kind::Born;
                    }
                }
            }
        }
    }

    // Newest first, the way every other life is. Stable, so git's own order
    // stands among commits of one second.
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.time));
    Ok(Page { entries, done })
}

/// The rebases and resets a local branch's reflog records.
fn reflog_events(repo: &gix::Repository, name: &str, since: Option<i64>) -> Vec<Entry> {
    let Ok(log) = reflog::reflog(
        repo,
        &ReflogQuery {
            reference: format!("refs/heads/{name}"),
            limit: REFLOG_LOOK,
        },
    ) else {
        return Vec::new();
    };

    log.entries
        .iter()
        .filter(|entry| since.is_none_or_older(entry.time))
        .filter_map(|entry| {
            let kind = reflog_kind(&entry.message)?;
            Some(Entry {
                commit: entry.after.clone(),
                short: entry.after_short.clone(),
                author_name: entry.author_name.clone(),
                author_email: String::new(),
                time: entry.time,
                summary: entry.message.clone(),
                agent: None,
                kind,
                added: 0,
                removed: 0,
                path: None,
                old_path: None,
                size: None,
                share: None,
                files_added: 0,
                files_removed: 0,
                files_renamed: 0,
                text: None,
                note: Some(format!("{} → {}", entry.before_short, entry.after_short)),
            })
        })
        .collect()
}

/// Whether history goes on past the `read` commits a read with a range gave.
///
/// One more commit is asked for with the range taken off. When there is one,
/// what was read stops where the range does and not at the beginning, so
/// nothing in it is a birth. When git cannot be asked, no birth is claimed.
fn goes_further_back(
    repo: &gix::Repository,
    revision: &str,
    path: Option<&str>,
    read: usize,
) -> bool {
    shell::life_log(
        dir(repo),
        &LifeLog {
            revision,
            path,
            follow: false,
            skip: read,
            limit: 1,
            since: None,
        },
    )
    .map(|out| !parse_log(&out).is_empty())
    .unwrap_or(true)
}

/// `since`, read as "is this time inside the range".
trait Since {
    fn is_none_or_older(&self, time: i64) -> bool;
}

impl Since for Option<i64> {
    fn is_none_or_older(&self, time: i64) -> bool {
        match self {
            Some(since) => time >= *since,
            None => true,
        }
    }
}

/// What a reflog message says was done to the branch, when it was a rebase or
/// a reset. A rebase writes one line as it finishes; the lines for each
/// commit it replayed are on `HEAD`'s log, not the branch's.
fn reflog_kind(message: &str) -> Option<Kind> {
    let message = message.trim_start();
    if message.starts_with("rebase") {
        Some(Kind::Rebase)
    } else if message.starts_with("reset:") {
        Some(Kind::Reset)
    } else {
        None
    }
}

fn line_life(repo: &gix::Repository, rev: &str, path: &str, line: u32) -> Result<Page> {
    let out = shell::line_log(dir(repo), rev, path, line.max(1), LIFE_MAX)?;
    let mut entries: Vec<Entry> = parse_log(&out)
        .iter()
        .map(|commit| {
            let (text, added, removed, at) = line_patch(&commit.rest, path);
            Entry {
                added,
                removed,
                path: Some(at),
                text: Some(text),
                ..blank(commit, Kind::Edit)
            }
        })
        .collect();
    // `-L` follows the line back to the commit that first wrote it.
    if let Some(oldest) = entries.last_mut() {
        oldest.kind = Kind::Born;
    }
    Ok(Page {
        entries,
        done: true,
    })
}

/// A line log's patch: the line as the commit left it, how many lines it
/// added and removed there, and the path it was under.
fn line_patch(patch: &[String], asked: &str) -> (String, u32, u32, String) {
    let mut text: Vec<&str> = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut path = asked.to_string();
    let mut in_hunk = false;

    for line in patch {
        if let Some(to) = line.strip_prefix("+++ b/") {
            path = to.to_string();
            continue;
        }
        if line.starts_with("@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            continue;
        }
        if let Some(kept) = line.strip_prefix('+') {
            added += 1;
            text.push(kept);
        } else if line.starts_with('-') {
            removed += 1;
        } else if let Some(kept) = line.strip_prefix(' ') {
            text.push(kept);
        }
    }
    (text.join("\n"), added, removed, path)
}

/// Who a file belongs to today, and what each commit still has in it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    /// The file's lines at the revision.
    pub lines: u32,
    /// By author, most lines first. An agent's commits are the agent's.
    pub owners: Vec<Owner>,
    /// For each commit that still has lines in the file: how many.
    pub alive: HashMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub name: String,
    pub email: String,
    pub agent: bool,
    pub lines: u32,
    /// `lines` over the file's lines, from 0 to 1.
    pub share: f64,
}

/// `path` at `rev`, read from one blame: who owns it, and what survives.
///
/// A file that cannot be blamed — binary, too large, not there — has no
/// owners and nothing alive, which is the truth and not a failure.
pub fn today(repo: &gix::Repository, path: &str, rev: &str) -> Result<Today> {
    let blamed = blame::compact(repo, path, rev)?;
    let mut alive: HashMap<String, u32> = HashMap::new();
    let mut owned: HashMap<(String, bool), (String, u32)> = HashMap::new();

    for &index in &blamed.lines {
        let Some(commit) = blamed.commits.get(index as usize) else {
            continue;
        };
        *alive.entry(commit.id.clone()).or_insert(0) += 1;
        let key = match &commit.agent {
            Some(agent) => (agent.clone(), true),
            None => (commit.author_name.clone(), false),
        };
        let held = owned
            .entry(key)
            .or_insert_with(|| (commit.author_email.clone(), 0));
        held.1 += 1;
    }

    let lines = blamed.lines.len() as u32;
    let mut owners: Vec<Owner> = owned
        .into_iter()
        .map(|((name, agent), (email, count))| Owner {
            name,
            email,
            agent,
            lines: count,
            share: if lines == 0 {
                0.0
            } else {
                f64::from(count) / f64::from(lines)
            },
        })
        .collect();
    owners.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.name.cmp(&b.name)));

    Ok(Today {
        lines,
        owners,
        alive,
    })
}

fn changed_lines(hunk: &Hunk) -> usize {
    hunk.lines
        .iter()
        .filter(|line| line.origin != LineOrigin::Context)
        .count()
}

fn first_parent(repo: &gix::Repository, commit: &str) -> Option<String> {
    let id = gix::ObjectId::from_hex(commit.as_bytes()).ok()?;
    repo.find_commit(id)
        .ok()?
        .parent_ids()
        .next()
        .map(|id| id.to_string())
}

/// The largest hunk of what `commit` did to `path`: the one with the most
/// lines added and removed. `old_path` is where the file was before, when the
/// commit renamed it. `None` when the commit changed no line of it — a pure
/// rename, a binary file.
pub fn largest_hunk(
    repo: &gix::Repository,
    commit: &str,
    path: &str,
    old_path: Option<&str>,
) -> Result<Option<Hunk>> {
    let commit = full_id(repo, commit)?;
    let new = blob_at(repo, &commit, path).unwrap_or_default();
    let old = first_parent(repo, &commit)
        .and_then(|parent| blob_at(repo, &parent, old_path.unwrap_or(path)))
        .unwrap_or_default();
    if lines_of(&old).is_none() || lines_of(&new).is_none() {
        return Ok(None);
    }
    Ok(diff::hunks(&old, &new)
        .into_iter()
        .max_by_key(changed_lines)
        .filter(|hunk| changed_lines(hunk) > 0))
}

/// Every hunk between `path` as `commit` left it and `today_path` at `rev`:
/// *Compare with today*.
pub fn compare(
    repo: &gix::Repository,
    commit: &str,
    path: &str,
    rev: &str,
    today_path: &str,
) -> Result<Vec<Hunk>> {
    let then = full_id(repo, commit)?;
    let now = full_id(repo, if rev.trim().is_empty() { "HEAD" } else { rev })?;
    let old = blob_at(repo, &then, path).unwrap_or_default();
    let new = blob_at(repo, &now, today_path).unwrap_or_default();
    if lines_of(&old).is_none() || lines_of(&new).is_none() {
        return Ok(Vec::new());
    }
    Ok(diff::hunks(&old, &new))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    const T0: u64 = 1_700_000_000;
    const DAY: u64 = 86_400;

    /// Stage everything and commit at a fixed time.
    fn save(fixture: &Fixture, message: &str, day: u64) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit_at(message, T0 + day * DAY)
    }

    fn numbered(count: usize) -> String {
        (1..=count).map(|n| format!("line {n}\n")).collect()
    }

    fn ask(target: Target) -> Ask {
        Ask {
            target,
            rev: String::new(),
            follow: true,
            since: None,
            skip: 0,
            limit: 100,
        }
    }

    fn file(path: &str) -> Ask {
        ask(Target::File {
            path: path.to_string(),
        })
    }

    fn kinds(page: &Page) -> Vec<Kind> {
        page.entries.iter().map(|entry| entry.kind).collect()
    }

    /// Born, edited, renamed in place, moved to another folder, rewritten.
    fn a_life() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("src/avatar.rs", &numbered(20));
        save(&fixture, "Add avatar cache", 0);
        fixture.write("src/avatar.rs", &(numbered(20) + "one more\n"));
        save(&fixture, "Key avatars by size", 10);
        fixture.git(&["mv", "src/avatar.rs", "src/avatars.rs"]);
        save(&fixture, "Call it what it holds", 20);
        std::fs::create_dir_all(fixture.at("core")).expect("the folder to move into");
        fixture.git(&["mv", "src/avatars.rs", "core/avatars.rs"]);
        save(&fixture, "Move avatars into core", 30);
        // Twelve of twenty-one lines replaced: more than half.
        let rewritten: String = (1..=21)
            .map(|n| {
                if n <= 12 {
                    format!("rewritten {n}\n")
                } else if n <= 20 {
                    format!("line {n}\n")
                } else {
                    "one more\n".to_string()
                }
            })
            .collect();
        fixture.write("core/avatars.rs", &rewritten);
        save(&fixture, "Expire avatars daily", 40);
        fixture
    }

    #[test]
    fn a_files_life_goes_back_past_its_renames_to_the_commit_that_added_it() {
        let fixture = a_life();
        let repo = fixture.open();

        let page = life(&repo, &file("core/avatars.rs")).expect("life");

        assert!(page.done);
        assert_eq!(
            kinds(&page),
            [
                Kind::Rewrite,
                Kind::Moved,
                Kind::Renamed,
                Kind::Edit,
                Kind::Born
            ]
        );
        let born = page.entries.last().expect("born");
        assert_eq!(born.summary, "Add avatar cache");
        assert_eq!(born.path.as_deref(), Some("src/avatar.rs"));
        assert_eq!(born.author_name, "Ada Lovelace");
        assert_eq!(born.time, T0 as i64);
    }

    #[test]
    fn a_new_name_in_the_same_folder_is_renamed_and_another_folder_is_moved() {
        let fixture = a_life();
        let repo = fixture.open();
        let page = life(&repo, &file("core/avatars.rs")).expect("life");

        let renamed = &page.entries[2];
        assert_eq!(renamed.kind, Kind::Renamed);
        assert_eq!(renamed.old_path.as_deref(), Some("src/avatar.rs"));
        assert_eq!(renamed.path.as_deref(), Some("src/avatars.rs"));
        assert_eq!(renamed.note.as_deref(), Some("avatar.rs → avatars.rs"));

        let moved = &page.entries[1];
        assert_eq!(moved.kind, Kind::Moved);
        assert_eq!(moved.old_path.as_deref(), Some("src/avatars.rs"));
        assert_eq!(moved.note.as_deref(), Some("src → core"));
    }

    #[test]
    fn without_following_renames_the_life_begins_where_the_name_does() {
        let fixture = a_life();
        let repo = fixture.open();

        let page = life(
            &repo,
            &Ask {
                follow: false,
                ..file("core/avatars.rs")
            },
        )
        .expect("life");

        // The move is where this name first appears, so it reads as born.
        assert_eq!(kinds(&page), [Kind::Rewrite, Kind::Born]);
        assert_eq!(page.entries[1].summary, "Move avatars into core");
    }

    #[test]
    fn the_size_after_each_commit_is_counted_from_the_file_itself() {
        let fixture = a_life();
        let repo = fixture.open();
        let page = life(&repo, &file("core/avatars.rs")).expect("life");

        let sizes: Vec<Option<u32>> = page.entries.iter().map(|entry| entry.size).collect();
        assert_eq!(sizes, [Some(21), Some(21), Some(21), Some(21), Some(20)]);
        let edit = &page.entries[3];
        assert_eq!((edit.added, edit.removed), (1, 0));
    }

    /// The share is of the lines as they were then: `REWRITE_SHARE` and over
    /// is a rewrite, and one line fewer is an edit.
    #[test]
    fn half_the_files_lines_is_a_rewrite_and_one_line_fewer_is_an_edit() {
        assert_eq!(REWRITE_SHARE, 0.5);
        let fixture = Fixture::empty();
        fixture.write("a.txt", &numbered(20));
        save(&fixture, "Add", 0);

        // Nine of twenty.
        let nine: String = (1..=20)
            .map(|n| {
                if n <= 9 {
                    format!("changed {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect();
        fixture.write("a.txt", &nine);
        save(&fixture, "Nine lines", 1);

        // Ten of twenty.
        let ten: String = (1..=20)
            .map(|n| {
                if n > 10 {
                    format!("other {n}\n")
                } else if n <= 9 {
                    format!("changed {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect();
        fixture.write("a.txt", &ten);
        save(&fixture, "Ten lines", 2);
        let repo = fixture.open();

        let page = life(&repo, &file("a.txt")).expect("life");

        assert_eq!(kinds(&page), [Kind::Rewrite, Kind::Edit, Kind::Born]);
        assert_eq!(page.entries[0].share, Some(0.5));
        assert_eq!(page.entries[1].share, Some(0.45));
        // The commit that added it removed nothing of nothing.
        assert_eq!(page.entries[2].share, None);
    }

    #[test]
    fn a_deleted_file_ends_with_nothing_and_says_so() {
        let fixture = Fixture::empty();
        fixture.write("gone.txt", &numbered(5));
        fixture.write("stays.txt", "x\n");
        save(&fixture, "Add", 0);
        fixture.git(&["rm", "-q", "gone.txt"]);
        save(&fixture, "Remove it", 1);
        let repo = fixture.open();

        let page = life(&repo, &file("gone.txt")).expect("life");

        assert_eq!(kinds(&page), [Kind::Deleted, Kind::Born]);
        assert_eq!(page.entries[0].size, Some(0));
        assert_eq!(page.entries[0].removed, 5);
    }

    #[test]
    fn a_life_is_read_a_page_at_a_time() {
        let fixture = Fixture::empty();
        for n in 0..5 {
            fixture.write("a.txt", &numbered(n + 1));
            save(&fixture, &format!("Change {n}"), n as u64);
        }
        let repo = fixture.open();

        let first = life(
            &repo,
            &Ask {
                limit: 2,
                ..file("a.txt")
            },
        )
        .expect("life");
        assert_eq!(first.entries.len(), 2);
        assert!(!first.done);
        assert_eq!(first.entries[0].summary, "Change 4");

        let last = life(
            &repo,
            &Ask {
                limit: 2,
                skip: 4,
                ..file("a.txt")
            },
        )
        .expect("life");
        assert_eq!(last.entries.len(), 1);
        assert!(last.done);
        assert_eq!(last.entries[0].kind, Kind::Born);
    }

    #[test]
    fn a_range_leaves_out_what_is_older() {
        let fixture = a_life();
        let repo = fixture.open();

        let page = life(
            &repo,
            &Ask {
                since: Some((T0 + 25 * DAY) as i64),
                ..file("core/avatars.rs")
            },
        )
        .expect("life");

        assert_eq!(kinds(&page), [Kind::Rewrite, Kind::Moved]);
    }

    #[test]
    fn a_commit_an_agent_co_authored_is_the_agents() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        save(&fixture, "Add", 0);
        fixture.write("a.txt", "one\ntwo\n");
        save(
            &fixture,
            "Cache on disk\n\nCo-authored-by: Codex <noreply@example.com>",
            1,
        );
        fixture.write("a.txt", "one\ntwo\nthree\n");
        save(
            &fixture,
            "Tidy\n\nCo-authored-by: Claude Shannon <claude@bell-labs.example>",
            2,
        );
        let repo = fixture.open();

        let page = life(&repo, &file("a.txt")).expect("life");
        let agents: Vec<Option<&str>> = page
            .entries
            .iter()
            .map(|entry| entry.agent.as_deref())
            .collect();

        assert_eq!(agents, [None, Some("Codex"), None]);
    }

    #[test]
    fn a_folders_life_counts_the_files_added_removed_and_renamed_under_it() {
        let fixture = Fixture::empty();
        fixture.write("docs/a.md", &numbered(3));
        fixture.write("docs/b.md", &numbered(3));
        fixture.write("src/outside.rs", "x\n");
        save(&fixture, "Start the docs", 0);
        fixture.write("docs/a.md", &numbered(8));
        fixture.write("src/outside.rs", "y\n");
        save(&fixture, "Grow a", 1);
        fixture.git(&["mv", "docs/b.md", "docs/c.md"]);
        save(&fixture, "Rename b", 2);
        fixture.git(&["rm", "-q", "docs/a.md"]);
        save(&fixture, "Remove a", 3);
        fixture.write("src/outside.rs", "z\n");
        save(&fixture, "Not the docs", 4);
        let repo = fixture.open();

        let page = life(
            &repo,
            &ask(Target::Folder {
                path: "docs/".to_string(),
            }),
        )
        .expect("life");

        assert!(page.done);
        // The commit that only touched src is not in the folder's life.
        assert_eq!(
            kinds(&page),
            [Kind::Removed, Kind::Renamed, Kind::Edit, Kind::Born]
        );
        assert_eq!(page.entries[3].files_added, 2);
        assert_eq!((page.entries[3].added, page.entries[3].removed), (6, 0));
        assert_eq!(page.entries[2].added, 5);
        assert_eq!(page.entries[1].files_renamed, 1);
        assert_eq!(page.entries[0].files_removed, 1);
        assert_eq!(page.entries[0].removed, 8);
    }

    #[test]
    fn a_folder_is_not_born_until_its_oldest_commit_has_been_read() {
        let fixture = Fixture::empty();
        for n in 0..4 {
            fixture.write(&format!("docs/{n}.md"), "x\n");
            save(&fixture, &format!("Add {n}"), n);
        }
        let repo = fixture.open();
        let folder = || {
            ask(Target::Folder {
                path: "docs".to_string(),
            })
        };

        let first = life(
            &repo,
            &Ask {
                limit: 2,
                ..folder()
            },
        )
        .expect("life");
        assert!(!first.done);
        assert!(first.entries.iter().all(|entry| entry.kind == Kind::Added));

        let rest = life(
            &repo,
            &Ask {
                limit: 2,
                skip: 2,
                ..folder()
            },
        )
        .expect("life");
        assert!(!rest.done, "a full page may not be the last");
        let end = life(
            &repo,
            &Ask {
                limit: 2,
                skip: 4,
                ..folder()
            },
        )
        .expect("life");
        assert!(end.done);
        assert!(end.entries.is_empty());
    }

    /// A range that begins after a thing was born has no birth in it: the
    /// oldest commit in the range is only the oldest in the range.
    #[test]
    fn a_range_that_begins_after_the_birth_has_no_birth_in_it() {
        let after = |day: u64| Some((T0 + day * DAY - DAY / 2) as i64);

        // A folder with a commit a day, from day 0 to day 3.
        let fixture = Fixture::empty();
        for n in 0..4 {
            fixture.write(&format!("docs/{n}.md"), "x\n");
            save(&fixture, &format!("Add {n}"), n);
        }
        let repo = fixture.open();
        let folder = |since| Ask {
            since,
            ..ask(Target::Folder {
                path: "docs".to_string(),
            })
        };
        let late = life(&repo, &folder(after(2))).expect("life");
        assert!(late.done);
        assert_eq!(kinds(&late), [Kind::Added, Kind::Added]);
        // A range that reaches the first commit still finds the birth.
        let all = life(&repo, &folder(after(0))).expect("life");
        assert_eq!(
            kinds(&all),
            [Kind::Added, Kind::Added, Kind::Added, Kind::Born]
        );

        // A branch that left main on day 1, with commits on days 2 and 3.
        let fixture = forked();
        let repo = fixture.open();
        let feature = |since| Ask {
            since,
            ..branch("feature")
        };
        let late = life(&repo, &feature(after(3))).expect("life");
        assert_eq!(late.entries.len(), 1);
        assert_eq!(late.entries[0].summary, "C2");
        assert_eq!(late.entries[0].kind, Kind::Commit);
        // The fork is in a range that goes back to it.
        let all = life(&repo, &feature(after(1))).expect("life");
        assert_eq!(kinds(&all), [Kind::Commit, Kind::Commit, Kind::Born]);

        // The default branch, born at its root on day 0.
        let main = |since| Ask {
            since,
            ..branch("main")
        };
        let late = life(&repo, &main(after(4))).expect("life");
        assert_eq!(late.entries.len(), 1);
        assert_eq!(late.entries[0].summary, "D");
        assert_eq!(late.entries[0].kind, Kind::Commit);
        let all = life(&repo, &main(after(0))).expect("life");
        assert_eq!(all.entries.last().map(|entry| entry.kind), Some(Kind::Born));
    }

    /// main: A — B — D
    ///            \
    /// feature:    C1 — C2
    fn forked() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "a\n");
        save(&fixture, "A", 0);
        fixture.write("a.txt", "a\nb\n");
        save(&fixture, "B", 1);
        fixture.git(&["switch", "-q", "-c", "feature"]);
        fixture.write("f.txt", "one\n");
        save(&fixture, "C1", 2);
        fixture.write("f.txt", "one\ntwo\n");
        save(&fixture, "C2", 3);
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("d.txt", "d\n");
        save(&fixture, "D", 4);
        fixture
    }

    fn branch(name: &str) -> Ask {
        ask(Target::Branch {
            name: name.to_string(),
        })
    }

    #[test]
    fn a_branch_is_born_where_it_left_the_default_branch() {
        let fixture = forked();
        let repo = fixture.open();
        let fork = fixture.rev("main~1");

        let facts = branch_facts(&repo, "feature").expect("facts");
        assert_eq!(facts.against.as_deref(), Some("main"));
        assert_eq!(facts.fork.as_deref(), Some(fork.as_str()));
        assert_eq!((facts.ahead, facts.behind), (2, 1));
        assert!(facts.local);

        let page = life(&repo, &branch("feature")).expect("life");
        assert!(page.done);
        assert_eq!(kinds(&page), [Kind::Commit, Kind::Commit, Kind::Born]);
        let born = page.entries.last().expect("born");
        assert_eq!(born.commit, fork);
        assert_eq!(born.summary, "B");
        assert_eq!(born.note.as_deref(), Some("from main"));
        assert_eq!(page.entries[0].summary, "C2");
        assert_eq!((page.entries[0].added, page.entries[0].removed), (1, 0));
    }

    /// The same birth with no reflog to read: a branch that is only a remote's.
    #[test]
    fn a_branch_with_no_reflog_is_born_at_the_same_place() {
        let fixture = forked();
        let tip = fixture.rev("feature");
        let fork = fixture.rev("main~1");
        fixture.git(&["update-ref", "refs/remotes/origin/feature", &tip]);
        fixture.git(&["branch", "-D", "-q", "feature"]);
        let repo = fixture.open();

        let facts = branch_facts(&repo, "origin/feature").expect("facts");
        assert!(!facts.local);
        assert_eq!(facts.fork.as_deref(), Some(fork.as_str()));

        let page = life(&repo, &branch("origin/feature")).expect("life");
        assert_eq!(kinds(&page), [Kind::Commit, Kind::Commit, Kind::Born]);
        assert_eq!(page.entries.last().expect("born").commit, fork);
    }

    #[test]
    fn a_local_branchs_reflog_gives_its_resets_and_rebases() {
        let fixture = forked();
        fixture.git(&["switch", "-q", "feature"]);
        fixture.git(&["reset", "-q", "--hard", "HEAD~1"]);
        fixture.git(&["rebase", "-q", "main"]);
        let repo = fixture.open();

        let page = life(&repo, &branch("feature")).expect("life");
        let seen = kinds(&page);

        assert!(seen.contains(&Kind::Reset), "{seen:?}");
        assert!(seen.contains(&Kind::Rebase), "{seen:?}");
        // Rebased onto main's tip, so that is where it now leaves main.
        let born = page
            .entries
            .iter()
            .find(|entry| entry.kind == Kind::Born)
            .expect("born");
        assert_eq!(born.commit, fixture.rev("main"));
        let reset = page
            .entries
            .iter()
            .find(|entry| entry.kind == Kind::Reset)
            .expect("reset");
        assert!(reset.note.as_deref().is_some_and(|note| note.contains('→')));
    }

    #[test]
    fn a_merged_branch_says_where_it_landed_and_a_merge_into_it_is_one() {
        let fixture = forked();
        // main merged into feature, then feature merged into main.
        fixture.git(&["switch", "-q", "feature"]);
        fixture.git(&[
            "merge",
            "-q",
            "--no-ff",
            "main",
            "-m",
            "Merge main into feature",
        ]);
        fixture.git(&["switch", "-q", "main"]);
        fixture.git(&["merge", "-q", "--no-ff", "feature", "-m", "Merge feature"]);
        let landing = fixture.rev("main");
        let repo = fixture.open();

        let page = life(&repo, &branch("feature")).expect("life");

        let merged = page
            .entries
            .iter()
            .find(|entry| entry.kind == Kind::Merged)
            .expect("where it landed");
        assert_eq!(merged.commit, landing);
        assert_eq!(merged.note.as_deref(), Some("into main"));
        assert!(page
            .entries
            .iter()
            .any(|entry| entry.kind == Kind::Merge && entry.summary == "Merge main into feature"));
    }

    /// The merge base moves when the default branch is merged in, and again
    /// when the branch lands. Where the work began does not.
    #[test]
    fn a_branch_is_born_where_its_work_began_whatever_was_merged_since() {
        let fixture = forked();
        let began = fixture.rev("main~1");
        fixture.git(&["switch", "-q", "feature"]);
        fixture.git(&[
            "merge",
            "-q",
            "--no-ff",
            "main",
            "-m",
            "Merge main into feature",
        ]);
        let repo = fixture.open();

        // The merge base is now main's tip; the fork is still B.
        let facts = branch_facts(&repo, "feature").expect("facts");
        assert_eq!(
            shell::merge_base(fixture.path(), "main", "feature").expect("base"),
            Some(fixture.rev("main"))
        );
        assert_eq!(facts.fork.as_deref(), Some(began.as_str()));
        assert_eq!(facts.landed, None);

        // And landed: every commit of it is in main, and it was still born at B.
        fixture.git(&["switch", "-q", "main"]);
        fixture.git(&["merge", "-q", "--no-ff", "feature", "-m", "Merge feature"]);
        let facts = branch_facts(&repo, "feature").expect("facts");
        assert_eq!(facts.fork.as_deref(), Some(began.as_str()));
        assert_eq!(facts.landed, Some(fixture.rev("main")));
        assert_eq!(facts.ahead, 0);

        let page = life(&repo, &branch("feature")).expect("life");
        let born = page.entries.last().expect("born");
        assert_eq!(born.kind, Kind::Born);
        assert_eq!(born.commit, began);
        // Its own two commits and the merge of main into it; not main's D.
        let own: Vec<&str> = page
            .entries
            .iter()
            .filter(|entry| matches!(entry.kind, Kind::Commit | Kind::Merge))
            .map(|entry| entry.summary.as_str())
            .collect();
        assert_eq!(own, ["Merge main into feature", "C2", "C1"]);
    }

    #[test]
    fn a_branch_with_no_commit_of_its_own_is_still_where_it_was_cut() {
        let fixture = forked();
        fixture.git(&["branch", "fresh", "main"]);
        let repo = fixture.open();

        let facts = branch_facts(&repo, "fresh").expect("facts");
        assert_eq!(facts.fork, Some(fixture.rev("main")));
        assert_eq!((facts.ahead, facts.behind), (0, 0));

        let page = life(&repo, &branch("fresh")).expect("life");
        assert_eq!(kinds(&page), [Kind::Born]);
    }

    #[test]
    fn the_default_branch_is_born_at_its_root_and_measured_against_nothing() {
        let fixture = forked();
        let repo = fixture.open();

        assert_eq!(default_branch(&repo).as_deref(), Some("main"));
        let facts = branch_facts(&repo, "main").expect("facts");
        assert_eq!(facts.against, None);
        assert_eq!(facts.fork, None);

        let page = life(&repo, &branch("main")).expect("life");
        assert_eq!(page.entries.last().expect("root").kind, Kind::Born);
        assert_eq!(page.entries.last().expect("root").summary, "A");
        assert_eq!(page.entries.len(), 3);
    }

    #[test]
    fn the_remote_says_which_branch_is_the_default_when_it_says() {
        let fixture = forked();
        fixture.git(&["branch", "-m", "main", "develop"]);
        let tip = fixture.rev("develop");
        fixture.git(&["update-ref", "refs/remotes/origin/develop", &tip]);
        fixture.git(&[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/develop",
        ]);
        let repo = fixture.open();

        assert_eq!(default_branch(&repo).as_deref(), Some("develop"));
    }

    #[test]
    fn a_lines_life_is_every_commit_that_changed_it_with_its_text_then() {
        let fixture = Fixture::empty();
        fixture.write("a.rs", "fn one() {}\nfn two() {}\nfn three() {}\n");
        save(&fixture, "Add", 0);
        fixture.write("a.rs", "fn one() {}\nfn two() { 2 }\nfn three() {}\n");
        save(&fixture, "Give two a body", 1);
        fixture.write("a.rs", "fn one() {}\nfn three() {}\n");
        fixture.write("b.rs", "unrelated\n");
        save(&fixture, "Not line two's business", 2);
        fixture.write("a.rs", "// head\nfn one() {}\nfn three() { 3 }\n");
        save(&fixture, "Give three a body", 3);
        let repo = fixture.open();

        // Line 3 today is `fn three`, which was line 3, then line 2.
        let page = life(
            &repo,
            &ask(Target::Line {
                path: "a.rs".to_string(),
                line: 3,
            }),
        )
        .expect("life");

        assert!(page.done);
        assert_eq!(kinds(&page), [Kind::Edit, Kind::Born]);
        assert_eq!(page.entries[0].summary, "Give three a body");
        assert_eq!(page.entries[0].text.as_deref(), Some("fn three() { 3 }"));
        assert_eq!((page.entries[0].added, page.entries[0].removed), (1, 1));
        assert_eq!(page.entries[1].summary, "Add");
        assert_eq!(page.entries[1].text.as_deref(), Some("fn three() {}"));
        assert_eq!(page.entries[1].path.as_deref(), Some("a.rs"));
    }

    #[test]
    fn todays_blame_says_who_owns_the_file_and_what_survives_of_each_commit() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", &numbered(10));
        let first = save(&fixture, "Add", 0);
        // Four of the ten replaced by an agent's commit.
        let edited: String = (1..=10)
            .map(|n| {
                if n <= 4 {
                    format!("agent {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect();
        fixture.write("a.txt", &edited);
        let second = save(
            &fixture,
            "Rework the head\n\nCo-authored-by: Codex <noreply@example.com>",
            1,
        );
        // And one of the agent's lines replaced again.
        fixture.write("a.txt", &edited.replace("agent 1\n", "by hand\n"));
        let third = save(&fixture, "Fix the first line", 2);
        let repo = fixture.open();

        let now = today(&repo, "a.txt", "HEAD").expect("today");

        assert_eq!(now.lines, 10);
        assert_eq!(now.alive.get(&first), Some(&6));
        // The agent wrote four and three survive.
        assert_eq!(now.alive.get(&second), Some(&3));
        assert_eq!(now.alive.get(&third), Some(&1));

        assert_eq!(now.owners.len(), 2);
        assert_eq!(now.owners[0].name, "Ada Lovelace");
        assert!(!now.owners[0].agent);
        assert_eq!(now.owners[0].lines, 7);
        assert_eq!(now.owners[0].share, 0.7);
        assert_eq!(now.owners[1].name, "Codex");
        assert!(now.owners[1].agent);
        assert_eq!(now.owners[1].lines, 3);
    }

    #[test]
    fn a_file_that_cannot_be_blamed_has_no_owners_and_is_not_an_error() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        let now = today(&repo, "logo.bin", "HEAD").expect("today");

        assert_eq!(now.lines, 0);
        assert!(now.owners.is_empty());
        assert!(now.alive.is_empty());
    }

    #[test]
    fn the_largest_hunk_is_the_one_with_the_most_lines_changed() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", &numbered(60));
        save(&fixture, "Add", 0);
        let edited: String = (1..=60)
            .map(|n| match n {
                2 => "small change\n".to_string(),
                40..=44 => format!("big change {n}\n"),
                _ => format!("line {n}\n"),
            })
            .collect();
        fixture.write("a.txt", &edited);
        let commit = save(&fixture, "Two changes", 1);
        let repo = fixture.open();

        let hunk = largest_hunk(&repo, &commit, "a.txt", None)
            .expect("hunk")
            .expect("a hunk");

        assert_eq!(changed_lines(&hunk), 10);
        assert!(hunk
            .lines
            .iter()
            .any(|line| line.origin == LineOrigin::Added && line.text == "big change 42"));
        assert!(!hunk.lines.iter().any(|line| line.text == "small change"));
    }

    #[test]
    fn a_rename_that_changed_no_line_has_no_hunk_and_one_that_did_has() {
        let fixture = a_life();
        let repo = fixture.open();
        let page = life(&repo, &file("core/avatars.rs")).expect("life");
        let renamed = &page.entries[2];
        let rewrite = &page.entries[0];

        assert!(largest_hunk(
            &repo,
            &renamed.commit,
            "src/avatars.rs",
            Some("src/avatar.rs")
        )
        .expect("hunk")
        .is_none());
        assert!(
            largest_hunk(&repo, &rewrite.commit, "core/avatars.rs", None)
                .expect("hunk")
                .is_some()
        );
        // The commit that added the file: all of it is the hunk.
        let born = page.entries.last().expect("born");
        let added = largest_hunk(&repo, &born.commit, "src/avatar.rs", None)
            .expect("hunk")
            .expect("a hunk");
        assert_eq!(changed_lines(&added), 20);
    }

    #[test]
    fn compare_with_today_is_the_file_then_against_the_file_now_under_its_new_name() {
        let fixture = a_life();
        let repo = fixture.open();
        let page = life(&repo, &file("core/avatars.rs")).expect("life");
        let born = page.entries.last().expect("born");

        let hunks = compare(
            &repo,
            &born.commit,
            "src/avatar.rs",
            "HEAD",
            "core/avatars.rs",
        )
        .expect("compare");
        let added: usize = hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| line.origin == LineOrigin::Added)
            .count();

        // Twelve rewritten and one appended since it was born.
        assert_eq!(added, 13);
        // The newest entry against today is nothing at all.
        assert!(compare(
            &repo,
            &page.entries[0].commit,
            "core/avatars.rs",
            "",
            "core/avatars.rs"
        )
        .expect("compare")
        .is_empty());
    }

    #[test]
    fn a_log_page_is_read_field_by_field() {
        let id = "a".repeat(40);
        let parent = "b".repeat(40);
        let out = format!(
            "{r}{id}{f}aaaaaaa{f}Ada Lovelace{f}ada@example.com{f}1700000000{f}{parent}{f}Rename it{f}Codex <c@example.com>{t}Ada <a@example.com>\n\n:100644 100644 {id} {parent} R090\told dir/a b.rs\tnew dir/a b.rs\n3\t1\told dir/a b.rs => new dir/a b.rs\n-\t-\tlogo.png\n",
            r = LIFE_RECORD,
            f = LIFE_FIELD,
            t = LIFE_TRAILER
        );

        let logged = parse_log(&out);

        assert_eq!(logged.len(), 1);
        let commit = &logged[0];
        assert_eq!(commit.summary, "Rename it");
        assert_eq!(commit.parents, [parent]);
        assert_eq!(commit.agent.as_deref(), Some("Codex"));
        assert_eq!(
            commit.changes,
            [Change {
                status: 'R',
                path: "new dir/a b.rs".to_string(),
                old_path: Some("old dir/a b.rs".to_string())
            }]
        );
        // A binary file's dashes count as nothing.
        assert_eq!((commit.added, commit.removed), (3, 1));
        assert!(parse_log("").is_empty());
    }

    #[test]
    fn a_revision_or_a_branch_that_does_not_exist_is_an_error() {
        let fixture = forked();
        let repo = fixture.open();

        assert!(life(&repo, &branch("nowhere")).is_err());
        assert!(life(
            &repo,
            &Ask {
                rev: "nowhere".to_string(),
                ..file("a.txt")
            }
        )
        .is_err());
        assert!(branch_facts(&repo, "nowhere").is_err());
    }

    /// Like Files: given a revision, it answers from objects.
    #[test]
    fn a_life_does_not_read_the_working_copy() {
        let fixture = a_life();
        let repo = fixture.open();
        fixture.write("core/avatars.rs", "changed on disk and not committed\n");

        let page = life(&repo, &file("core/avatars.rs")).expect("life");

        assert_eq!(page.entries.len(), 5);
        assert_eq!(page.entries[0].size, Some(21));
        assert_eq!(
            today(&repo, "core/avatars.rs", "HEAD")
                .expect("today")
                .lines,
            21
        );
    }
}

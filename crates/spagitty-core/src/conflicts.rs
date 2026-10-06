// SPDX-License-Identifier: GPL-3.0-or-later

//! A repository stopped mid-operation.
//!
//! When git cannot merge two versions of a file it does not pick one. It writes
//! all three into the index — stage 1 the common ancestor, stage 2 ours, stage 3
//! theirs — and leaves the working-tree file with conflict markers in it. That
//! is the whole data model of this module: read those stages and say what each
//! side held.
//!
//! Since FEAT-016 it writes as well, and the writing half is built on the same
//! three stages. There are exactly three ways a file leaves this screen — a
//! whole side taken, one marker region resolved, or text supplied by the person
//! reading it — and all three end at the same place: the file on disk, followed
//! by an explicit `git add`. Nothing marks a file resolved on the user's behalf.
//! That is the one check that resolution actually worked, and it belongs to the
//! person who looked at the result.
//!
//! # Marker parsing is a text format, not a guess
//!
//! [`regions`] reads the conflict markers git wrote. It is deliberately strict:
//! a region needs its opening, its separator and its close, in that order, at
//! the start of a line. Anything else — a `<<<<<<<` inside a string literal, a
//! half-edited file — is left alone rather than reinterpreted, because the
//! failure mode of guessing here is silently keeping the wrong half of
//! somebody's work.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// How far into a blob we look for a NUL byte before calling it binary. Git's
/// own rule, and the same constant `diff.rs` uses, so both modules call the same
/// files binary that `git diff` does.
const BINARY_SNIFF_BYTES: usize = 8000;

/// Above this, a side is reported as too large rather than sent to the screen.
/// Three sides render at once here, so the ceiling matters more than it does on
/// the Diff screen.
const MAX_SIDE_BYTES: usize = 8 * 1024 * 1024;

/// What the repository is in the middle of.
///
/// Read from the repository's own state rather than inferred from the presence
/// of conflicts. Merge, rebase, cherry-pick and revert all leave conflicts
/// behind, and telling someone "merge" during a rebase sends them to the wrong
/// command to get out of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    Merge,
    Rebase,
    RebaseInteractive,
    CherryPick,
    Revert,
    ApplyMailbox,
    Bisect,
    /// Nothing in progress. Conflicts can still exist — an unresolved index
    /// outlives the command that made it — and saying "none" is the honest
    /// answer rather than picking the likeliest operation.
    None,
}

impl Operation {
    /// What git calls this, for a screen that has to name the command the user
    /// will type to get out.
    pub fn label(self) -> &'static str {
        match self {
            Operation::Merge => "merge",
            Operation::Rebase => "rebase",
            Operation::RebaseInteractive => "interactive rebase",
            Operation::CherryPick => "cherry-pick",
            Operation::Revert => "revert",
            Operation::ApplyMailbox => "am",
            Operation::Bisect => "bisect",
            Operation::None => "none",
        }
    }
}

/// Which side of a conflict to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// Stage 2 — what the branch you are on had.
    Ours,
    /// Stage 3 — what is coming in.
    Theirs,
}

/// One `<<<<<<< ======= >>>>>>>` block in a file on disk.
///
/// Line numbers are 1-based and inclusive, matching what the merged pane shows,
/// so a region can be pointed at on screen without a second numbering scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    /// Position in the file, counting from 0. What a caller names it by.
    pub index: usize,
    /// The `<<<<<<<` line.
    pub start_line: usize,
    /// The `>>>>>>>` line.
    pub end_line: usize,
    /// Our lines, without the markers.
    pub ours: String,
    /// The base's lines, when the file was merged with `diff3` markers.
    pub base: Option<String>,
    /// Their lines, without the markers.
    pub theirs: String,
}

/// Which sides a conflicted path has, which is the same thing as what kind of
/// conflict it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    /// Stages 1, 2 and 3: both sides changed a file that already existed.
    BothModified,
    /// Stages 2 and 3 with no base: both sides added the same path.
    BothAdded,
    /// No stage 2: our side deleted it, theirs changed it.
    DeletedByUs,
    /// No stage 3: their side deleted it, ours changed it.
    DeletedByThem,
}

impl ConflictKind {
    fn from_stages(base: bool, ours: bool, theirs: bool) -> Option<Self> {
        match (base, ours, theirs) {
            (_, true, true) if base => Some(ConflictKind::BothModified),
            (false, true, true) => Some(ConflictKind::BothAdded),
            (_, false, true) => Some(ConflictKind::DeletedByUs),
            (_, true, false) => Some(ConflictKind::DeletedByThem),
            _ => None,
        }
    }
}

/// One conflicted path, as the pager lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFile {
    pub path: String,
    pub kind: ConflictKind,
}

/// One version of a conflicted file.
///
/// `None` where a side does not exist is the caller's business; this is what a
/// side that *does* exist holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSide {
    /// Empty when `binary` or `too_large`, because there is nothing honest to
    /// put there.
    pub text: String,
    pub lines: usize,
    pub bytes: usize,
    pub binary: bool,
    pub too_large: bool,
}

/// The three index stages of one conflicted path, plus what is on disk.
///
/// A missing side is [`None`] rather than an empty [`ConflictSide`]: "the file
/// was deleted on that side" and "the file was emptied on that side" are
/// different things, and the second one loses work if acted on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSides {
    pub path: String,
    pub kind: ConflictKind,
    /// Stage 1 — the common ancestor. Absent when both sides added the path.
    pub base: Option<ConflictSide>,
    /// Stage 2 — ours, which is `HEAD`.
    pub ours: Option<ConflictSide>,
    /// Stage 3 — theirs, the incoming side.
    pub theirs: Option<ConflictSide>,
    /// The working-tree file, markers and all. Absent when there is no file on
    /// disk, which is what a delete/modify conflict looks like until it is
    /// resolved.
    pub merged: Option<ConflictSide>,
}

/// Everything the screen needs before a file is chosen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictState {
    pub operation: Operation,
    pub files: Vec<ConflictFile>,
}

/// What the repository is in the middle of, from its own state.
pub fn operation(repo: &gix::Repository) -> Operation {
    use gix::state::InProgress;

    match repo.state() {
        Some(InProgress::Merge) => Operation::Merge,
        Some(InProgress::Rebase) => Operation::Rebase,
        Some(InProgress::RebaseInteractive) => Operation::RebaseInteractive,
        Some(InProgress::CherryPick | InProgress::CherryPickSequence) => Operation::CherryPick,
        Some(InProgress::Revert | InProgress::RevertSequence) => Operation::Revert,
        Some(InProgress::ApplyMailbox | InProgress::ApplyMailboxRebase) => Operation::ApplyMailbox,
        Some(InProgress::Bisect) => Operation::Bisect,
        None => Operation::None,
    }
}

/// Every conflicted path, sorted, with what kind of conflict each one is.
///
/// This is the index's own answer: entries at stages 1 to 3. A bare repository
/// has no index to read and reports nothing rather than failing, the same way
/// the rail's counts do.
pub fn conflicted(repo: &gix::Repository) -> Result<Vec<ConflictFile>> {
    let Some(index) = read_index(repo)? else {
        return Ok(Vec::new());
    };

    let mut files: Vec<ConflictFile> = Vec::new();
    for (path, stages) in group_by_path(&index) {
        if let Some(kind) = ConflictKind::from_stages(
            stages.base.is_some(),
            stages.ours.is_some(),
            stages.theirs.is_some(),
        ) {
            files.push(ConflictFile { path, kind });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The state of the screen before a file is chosen: what is running, and what
/// is conflicted.
pub fn state(repo: &gix::Repository) -> Result<ConflictState> {
    Ok(ConflictState {
        operation: operation(repo),
        files: conflicted(repo)?,
    })
}

/// The three sides of one conflicted path, plus the file on disk.
///
/// A path that is not conflicted is an error rather than an empty result: the
/// screen only ever asks about paths it was given, so being asked about
/// anything else means the screen and the repository have drifted apart.
pub fn sides(repo: &gix::Repository, path: &str) -> Result<ConflictSides> {
    let index = read_index(repo)?.ok_or_else(|| Error::UnknownPath(path.to_string()))?;

    let stages = group_by_path(&index)
        .into_iter()
        .find(|(entry_path, _)| entry_path == path)
        .map(|(_, stages)| stages)
        .ok_or_else(|| Error::UnknownPath(path.to_string()))?;

    let kind = ConflictKind::from_stages(
        stages.base.is_some(),
        stages.ours.is_some(),
        stages.theirs.is_some(),
    )
    .ok_or_else(|| Error::UnknownPath(path.to_string()))?;

    Ok(ConflictSides {
        path: path.to_string(),
        kind,
        base: blob_side(repo, stages.base)?,
        ours: blob_side(repo, stages.ours)?,
        theirs: blob_side(repo, stages.theirs)?,
        merged: worktree_side(repo, path),
    })
}

// --- Reading the index ----------------------------------------------------

/// The three blob ids a conflicted path can have.
#[derive(Default)]
struct Stages {
    base: Option<gix::ObjectId>,
    ours: Option<gix::ObjectId>,
    theirs: Option<gix::ObjectId>,
}

/// The index, or `None` for a repository that has none — a bare one, or one
/// with no commits and nothing staged.
fn read_index(repo: &gix::Repository) -> Result<Option<gix::worktree::Index>> {
    match repo.index() {
        Ok(index) => Ok(Some(index)),
        // Not "the index is broken": a repository with nothing in it has no
        // index file at all, and that is not a conflicted repository.
        Err(_) => Ok(None),
    }
}

/// Conflicted entries grouped by path, in index order.
///
/// The index is sorted by path with stages ascending within a path, so entries
/// for one path are adjacent; this still groups by path rather than relying on
/// that, because the grouping is what the caller means and the adjacency is an
/// implementation detail of the format.
fn group_by_path(index: &gix::worktree::Index) -> Vec<(String, Stages)> {
    use gix::index::entry::Stage;

    let mut out: Vec<(String, Stages)> = Vec::new();

    for entry in index.entries() {
        let stage = entry.stage();
        if stage == Stage::Unconflicted {
            continue;
        }

        let path = entry.path(index).to_string();
        let slot = match out.iter_mut().find(|(existing, _)| *existing == path) {
            Some(slot) => slot,
            None => {
                out.push((path, Stages::default()));
                out.last_mut().expect("just pushed")
            }
        };

        match stage {
            Stage::Base => slot.1.base = Some(entry.id),
            Stage::Ours => slot.1.ours = Some(entry.id),
            Stage::Theirs => slot.1.theirs = Some(entry.id),
            Stage::Unconflicted => unreachable!("skipped above"),
        }
    }

    out
}

// --- Reading a side -------------------------------------------------------

fn blob_side(repo: &gix::Repository, id: Option<gix::ObjectId>) -> Result<Option<ConflictSide>> {
    let Some(id) = id else { return Ok(None) };
    let object = repo
        .find_object(id)
        .map_err(|e| Error::Diff(e.to_string()))?;

    Ok(Some(side_from(&object.detach().data)))
}

/// The working-tree file. Absent rather than an error when there is nothing on
/// disk: a delete/modify conflict leaves no file until it is resolved, and an
/// unreadable one is a side we cannot show rather than a failure of the screen.
fn worktree_side(repo: &gix::Repository, path: &str) -> Option<ConflictSide> {
    let workdir = repo.workdir()?;
    let bytes = std::fs::read(workdir.join(path)).ok()?;
    Some(side_from(&bytes))
}

pub(crate) fn side_from(data: &[u8]) -> ConflictSide {
    let bytes = data.len();

    if is_binary(data) {
        return ConflictSide {
            text: String::new(),
            lines: 0,
            bytes,
            binary: true,
            too_large: false,
        };
    }
    if bytes > MAX_SIDE_BYTES {
        return ConflictSide {
            text: String::new(),
            lines: 0,
            bytes,
            binary: false,
            too_large: true,
        };
    }

    let text = String::from_utf8_lossy(data).into_owned();
    ConflictSide {
        lines: line_count(&text),
        text,
        bytes,
        binary: false,
        too_large: false,
    }
}

/// Lines the way a file viewer counts them: a trailing newline ends the last
/// line rather than starting an empty one.
fn line_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.lines().count()
}

/// Git's rule: a NUL byte near the start means binary.
fn is_binary(data: &[u8]) -> bool {
    data[..data.len().min(BINARY_SNIFF_BYTES)].contains(&0)
}

// --- Resolving (FEAT-016) ---------------------------------------------------

const OPEN: &str = "<<<<<<<";
const BASE: &str = "|||||||";
const SPLIT: &str = "=======";
const CLOSE: &str = ">>>>>>>";

/// Every conflict region in a merged file, in the order they appear.
///
/// Strict by design: a region must open, then optionally give a base, then
/// split, then close, each at the start of its own line. A file that does not
/// parse that way returns the regions it *could* read and stops — a marker
/// inside a string literal is far likelier than a file with a genuinely broken
/// conflict, and reinterpreting it would silently keep the wrong half.
pub fn regions(text: &str) -> Vec<Region> {
    let lines: Vec<&str> = text.lines().collect();
    let mut regions = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if !lines[i].starts_with(OPEN) {
            i += 1;
            continue;
        }

        let start = i;
        let mut ours: Vec<&str> = Vec::new();
        let mut base: Option<Vec<&str>> = None;
        let mut theirs: Vec<&str> = Vec::new();
        let mut phase = Phase::Ours;
        i += 1;

        let end = loop {
            let Some(line) = lines.get(i) else {
                // Ran off the end with the region still open. Not a region.
                break None;
            };

            if line.starts_with(BASE) && phase == Phase::Ours {
                phase = Phase::Base;
                base = Some(Vec::new());
            } else if line.starts_with(SPLIT) && phase != Phase::Theirs {
                phase = Phase::Theirs;
            } else if line.starts_with(CLOSE) {
                break if phase == Phase::Theirs {
                    Some(i)
                } else {
                    None
                };
            } else {
                match phase {
                    Phase::Ours => ours.push(line),
                    Phase::Base => base.as_mut().expect("base started").push(line),
                    Phase::Theirs => theirs.push(line),
                }
            }
            i += 1;
        };

        match end {
            Some(end) => {
                regions.push(Region {
                    index: regions.len(),
                    start_line: start + 1,
                    end_line: end + 1,
                    ours: joined(&ours),
                    base: base.as_deref().map(joined),
                    theirs: joined(&theirs),
                });
                i = end + 1;
            }
            // Unterminated: give up on this file rather than guessing where it
            // was meant to end.
            None => break,
        }
    }

    regions
}

#[derive(PartialEq, Eq)]
enum Phase {
    Ours,
    Base,
    Theirs,
}

/// Lines back into text, each one ended rather than separated.
///
/// An empty side is an empty string, not a newline: "this side contributed
/// nothing" and "this side contributed a blank line" are different results and
/// the second one is wrong.
fn joined(lines: &[&str]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Replace one region with the side chosen, markers and all.
///
/// The index is the region's position, not a line number, so a caller holding a
/// list from [`regions`] cannot be off by the length of an earlier resolution.
pub fn resolve_region(text: &str, index: usize, side: Side) -> Result<String> {
    let regions = regions(text);
    let region = regions.get(index).ok_or_else(|| {
        Error::NotStageable(format!("there is no conflict {} in this file", index + 1))
    })?;

    Ok(replace(text, region, keep(region, side)))
}

/// Replace every region with the side chosen.
///
/// Applied back to front, so each replacement's line numbers are still the ones
/// [`regions`] reported.
pub fn resolve_all(text: &str, side: Side) -> String {
    let mut result = text.to_string();
    for region in regions(text).iter().rev() {
        result = replace(&result, region, keep(region, side));
    }
    result
}

fn keep(region: &Region, side: Side) -> &str {
    match side {
        Side::Ours => &region.ours,
        Side::Theirs => &region.theirs,
    }
}

/// Swap `region`'s lines — markers included — for `kept`.
fn replace(text: &str, region: &Region, kept: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();

    for line in &lines[..region.start_line - 1] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(kept);
    for line in &lines[region.end_line..] {
        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Write the merged file, exactly as given.
///
/// A plain filesystem write rather than a `git` call, and it is the one write in
/// this crate that is: there is no git command for "put these bytes in the
/// working tree", and the working tree is not the shared state the `shell.rs`
/// boundary is about. The index is untouched, so the file stays conflicted until
/// somebody marks it resolved.
pub fn write_merged(repo: &gix::Repository, path: &str, text: &str) -> Result<()> {
    let full = crate::repo::workdir(repo)?.join(path);
    std::fs::write(full, text)?;
    Ok(())
}

/// Take one whole side of a conflicted file into the working tree.
///
/// `git checkout --ours` / `--theirs`, which reads the stage out of the index
/// rather than us reconstructing it — the two could disagree, and git's answer
/// is the one the rest of the ecosystem will see.
pub fn take(repo: &gix::Repository, path: &str, side: Side) -> Result<()> {
    crate::shell::checkout_side(crate::repo::workdir(repo)?, path, side)
}

/// Mark paths resolved: `git add`.
///
/// The index's three stages collapse to one when this succeeds, which is the
/// only real check that a resolution worked. It is never done on the user's
/// behalf as part of taking a side or saving an edit — looking at the result is
/// the point of the screen, and staging for them skips it.
pub fn mark_resolved(repo: &gix::Repository, paths: &[String]) -> Result<()> {
    crate::shell::stage(crate::repo::workdir(repo)?, paths)
}

/// Settle one conflicted file with what was chosen for it, and mark it
/// resolved (FEAT-102): its text, or one side whole — which, for the side that
/// deleted the file, deletes it.
///
/// One call for the resolver's *Mark resolved*, which the person presses
/// having seen the result: the write and the `git add` belong together there,
/// and the index is not touched until then.
pub fn settle(
    repo: &gix::Repository,
    path: &str,
    text: Option<&str>,
    whole: Option<Side>,
) -> Result<()> {
    match (text, whole) {
        (Some(text), None) => {
            if !regions(text).is_empty() {
                return Err(Error::NotStageable(format!(
                    "{path} still has conflict markers"
                )));
            }
            write_merged(repo, path, text)?;
            mark_resolved(repo, &[path.to_string()])
        }
        (None, Some(side)) => {
            let stages = sides(repo, path)?;
            let exists = match side {
                Side::Ours => stages.ours.is_some(),
                Side::Theirs => stages.theirs.is_some(),
            };
            if exists {
                take(repo, path, side)?;
                mark_resolved(repo, &[path.to_string()])
            } else {
                crate::shell::remove_path(crate::repo::workdir(repo)?, path)
            }
        }
        _ => Err(Error::NotStageable(format!(
            "{path} needs either text or a side"
        ))),
    }
}

/// Carry on with whatever the repository is in the middle of.
///
/// The operation decides the command. Telling git to continue a merge during a
/// rebase does not work, and guessing from the presence of conflicts is how
/// people end up running the wrong one.
pub fn continue_operation(repo: &gix::Repository, operation: Operation) -> Result<()> {
    let command = resumable(operation)?;
    crate::shell::sequencer(crate::repo::workdir(repo)?, command, "--continue")
}

/// Abandon it and put the repository back.
pub fn abort_operation(repo: &gix::Repository, operation: Operation) -> Result<()> {
    let command = resumable(operation)?;
    crate::shell::sequencer(crate::repo::workdir(repo)?, command, "--abort")
}

/// The `git` subcommand that continues or aborts this operation.
///
/// Bisect and `am` are refused rather than guessed at: neither is something this
/// screen starts, and `git bisect --abort` in particular means something quite
/// different from the others.
fn resumable(operation: Operation) -> Result<&'static str> {
    match operation {
        Operation::Merge => Ok("merge"),
        Operation::Rebase | Operation::RebaseInteractive => Ok("rebase"),
        Operation::CherryPick => Ok("cherry-pick"),
        Operation::Revert => Ok("revert"),
        other => Err(Error::NotStageable(format!(
            "there is no {} to continue or abort from here",
            other.label()
        ))),
    }
}

#[cfg(test)]
mod tests {

    /// A file with one ordinary conflict in the middle of it.
    const ONE: &str =
        "before\n<<<<<<< HEAD\nours line\n=======\ntheirs line\n>>>>>>> other\nafter\n";

    /// The same, written with `diff3` markers, which carry the base as well.
    const WITH_BASE: &str =
        "before\n<<<<<<< HEAD\nours\n||||||| base\nancestor\n=======\ntheirs\n>>>>>>> other\nafter\n";

    #[test]
    fn a_file_with_no_markers_has_no_regions() {
        assert!(regions("just some text\nand more\n").is_empty());
    }

    #[test]
    fn a_region_carries_both_sides_without_its_markers() {
        let regions = regions(ONE);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "ours line\n");
        assert_eq!(regions[0].theirs, "theirs line\n");
        assert!(regions[0].base.is_none());
    }

    #[test]
    fn a_region_knows_where_it_is_in_the_file() {
        // 1-based and inclusive, so the merged pane can point at it without a
        // second numbering scheme.
        let regions = regions(ONE);

        assert_eq!(regions[0].start_line, 2);
        assert_eq!(regions[0].end_line, 6);
    }

    #[test]
    fn diff3_markers_give_the_base_as_well() {
        let regions = regions(WITH_BASE);

        assert_eq!(regions[0].ours, "ours\n");
        assert_eq!(regions[0].base.as_deref(), Some("ancestor\n"));
        assert_eq!(regions[0].theirs, "theirs\n");
    }

    #[test]
    fn several_regions_are_numbered_in_order() {
        let text = format!("{ONE}{ONE}");
        let regions = regions(&text);

        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].index, 0);
        assert_eq!(regions[1].index, 1);
        assert!(regions[1].start_line > regions[0].end_line);
    }

    #[test]
    fn an_empty_side_is_empty_rather_than_a_blank_line() {
        // "contributed nothing" and "contributed a blank line" are different
        // results, and only one of them is right when a side deleted the lines.
        let text = "<<<<<<< HEAD\n=======\ntheirs\n>>>>>>> other\n";
        let regions = regions(text);

        assert_eq!(regions[0].ours, "");
        assert_eq!(regions[0].theirs, "theirs\n");
    }

    #[test]
    fn an_unterminated_region_is_not_a_region() {
        // A half-edited file, or a `<<<<<<<` that is really a string literal.
        // Guessing where it ends is how the wrong half of somebody's work gets
        // silently kept.
        let text = "<<<<<<< HEAD\nours\n=======\ntheirs\nno close here\n";
        assert!(regions(text).is_empty());
    }

    #[test]
    fn a_region_with_no_separator_is_not_a_region() {
        let text = "<<<<<<< HEAD\nours\n>>>>>>> other\n";
        assert!(regions(text).is_empty());
    }

    #[test]
    fn regions_before_a_broken_one_are_still_read() {
        let text = format!("{ONE}<<<<<<< HEAD\nunterminated\n");
        assert_eq!(regions(&text).len(), 1);
    }

    #[test]
    fn taking_ours_keeps_our_lines_and_removes_every_marker() {
        let resolved = resolve_region(ONE, 0, Side::Ours).expect("resolve");

        assert_eq!(resolved, "before\nours line\nafter\n");
    }

    #[test]
    fn taking_theirs_keeps_theirs() {
        let resolved = resolve_region(ONE, 0, Side::Theirs).expect("resolve");

        assert_eq!(resolved, "before\ntheirs line\nafter\n");
    }

    #[test]
    fn a_region_that_is_not_there_is_refused_by_name() {
        let error = resolve_region(ONE, 4, Side::Ours).unwrap_err();

        assert!(
            format!("{error}").contains("conflict 5"),
            "unexpected: {error}"
        );
    }

    #[test]
    fn resolving_one_of_several_leaves_the_others_alone() {
        let text = format!("{ONE}{ONE}");

        let resolved = resolve_region(&text, 0, Side::Ours).expect("resolve");

        assert!(resolved.starts_with("before\nours line\nafter\n"));
        // The second region is still a region, and is now the only one.
        assert_eq!(regions(&resolved).len(), 1);
    }

    #[test]
    fn resolving_all_of_them_leaves_no_markers_anywhere() {
        let text = format!("{ONE}middle\n{ONE}");

        let resolved = resolve_all(&text, Side::Theirs);

        assert!(regions(&resolved).is_empty());
        assert_eq!(
            resolved,
            "before\ntheirs line\nafter\nmiddle\nbefore\ntheirs line\nafter\n"
        );
    }

    #[test]
    fn resolving_all_of_a_file_with_no_conflicts_changes_nothing() {
        let text = "nothing to see\n";
        assert_eq!(resolve_all(text, Side::Ours), text);
    }

    #[test]
    fn a_side_that_deleted_the_lines_resolves_to_nothing_at_all() {
        let text = "keep\n<<<<<<< HEAD\ngoing away\n=======\n>>>>>>> other\nkeep too\n";

        assert_eq!(
            resolve_region(text, 0, Side::Theirs).expect("resolve"),
            "keep\nkeep too\n"
        );
    }

    #[test]
    fn the_base_is_dropped_along_with_the_markers() {
        let resolved = resolve_region(WITH_BASE, 0, Side::Ours).expect("resolve");

        assert_eq!(resolved, "before\nours\nafter\n");
    }

    #[test]
    fn taking_a_side_writes_the_working_file_and_leaves_the_index_conflicted() {
        // Taking a side is not resolving. The stages stay until somebody says
        // they looked at the result.
        let fixture = Fixture::conflicted();

        take(&fixture.open(), "shared.txt", Side::Ours).expect("take ours");

        assert_eq!(fixture.read("shared.txt"), "one\nOURS\nthree\n");
        assert_eq!(
            conflicted(&fixture.open()).expect("conflicted").len(),
            1,
            "the path is still conflicted until it is marked resolved"
        );
    }

    #[test]
    fn taking_theirs_takes_the_incoming_side() {
        let fixture = Fixture::conflicted();

        take(&fixture.open(), "shared.txt", Side::Theirs).expect("take theirs");

        assert_eq!(fixture.read("shared.txt"), "one\nTHEIRS\nthree\n");
    }

    #[test]
    fn writing_the_merged_file_puts_exactly_what_it_was_given_on_disk() {
        let fixture = Fixture::conflicted();

        write_merged(
            &fixture.open(),
            "shared.txt",
            "one\nboth, actually\nthree\n",
        )
        .expect("write");

        assert_eq!(fixture.read("shared.txt"), "one\nboth, actually\nthree\n");
        assert_eq!(conflicted(&fixture.open()).expect("conflicted").len(), 1);
    }

    #[test]
    fn marking_resolved_collapses_the_index_stages() {
        // The only real check that a resolution worked: three stages become
        // one entry.
        let fixture = Fixture::conflicted();
        take(&fixture.open(), "shared.txt", Side::Ours).expect("take");

        mark_resolved(&fixture.open(), &["shared.txt".to_string()]).expect("mark");

        assert!(conflicted(&fixture.open()).expect("conflicted").is_empty());
    }

    #[test]
    fn continuing_a_merge_once_everything_is_resolved_makes_the_commit() {
        let fixture = Fixture::conflicted();
        take(&fixture.open(), "shared.txt", Side::Theirs).expect("take");
        mark_resolved(&fixture.open(), &["shared.txt".to_string()]).expect("mark");

        continue_operation(&fixture.open(), Operation::Merge).expect("continue");

        assert_eq!(operation(&fixture.open()), Operation::None);
        // A merge commit has two parents, which is the thing that was being
        // attempted in the first place.
        let parents = fixture.git(&["rev-list", "--parents", "-n", "1", "HEAD"]);
        assert_eq!(
            parents.split_whitespace().count(),
            3,
            "unexpected: {parents}"
        );
    }

    #[test]
    fn continuing_with_a_file_still_conflicted_is_refused() {
        let fixture = Fixture::conflicted();

        let error = continue_operation(&fixture.open(), Operation::Merge).unwrap_err();

        assert!(!format!("{error}").is_empty());
        assert_eq!(operation(&fixture.open()), Operation::Merge);
    }

    #[test]
    fn aborting_a_merge_puts_the_file_and_the_repository_back() {
        let fixture = Fixture::conflicted();
        take(&fixture.open(), "shared.txt", Side::Theirs).expect("take");

        abort_operation(&fixture.open(), Operation::Merge).expect("abort");

        assert_eq!(operation(&fixture.open()), Operation::None);
        assert_eq!(fixture.read("shared.txt"), "one\nOURS\nthree\n");
        assert!(conflicted(&fixture.open()).expect("conflicted").is_empty());
    }

    #[test]
    fn continuing_or_aborting_a_bisect_is_refused_rather_than_guessed() {
        // `git bisect --abort` means something quite different from the others,
        // and this screen never starts one.
        let error = resumable(Operation::Bisect).unwrap_err();
        assert!(format!("{error}").contains("bisect"), "unexpected: {error}");
    }

    #[test]
    fn an_interactive_rebase_continues_as_a_rebase() {
        assert_eq!(
            resumable(Operation::RebaseInteractive).expect("rebase"),
            "rebase"
        );
    }
    use super::*;
    use crate::fixture::Fixture;
    use crate::status;

    #[test]
    fn the_conflicted_paths_match_what_git_reports() {
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        let ours: Vec<String> = conflicted(&repo)
            .expect("conflicts")
            .into_iter()
            .map(|file| file.path)
            .collect();

        let theirs: Vec<String> = fixture
            .git(&["diff", "--name-only", "--diff-filter=U"])
            .lines()
            .map(str::to_string)
            .collect();

        assert_eq!(ours, theirs);
    }

    #[test]
    fn the_status_walk_finds_the_same_paths_by_a_different_route() {
        // Two ways of finding the same thing is worth more than one way used
        // twice: this module reads the index, `status` walks the working copy.
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        let from_index: Vec<String> = conflicted(&repo)
            .expect("conflicts")
            .into_iter()
            .map(|file| file.path)
            .collect();
        let from_walk: Vec<String> = status::working_copy(&repo)
            .expect("status")
            .conflicted
            .into_iter()
            .map(|entry| entry.path)
            .collect();

        assert_eq!(from_index, from_walk);
    }

    #[test]
    fn each_side_is_what_git_show_reports_for_that_stage() {
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        let sides = sides(&repo, "shared.txt").expect("sides");

        for (side, stage) in [(&sides.base, "1"), (&sides.ours, "2"), (&sides.theirs, "3")] {
            let expected = fixture.git(&["show", &format!(":{stage}:shared.txt")]);
            assert_eq!(
                side.as_ref().expect("a side").text,
                expected,
                "stage {stage}"
            );
        }
    }

    #[test]
    fn the_merged_side_is_the_file_on_disk_with_its_markers() {
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        let merged = sides(&repo, "shared.txt")
            .expect("sides")
            .merged
            .expect("a file on disk");

        assert!(merged.text.contains("<<<<<<<"), "{}", merged.text);
        assert!(merged.text.contains(">>>>>>>"), "{}", merged.text);
        assert_eq!(
            merged.text,
            std::fs::read_to_string(fixture.path().join("shared.txt")).expect("the file")
        );
    }

    #[test]
    fn a_file_added_on_both_sides_has_no_base_and_says_so() {
        let fixture = Fixture::added_on_both_sides();
        let repo = fixture.open();

        let file = &conflicted(&repo).expect("conflicts")[0];
        assert_eq!(file.kind, ConflictKind::BothAdded);

        let sides = sides(&repo, "both.txt").expect("sides");
        assert!(sides.base.is_none(), "there is no common ancestor");
        assert!(sides.ours.is_some());
        assert!(sides.theirs.is_some());
    }

    #[test]
    fn a_delete_modify_conflict_names_the_side_that_deleted_it() {
        // An empty pane would read as "they emptied the file", which is a
        // different thing and one that loses work if acted on.
        let fixture = Fixture::deleted_on_one_side();
        let repo = fixture.open();

        let file = conflicted(&repo)
            .expect("conflicts")
            .into_iter()
            .find(|file| file.path == "gone.txt")
            .expect("the deleted path");

        assert_eq!(file.kind, ConflictKind::DeletedByThem);

        let sides = sides(&repo, "gone.txt").expect("sides");
        assert!(sides.base.is_some(), "it existed before");
        assert!(sides.ours.is_some(), "we still have it");
        assert!(sides.theirs.is_none(), "they deleted it");
    }

    #[test]
    fn a_binary_side_is_named_rather_than_decoded() {
        let fixture = Fixture::binary_conflict();
        let repo = fixture.open();

        let sides = sides(&repo, "logo.bin").expect("sides");
        let ours = sides.ours.expect("our side");

        assert!(ours.binary);
        assert!(ours.text.is_empty(), "nothing honest to put there");
        assert!(ours.bytes > 0, "the size is still known");
    }

    #[test]
    fn a_clean_repository_has_no_conflicts_and_no_operation() {
        let fixture = Fixture::woven();
        let repo = fixture.open();

        assert!(conflicted(&repo).expect("conflicts").is_empty());
        assert_eq!(operation(&repo), Operation::None);
    }

    #[test]
    fn the_operation_is_read_from_the_repository_rather_than_guessed() {
        // Merge, rebase, cherry-pick and revert all leave conflicts behind.
        // Naming the wrong one sends the user to the wrong command to get out.
        assert_eq!(operation(&Fixture::conflicted().open()), Operation::Merge);
        assert_eq!(
            operation(&Fixture::cherry_pick_conflict().open()),
            Operation::CherryPick
        );
    }

    #[test]
    fn asking_about_a_path_that_is_not_conflicted_is_an_error() {
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        assert!(sides(&repo, "untouched.txt").is_err());
        assert!(sides(&repo, "no/such/file.txt").is_err());
    }

    #[test]
    fn an_empty_repository_has_no_conflicts_rather_than_a_failure() {
        let fixture = Fixture::empty();
        let repo = fixture.open();

        assert!(conflicted(&repo).expect("conflicts").is_empty());
        assert_eq!(operation(&repo), Operation::None);
    }

    #[test]
    fn reading_every_side_never_writes_to_the_repository() {
        // Both readings are taken with nothing in between, because a status
        // walk rewrites the index to cache stat information — that is what
        // failed the first version of the same test on `repo::summary`.
        let fixture = Fixture::conflicted();
        let repo = fixture.open();
        let index_path = fixture.path().join(".git/index");

        let mtime = || {
            std::fs::metadata(&index_path)
                .expect("index")
                .modified()
                .expect("a modification time")
        };

        let before = mtime();
        for file in conflicted(&repo).expect("conflicts") {
            sides(&repo, &file.path).expect("sides");
        }
        let after = mtime();

        assert_eq!(before, after, "the index was rewritten");
        assert!(
            !fixture.path().join(".git/index.lock").exists(),
            "a lock was left behind"
        );
    }

    #[test]
    fn the_state_carries_the_operation_and_the_files_together() {
        let state = state(&Fixture::conflicted().open()).expect("state");

        assert_eq!(state.operation, Operation::Merge);
        assert_eq!(state.files.len(), 1);
    }

    #[test]
    fn an_operation_names_itself_the_way_git_does() {
        assert_eq!(Operation::Merge.label(), "merge");
        assert_eq!(Operation::RebaseInteractive.label(), "interactive rebase");
        assert_eq!(Operation::None.label(), "none");
    }

    #[test]
    fn a_line_count_treats_a_trailing_newline_as_ending_the_last_line() {
        assert_eq!(line_count(""), 0);
        assert_eq!(line_count("one\n"), 1);
        assert_eq!(line_count("one\ntwo"), 2);
        assert_eq!(line_count("one\ntwo\n"), 2);
    }

    #[test]
    fn a_nul_byte_after_the_sniff_window_is_still_text() {
        // Git's rule, so Spagitty calls the same files binary that git does.
        let mut late = vec![b'a'; BINARY_SNIFF_BYTES + 10];
        late[BINARY_SNIFF_BYTES + 5] = 0;

        assert!(!is_binary(&late));
        assert!(is_binary(b"\0start"));
    }

    #[test]
    fn a_side_over_the_ceiling_is_reported_rather_than_sent_to_the_screen() {
        let huge = vec![b'a'; MAX_SIDE_BYTES + 1];
        let side = side_from(&huge);

        assert!(side.too_large);
        assert!(side.text.is_empty());
        assert_eq!(side.bytes, MAX_SIDE_BYTES + 1);
    }

    #[test]
    fn stages_map_to_the_kind_of_conflict_they_describe() {
        use ConflictKind::*;

        assert_eq!(
            ConflictKind::from_stages(true, true, true),
            Some(BothModified)
        );
        assert_eq!(
            ConflictKind::from_stages(false, true, true),
            Some(BothAdded)
        );
        assert_eq!(
            ConflictKind::from_stages(true, false, true),
            Some(DeletedByUs)
        );
        assert_eq!(
            ConflictKind::from_stages(true, true, false),
            Some(DeletedByThem)
        );
        assert_eq!(
            ConflictKind::from_stages(true, false, false),
            None,
            "a path with only a base is not a conflict anyone can act on"
        );
    }
}

#[cfg(test)]
mod settle_tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn settling_with_text_writes_it_and_marks_it_resolved() {
        let fixture = Fixture::conflicted();
        let repo = fixture.open();

        settle(&repo, "shared.txt", Some("one\nBOTH\nthree\n"), None).unwrap();

        assert_eq!(fixture.read("shared.txt"), "one\nBOTH\nthree\n");
        assert!(conflicted(&fixture.open()).unwrap().is_empty());
    }

    #[test]
    fn text_with_markers_is_refused_and_nothing_is_staged() {
        let fixture = Fixture::conflicted();
        let error = settle(
            &fixture.open(),
            "shared.txt",
            Some("<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n"),
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("markers"), "{error}");
        assert_eq!(conflicted(&fixture.open()).unwrap().len(), 1);
    }

    #[test]
    fn taking_a_side_takes_it_and_marks_it_resolved() {
        let fixture = Fixture::conflicted();
        settle(&fixture.open(), "shared.txt", None, Some(Side::Theirs)).unwrap();
        assert_eq!(fixture.read("shared.txt"), "one\nTHEIRS\nthree\n");
        assert!(conflicted(&fixture.open()).unwrap().is_empty());
    }

    #[test]
    fn taking_the_side_that_deleted_it_deletes_it() {
        let fixture = Fixture::deleted_on_one_side();
        settle(&fixture.open(), "gone.txt", None, Some(Side::Theirs)).unwrap();
        assert!(!fixture.at("gone.txt").exists());
        assert!(conflicted(&fixture.open()).unwrap().is_empty());
    }
}

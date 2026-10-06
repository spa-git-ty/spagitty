// SPDX-License-Identifier: GPL-3.0-or-later

//! What resolving needs to see (FEAT-102): every conflicted file of a dry run,
//! each side whole, the merged text with diff3 markers, and for each region
//! where its lines sit in each side's file and which commit put them there.
//!
//! Nothing here writes: it is the dry run again, read in full. The screen
//! keeps the choices; [`super::land`] is what turns them into a commit.

use std::path::Path;

use serde::Serialize;

use super::{dry_run, preferred, short, CommitLine, Conflicted};
use crate::conflicts::{self, ConflictKind, ConflictSide};
use crate::error::{Error, Result};
use crate::repo::workdir;
use crate::shell;

/// One conflict region, with where it came from on each side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionSource {
    /// Its position in the file, from 0, as [`conflicts::regions`] numbers it.
    pub index: usize,
    /// The first line of A's version in A's file, from 1. `None` when it could
    /// not be found, or A has no lines here.
    pub a_line: Option<usize>,
    pub b_line: Option<usize>,
    /// The newest commit since the split that changed A's lines here.
    pub a_commit: Option<CommitLine>,
    pub b_commit: Option<CommitLine>,
}

/// One conflicted file, every side of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConflict {
    pub path: String,
    pub kind: ConflictKind,
    pub base: Option<ConflictSide>,
    /// A's version: stage 2 of the dry run.
    pub a: Option<ConflictSide>,
    /// B's version: stage 3.
    pub b: Option<ConflictSide>,
    /// The merged file with diff3 markers, when the conflict is in its lines.
    pub merged: Option<ConflictSide>,
    pub regions: Vec<RegionSource>,
}

/// Every conflict of merging A and B.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflicts {
    pub base: String,
    pub base_short: String,
    pub a_tip: String,
    pub b_tip: String,
    pub files: Vec<FileConflict>,
}

/// Read every conflict of merging `a` and `b`, without writing.
pub fn conflict_files(repo: &gix::Repository, a: &str, b: &str) -> Result<MergeConflicts> {
    let dir = workdir(repo)?;
    let a_tip = shell::commit_id(dir, a)?;
    let b_tip = shell::commit_id(dir, b)?;
    let base = shell::merge_base(dir, &a_tip, &b_tip)?
        .ok_or_else(|| Error::NotStageable(format!("{a} and {b} share no history")))?;
    let outcome = dry_run(repo, &a_tip, &b_tip, preferred(dir))?;

    let files = outcome
        .conflicted
        .iter()
        .map(|conflicted| file(repo, dir, conflicted, &base, &a_tip, &b_tip))
        .collect::<Result<Vec<_>>>()?;

    Ok(MergeConflicts {
        base_short: short(&base),
        base,
        a_tip,
        b_tip,
        files,
    })
}

fn file(
    repo: &gix::Repository,
    dir: &Path,
    conflicted: &Conflicted,
    base: &str,
    a_tip: &str,
    b_tip: &str,
) -> Result<FileConflict> {
    let a = blob(repo, conflicted.ours.as_ref())?;
    let b = blob(repo, conflicted.theirs.as_ref())?;
    let merged = conflicted
        .merged
        .as_deref()
        .filter(|_| {
            matches!(
                conflicted.kind,
                ConflictKind::BothModified | ConflictKind::BothAdded
            )
        })
        .map(conflicts::side_from)
        .filter(|side| !side.binary && !side.too_large);

    let regions = match &merged {
        Some(merged) => {
            let found = conflicts::regions(&merged.text);
            let merged_lines: Vec<&str> = merged.text.lines().collect();
            let a_lines: Vec<&str> = a
                .as_ref()
                .map_or(Vec::new(), |side| side.text.lines().collect());
            let b_lines: Vec<&str> = b
                .as_ref()
                .map_or(Vec::new(), |side| side.text.lines().collect());
            let a_range = format!("{base}..{a_tip}");
            let b_range = format!("{base}..{b_tip}");
            let (mut a_from, mut b_from) = (0, 0);
            found
                .iter()
                .map(|region| {
                    let before = context_before(&merged_lines, &found, region.index);
                    let ours: Vec<&str> = region.ours.lines().collect();
                    let theirs: Vec<&str> = region.theirs.lines().collect();
                    let a_line = locate(&a_lines, &before, &ours, &mut a_from);
                    let b_line = locate(&b_lines, &before, &theirs, &mut b_from);
                    Ok(RegionSource {
                        index: region.index,
                        a_commit: introduced(dir, &a_range, &conflicted.path, a_line, ours.len())?,
                        b_commit: introduced(
                            dir,
                            &b_range,
                            &conflicted.path,
                            b_line,
                            theirs.len(),
                        )?,
                        a_line: a_line.filter(|_| !ours.is_empty()),
                        b_line: b_line.filter(|_| !theirs.is_empty()),
                    })
                })
                .collect::<Result<Vec<_>>>()?
        }
        None => Vec::new(),
    };

    Ok(FileConflict {
        path: conflicted.path.clone(),
        kind: conflicted.kind,
        base: blob(repo, conflicted.base.as_ref())?,
        a,
        b,
        merged,
        regions,
    })
}

fn blob(repo: &gix::Repository, stage: Option<&(String, String)>) -> Result<Option<ConflictSide>> {
    let Some((_, blob)) = stage else {
        return Ok(None);
    };
    let id = gix::ObjectId::from_hex(blob.as_bytes()).map_err(|e| Error::Diff(e.to_string()))?;
    let object = repo
        .find_object(id)
        .map_err(|e| Error::Diff(e.to_string()))?;
    Ok(Some(conflicts::side_from(&object.detach().data)))
}

/// Up to three merged lines just before region `index`, stopping at the
/// region before it.
pub(crate) fn context_before<'a>(
    merged: &[&'a str],
    regions: &[conflicts::Region],
    index: usize,
) -> Vec<&'a str> {
    let start = regions[index].start_line - 1;
    let floor = if index == 0 {
        0
    } else {
        regions[index - 1].end_line
    };
    merged[floor.max(start.saturating_sub(3))..start].to_vec()
}

/// Where `block` sits in `file`, from `*from` on, as a 1-based line: the first
/// match whose preceding line agrees with the context, else the first match.
/// An empty block is placed just after the context. `*from` moves past it, so
/// the regions of one file are found in order.
pub(crate) fn locate(
    file: &[&str],
    before: &[&str],
    block: &[&str],
    from: &mut usize,
) -> Option<usize> {
    let fits = |at: usize| at + block.len() <= file.len() && file[at..at + block.len()] == *block;
    let after_context = |at: usize| match before.last() {
        Some(line) => at > 0 && file[at - 1] == *line,
        None => true,
    };

    let found = if block.is_empty() {
        if before.is_empty() {
            Some(*from)
        } else {
            (*from..=file.len().saturating_sub(before.len()))
                .find(|&at| file[at..at + before.len()] == *before)
                .map(|at| at + before.len())
        }
    } else {
        let candidates: Vec<usize> = (*from..file.len()).filter(|&at| fits(at)).collect();
        candidates
            .iter()
            .copied()
            .find(|&at| after_context(at))
            .or_else(|| candidates.first().copied())
    }?;

    *from = found + block.len();
    Some(found + 1)
}

/// The newest commit in `range` that changed lines `line..line + len` of
/// `path`, or, when there are no lines to blame, that touched the file.
fn introduced(
    dir: &Path,
    range: &str,
    path: &str,
    line: Option<usize>,
    len: usize,
) -> Result<Option<CommitLine>> {
    if let (Some(line), true) = (line, len > 0) {
        if let Ok(out) = shell::blame_lines(dir, range, path, line, line + len - 1) {
            if let Some(found) = newest_blamed(&out) {
                return Ok(Some(found));
            }
        }
    }
    let out = shell::last_touch(dir, range, path)?;
    let mut parts = out.trim().split('\u{1f}');
    Ok(match (parts.next(), parts.next(), parts.next()) {
        (Some(id), Some(short), Some(summary)) if !id.is_empty() => Some(CommitLine {
            id: id.to_string(),
            short: short.to_string(),
            summary: summary.to_string(),
            time: 0,
        }),
        _ => None,
    })
}

/// The newest non-boundary commit in `blame --porcelain` output.
fn newest_blamed(out: &str) -> Option<CommitLine> {
    struct Entry {
        id: String,
        summary: String,
        time: i64,
        boundary: bool,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut current: Option<usize> = None;
    for line in out.lines() {
        let head = line.split(' ').next().unwrap_or_default();
        if head.len() >= 40 && head.chars().all(|c| c.is_ascii_hexdigit()) {
            current = Some(match entries.iter().position(|entry| entry.id == head) {
                Some(at) => at,
                None => {
                    entries.push(Entry {
                        id: head.to_string(),
                        summary: String::new(),
                        time: 0,
                        boundary: false,
                    });
                    entries.len() - 1
                }
            });
        } else if let Some(at) = current {
            if let Some(summary) = line.strip_prefix("summary ") {
                entries[at].summary = summary.to_string();
            } else if let Some(time) = line.strip_prefix("committer-time ") {
                entries[at].time = time.trim().parse().unwrap_or(0);
            } else if line == "boundary" {
                entries[at].boundary = true;
            }
        }
    }
    entries
        .into_iter()
        .filter(|entry| !entry.boundary)
        .max_by_key(|entry| entry.time)
        .map(|entry| CommitLine {
            short: short(&entry.id),
            id: entry.id,
            summary: entry.summary,
            time: entry.time,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn a_block_is_found_after_its_context() {
        let file = ["a", "x", "b", "x", "c"];
        let mut from = 0;
        assert_eq!(locate(&file, &["b"], &["x"], &mut from), Some(4));
        assert_eq!(from, 4);
    }

    #[test]
    fn an_empty_block_sits_after_its_context() {
        let file = ["a", "b", "c"];
        let mut from = 0;
        assert_eq!(locate(&file, &["a", "b"], &[], &mut from), Some(3));
    }

    #[test]
    fn a_block_that_is_not_there_is_not_found() {
        let file = ["a", "b"];
        let mut from = 0;
        assert_eq!(locate(&file, &[], &["zzz"], &mut from), None);
        assert_eq!(from, 0, "nothing moved");
    }

    /// Two conflicts in one file, the second after lines feat added above it,
    /// so the two sides number it differently.
    fn two_regions() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("f.txt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n");
        fixture.git(&["add", "-A"]);
        fixture.commit_at("Base", 1_000);
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.write("f.txt", "1\nTWO feat\n3\nnew\n4\n5\n6\n7\nEIGHT feat\n9\n");
        fixture.commit_all_at("Feat rewrites two and eight", 2_000);
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("f.txt", "1\nTWO main\n3\n4\n5\n6\n7\nEIGHT main\n9\n");
        fixture.commit_all_at("Main rewrites two and eight", 3_000);
        fixture
    }

    #[test]
    fn every_region_says_where_it_is_on_each_side_and_who_wrote_it() {
        let fixture = two_regions();
        let found = conflict_files(&fixture.open(), "main", "feat").expect("conflicts");

        assert_eq!(found.files.len(), 1);
        let file = &found.files[0];
        assert_eq!(file.path, "f.txt");
        assert!(
            file.merged.as_ref().unwrap().text.contains("|||||||"),
            "diff3 markers"
        );
        assert_eq!(
            file.a.as_ref().unwrap().text,
            "1\nTWO main\n3\n4\n5\n6\n7\nEIGHT main\n9\n"
        );
        assert_eq!(file.regions.len(), 2);

        assert_eq!(file.regions[0].a_line, Some(2));
        assert_eq!(file.regions[0].b_line, Some(2));
        assert_eq!(file.regions[1].a_line, Some(8));
        assert_eq!(
            file.regions[1].b_line,
            Some(9),
            "feat's added line moves it down"
        );

        let a_commit = file.regions[0].a_commit.as_ref().expect("a commit on main");
        assert_eq!(a_commit.summary, "Main rewrites two and eight");
        let b_commit = file.regions[1].b_commit.as_ref().expect("a commit on feat");
        assert_eq!(b_commit.summary, "Feat rewrites two and eight");
    }

    #[test]
    fn reading_the_conflicts_writes_nothing() {
        let fixture = two_regions();
        let index = std::fs::read(fixture.at(".git/index")).unwrap();
        let refs = fixture.git(&["for-each-ref"]);
        conflict_files(&fixture.open(), "main", "feat").unwrap();
        assert_eq!(std::fs::read(fixture.at(".git/index")).unwrap(), index);
        assert_eq!(fixture.git(&["for-each-ref"]), refs);
        assert!(fixture.git(&["status", "--porcelain"]).is_empty());
    }

    #[test]
    fn a_deleted_file_has_no_regions_and_one_side() {
        let fixture = Fixture::empty();
        fixture.write("gone.txt", "x\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.git(&["rm", "-q", "gone.txt"]);
        fixture.commit("Delete it");
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("gone.txt", "y\n");
        fixture.commit_all("Change it");

        let found = conflict_files(&fixture.open(), "main", "feat").unwrap();

        let file = &found.files[0];
        assert_eq!(file.kind, ConflictKind::DeletedByThem);
        assert!(file.a.is_some() && file.b.is_none());
        assert!(file.merged.is_none());
        assert!(file.regions.is_empty());
    }
}

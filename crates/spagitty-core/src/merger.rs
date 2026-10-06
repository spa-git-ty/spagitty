// SPDX-License-Identifier: GPL-3.0-or-later

//! Merger (FEAT-100): any two branches, and what merging them would do,
//! before anything is written.
//!
//! The graph's drag merges into the branch that is checked out, and the first
//! anyone hears of a conflict is git stopping halfway. Merger takes two
//! commits, A and B, and answers the questions a merge raises *first*: where
//! the two split, what each has that the other lacks, which files change, and
//! whether and where they conflict.
//!
//! # A dry run writes nothing anybody reads
//!
//! `git merge-tree --write-tree` (git 2.38) merges two commits without an index
//! or a working tree: it prints the tree it would commit, the stages of every
//! conflicted path and why each conflicted. It writes objects — the merged
//! blobs, with conflict markers — and nothing else: no ref, no index, no file.
//! An object nothing points at is garbage git collects in its own time, and is
//! not something any screen or tool reads.
//!
//! An older git has no such command, so the fallback is the same merge in a
//! scratch worktree under the repository's own git directory: `merge
//! --no-commit` there, read what it left, and remove the worktree. The
//! repository's index, working tree and refs are not touched either way.
//!
//! # A and B, always
//!
//! The dry run is always `A` merged with `B`, ours and theirs in that order,
//! whichever way the result is going to land: the conflicts of merging B into
//! A and of merging A into B are the same regions with the sides named the
//! other way round. The screen shows A | Result | B whatever the direction, so
//! one dry run per pair is all it needs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::conflicts::{self, ConflictKind};
use crate::error::{Error, Result};
use crate::repo::workdir;
use crate::shell;

/// How many of each side's newest commits a card lists.
pub const NEWEST: usize = 3;

/// The first git with `merge-tree --write-tree`.
const MERGE_TREE_SINCE: (u32, u32) = (2, 38);

/// What a name the screen was given turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefKind {
    /// A branch here: the only kind a merge can land on.
    Local,
    /// A remote-tracking branch, such as `origin/main`.
    Remote,
    Tag,
    /// Anything else `rev-parse` accepts: an id, `HEAD~2`.
    Commit,
}

/// One commit, as a card lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitLine {
    pub id: String,
    pub short: String,
    pub summary: String,
    /// Commit time, unix seconds.
    pub time: i64,
}

/// One of the two branches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Side {
    /// The name as given: `main`, `origin/main`, `v1.2`.
    pub name: String,
    pub kind: RefKind,
    pub tip: String,
    pub short: String,
    /// Commits since the two split.
    pub ahead: usize,
    /// The newest of them, newest first, at most [`NEWEST`].
    pub newest: Vec<CommitLine>,
    /// How many of them touch a file that conflicts: the most times a rebase
    /// of this side can stop.
    pub touching: usize,
    /// When the tip was committed, unix seconds.
    pub time: i64,
    /// The worktree it is checked out in, when it is a branch that is.
    pub checked_out: Option<String>,
}

/// Which side changed a file since the split, and whether the two collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Touch {
    /// Both changed it and git cannot put the two together.
    Conflict,
    /// Both changed it, and it merges on its own.
    Both,
    /// Only A changed it.
    A,
    /// Only B changed it.
    B,
}

/// A file either branch changed since the split.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanFile {
    pub path: String,
    pub touch: Touch,
    /// A added it since the split.
    pub added_by_a: bool,
    pub added_by_b: bool,
    /// A deleted it since the split.
    pub deleted_by_a: bool,
    pub deleted_by_b: bool,
    /// Conflict regions in it: one for a file that conflicts as a whole —
    /// deleted on one side, binary — and none when it merges.
    pub conflicts: usize,
    /// What kind of conflict, for a conflicted file.
    pub kind: Option<ConflictKind>,
}

/// How the dry run was done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DryRun {
    /// `merge-tree --write-tree`: no worktree at all.
    MergeTree,
    /// A scratch worktree, for a git older than 2.38.
    Worktree,
}

/// What merging A and B would do, worked out without writing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub a: Side,
    pub b: Side,
    /// Where the two split.
    pub base: String,
    pub base_short: String,
    /// A's history holds all of B: nothing comes in from B.
    pub a_has_b: bool,
    /// B's history holds all of A.
    pub b_has_a: bool,
    /// Every file either side changed since the split, conflicts first.
    pub files: Vec<PlanFile>,
    /// Conflict regions across every file.
    pub conflicts: usize,
    pub method: DryRun,
}

/// One conflicted path, as the dry run left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Conflicted {
    pub path: String,
    pub kind: ConflictKind,
    /// Mode and blob of each stage: the base, A and B.
    pub base: Option<(String, String)>,
    pub ours: Option<(String, String)>,
    pub theirs: Option<(String, String)>,
    /// The merged file with diff3 markers, when there is one.
    pub merged: Option<Vec<u8>>,
}

/// What a dry run found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// The tree `merge-tree` would commit, markers and all. `None` from the
    /// worktree fallback, which has no tree until its conflicts are resolved.
    pub tree: Option<String>,
    pub conflicted: Vec<Conflicted>,
    pub method: DryRun,
}

/// Merge A and B without writing, and say what that would do.
pub fn forecast(repo: &gix::Repository, a: &str, b: &str) -> Result<Forecast> {
    let dir = workdir(repo)?;
    let a_tip = shell::commit_id(dir, a)?;
    let b_tip = shell::commit_id(dir, b)?;
    let base = shell::merge_base(dir, &a_tip, &b_tip)?
        .ok_or_else(|| Error::NotStageable(format!("{a} and {b} share no history")))?;

    let outcome = dry_run(repo, &a_tip, &b_tip, preferred(dir))?;
    let worktrees = checked_out(repo);
    let paths: Vec<String> = outcome.conflicted.iter().map(|c| c.path.clone()).collect();

    Ok(Forecast {
        a: side(repo, dir, a, &a_tip, &base, &worktrees, &paths)?,
        b: side(repo, dir, b, &b_tip, &base, &worktrees, &paths)?,
        base_short: short(&base),
        a_has_b: shell::is_ancestor(dir, &b_tip, &a_tip)?,
        b_has_a: shell::is_ancestor(dir, &a_tip, &b_tip)?,
        files: files(dir, &base, &a_tip, &b_tip, &outcome)?,
        conflicts: outcome.conflicted.iter().map(regions_in).sum(),
        method: outcome.method,
        base,
    })
}

/// The dry run this git can do.
pub(crate) fn preferred(dir: &Path) -> DryRun {
    match shell::version_number(dir) {
        Some(version) if version >= MERGE_TREE_SINCE => DryRun::MergeTree,
        _ => DryRun::Worktree,
    }
}

/// Merge `ours` and `theirs` without touching the repository's index,
/// working tree or refs.
pub(crate) fn dry_run(
    repo: &gix::Repository,
    ours: &str,
    theirs: &str,
    method: DryRun,
) -> Result<Outcome> {
    let dir = workdir(repo)?;
    match method {
        DryRun::MergeTree => {
            let (_, out) = shell::merge_tree(dir, ours, theirs)?;
            let parsed = parse_merge_tree(&out);
            let tree = parsed.tree.clone();
            let conflicted = parsed
                .stages
                .into_iter()
                .map(|(path, stages)| {
                    let merged = blob_at(repo, &tree, &path);
                    conflicted_from(path, stages, merged)
                })
                .collect();
            Ok(Outcome {
                tree: Some(tree),
                conflicted,
                method,
            })
        }
        DryRun::Worktree => {
            let scratch = Scratch::add(repo, ours)?;
            shell::scratch_merge(scratch.path(), theirs)?;
            let unmerged = shell::unmerged_entries(scratch.path())?;
            let conflicted = parse_unmerged(&unmerged)
                .into_iter()
                .map(|(path, stages)| {
                    let merged = std::fs::read(scratch.path().join(&path)).ok();
                    conflicted_from(path, stages, merged)
                })
                .collect();
            Ok(Outcome {
                tree: None,
                conflicted,
                method,
            })
        }
    }
}

/// How many regions a conflicted file holds: its markers, or one for a file
/// that conflicts as a whole.
fn regions_in(conflicted: &Conflicted) -> usize {
    let found = conflicted
        .merged
        .as_deref()
        .filter(|_| {
            conflicted.kind == ConflictKind::BothModified
                || conflicted.kind == ConflictKind::BothAdded
        })
        .map(|bytes| conflicts::regions(&String::from_utf8_lossy(bytes)).len())
        .unwrap_or(0);
    found.max(1)
}

/// The stages of one path, by number.
type Stages = BTreeMap<u8, (String, String)>;

fn conflicted_from(path: String, stages: Stages, merged: Option<Vec<u8>>) -> Conflicted {
    let base = stages.get(&1).cloned();
    let ours = stages.get(&2).cloned();
    let theirs = stages.get(&3).cloned();
    let kind = match (base.is_some(), ours.is_some(), theirs.is_some()) {
        (_, false, _) => ConflictKind::DeletedByUs,
        (_, _, false) => ConflictKind::DeletedByThem,
        (false, true, true) => ConflictKind::BothAdded,
        (true, true, true) => ConflictKind::BothModified,
    };
    Conflicted {
        path,
        kind,
        base,
        ours,
        theirs,
        merged,
    }
}

/// `merge-tree -z` output: the tree, then each conflicted stage, then — after
/// an empty field — the messages, which are not needed here.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MergeTree {
    pub tree: String,
    pub stages: BTreeMap<String, Stages>,
}

pub(crate) fn parse_merge_tree(out: &str) -> MergeTree {
    let mut fields = out.split('\0');
    let tree = fields.next().unwrap_or_default().trim().to_string();
    let mut stages: BTreeMap<String, Stages> = BTreeMap::new();
    for field in fields {
        let Some((path, stage, entry)) = stage_entry(field) else {
            break;
        };
        stages.entry(path).or_default().insert(stage, entry);
    }
    MergeTree { tree, stages }
}

/// `ls-files -u -z` output: one stage per field.
fn parse_unmerged(out: &str) -> BTreeMap<String, Stages> {
    let mut stages: BTreeMap<String, Stages> = BTreeMap::new();
    for field in out.split('\0') {
        if let Some((path, stage, entry)) = stage_entry(field) {
            stages.entry(path).or_default().insert(stage, entry);
        }
    }
    stages
}

/// `<mode> <blob> <stage>\t<path>`.
fn stage_entry(field: &str) -> Option<(String, u8, (String, String))> {
    let (meta, path) = field.split_once('\t')?;
    let mut parts = meta.split(' ');
    let mode = parts.next()?;
    let blob = parts.next()?;
    let stage: u8 = parts.next()?.parse().ok()?;
    if !(1..=3).contains(&stage) || mode.is_empty() || blob.is_empty() {
        return None;
    }
    Some((
        path.to_string(),
        stage,
        (mode.to_string(), blob.to_string()),
    ))
}

/// The bytes at `path` in `tree`, when there is a blob there.
fn blob_at(repo: &gix::Repository, tree: &str, path: &str) -> Option<Vec<u8>> {
    let id = gix::ObjectId::from_hex(tree.as_bytes()).ok()?;
    let tree = repo.find_object(id).ok()?.try_into_tree().ok()?;
    let entry = tree.lookup_entry_by_path(path).ok()??;
    let object = entry.object().ok()?;
    (object.kind == gix::object::Kind::Blob).then(|| object.detach().data)
}

/// Every file either side changed since `base`, conflicts first.
fn files(dir: &Path, base: &str, a: &str, b: &str, outcome: &Outcome) -> Result<Vec<PlanFile>> {
    let in_a = statuses(&shell::changed_names(dir, base, a)?);
    let in_b = statuses(&shell::changed_names(dir, base, b)?);
    let conflicted: BTreeMap<&str, &Conflicted> = outcome
        .conflicted
        .iter()
        .map(|c| (c.path.as_str(), c))
        .collect();

    let paths: BTreeSet<&str> = in_a
        .keys()
        .chain(in_b.keys())
        .map(String::as_str)
        .chain(conflicted.keys().copied())
        .collect();

    let mut files: Vec<PlanFile> = paths
        .into_iter()
        .map(|path| {
            let a_status = in_a.get(path).copied();
            let b_status = in_b.get(path).copied();
            let conflict = conflicted.get(path);
            let touch = match (conflict, a_status, b_status) {
                (Some(_), _, _) => Touch::Conflict,
                (None, Some(_), Some(_)) => Touch::Both,
                (None, Some(_), None) => Touch::A,
                _ => Touch::B,
            };
            PlanFile {
                path: path.to_string(),
                touch,
                added_by_a: a_status == Some('A'),
                added_by_b: b_status == Some('A'),
                deleted_by_a: a_status == Some('D'),
                deleted_by_b: b_status == Some('D'),
                conflicts: conflict.map_or(0, |c| regions_in(c)),
                kind: conflict.map(|c| c.kind),
            }
        })
        .collect();

    // Conflicts, then both, then what comes from either side, each by path.
    files.sort_by_key(|file| {
        let rank = match file.touch {
            Touch::Conflict => 0,
            Touch::Both => 1,
            Touch::B => 2,
            Touch::A => 3,
        };
        (rank, file.path.clone())
    });
    Ok(files)
}

/// `--name-status -z`: status and path, alternating.
fn statuses(out: &str) -> BTreeMap<String, char> {
    let mut map = BTreeMap::new();
    let mut fields = out.split('\0').filter(|field| !field.is_empty());
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        if let Some(letter) = status.chars().next() {
            map.insert(path.to_string(), letter);
        }
    }
    map
}

/// One side's card.
fn side(
    repo: &gix::Repository,
    dir: &Path,
    name: &str,
    tip: &str,
    base: &str,
    worktrees: &BTreeMap<String, String>,
    conflicted: &[String],
) -> Result<Side> {
    let range = format!("{base}..{tip}");
    let newest = commit_lines(&shell::commit_records(dir, &range, NEWEST)?);
    let time = match newest.first() {
        Some(line) if line.id == tip => line.time,
        _ => commit_lines(&shell::commit_records(dir, tip, 1)?)
            .first()
            .map_or(0, |line| line.time),
    };
    let kind = kind_of(repo, name);
    Ok(Side {
        name: name.to_string(),
        kind,
        tip: tip.to_string(),
        short: short(tip),
        ahead: shell::count_commits(dir, &range)?,
        touching: if conflicted.is_empty() {
            0
        } else {
            shell::count_commits_touching(dir, &range, conflicted)?
        },
        newest,
        time,
        checked_out: if kind == RefKind::Local {
            worktrees.get(name).cloned()
        } else {
            None
        },
    })
}

/// `commit_records` output, record by record.
fn commit_lines(out: &str) -> Vec<CommitLine> {
    out.split('\u{1e}')
        .filter_map(|record| {
            let mut parts = record.trim_start_matches('\n').split('\u{1f}');
            let id = parts.next()?.trim();
            if id.is_empty() {
                return None;
            }
            Some(CommitLine {
                id: id.to_string(),
                short: parts.next()?.to_string(),
                summary: parts.next()?.to_string(),
                time: parts.next()?.trim().parse().unwrap_or(0),
            })
        })
        .collect()
}

/// What `name` is: a branch here first, as `git rev-parse` reads it.
pub fn kind_of(repo: &gix::Repository, name: &str) -> RefKind {
    let exists = |full: String| matches!(repo.try_find_reference(full.as_str()), Ok(Some(_)));
    if exists(format!("refs/heads/{name}")) {
        RefKind::Local
    } else if exists(format!("refs/remotes/{name}")) {
        RefKind::Remote
    } else if exists(format!("refs/tags/{name}")) {
        RefKind::Tag
    } else {
        RefKind::Commit
    }
}

/// Every branch that is checked out, and where.
pub(crate) fn checked_out(repo: &gix::Repository) -> BTreeMap<String, String> {
    crate::worktrees::list(repo)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|worktree| Some((worktree.branch?, worktree.path)))
        .collect()
}

fn short(id: &str) -> String {
    id.chars().take(7).collect()
}

/// A worktree of Spagitty's own under the repository's git directory, removed
/// when dropped — whatever happened in it.
pub(crate) struct Scratch {
    repo: PathBuf,
    path: PathBuf,
}

impl Scratch {
    /// A detached worktree at `commit`.
    pub fn add(repo: &gix::Repository, commit: &str) -> Result<Self> {
        let dir = workdir(repo)?.to_path_buf();
        let path = scratch_root(repo).join(unique_name());
        std::fs::create_dir_all(path.parent().expect("scratch has a parent"))?;
        shell::worktree_add(&dir, &path, Some(commit), None, true)?;
        Ok(Scratch { repo: dir, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = shell::worktree_remove(&self.repo, &self.path, true);
        let _ = std::fs::remove_dir_all(&self.path);
        let _ = shell::worktree_prune(&self.repo);
    }
}

/// Where Merger's worktrees live: inside the git directory, where no editor,
/// file watcher or `git status` of the user's looks.
pub(crate) fn scratch_root(repo: &gix::Repository) -> PathBuf {
    repo.common_dir().join("spagitty").join("merger")
}

fn unique_name() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    format!(
        "{}-{}-{nanos}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// ```text
    /// main:  base ─ A1 (notes.txt, shared.txt line 2, a-only.txt)
    /// feat:  base ─ B1 (shared.txt line 2, b-new.txt) ─ B2 (both.txt end)
    /// ```
    ///
    /// `shared.txt` conflicts, `both.txt` is changed on both sides apart and
    /// merges, `a-only.txt` is A's and `b-new.txt` is new on B.
    fn split() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("shared.txt", "one\ntwo\nthree\n");
        fixture.write("both.txt", "1\n2\n3\n4\n5\n6\n7\n8\n");
        fixture.write("a-only.txt", "a\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.write("shared.txt", "one\nTWO from feat\nthree\n");
        fixture.write("b-new.txt", "new\n");
        fixture.write("both.txt", "one\n2\n3\n4\n5\n6\n7\n8\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Feat changes two");
        fixture.write("both.txt", "one\n2\n3\n4\n5\n6\n7\neight\n");
        fixture.commit_all("Feat changes eight");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write("shared.txt", "one\nTWO from main\nthree\n");
        fixture.write("a-only.txt", "a, changed\n");
        fixture.write("both.txt", "1\n2\n3\nfour\n5\n6\n7\n8\n");
        fixture.commit_all("Main changes two");
        fixture
    }

    #[test]
    fn a_version_reads_as_major_and_minor() {
        assert_eq!(
            shell::parse_version("git version 2.55.0.windows.3"),
            Some((2, 55))
        );
        assert_eq!(shell::parse_version("git version 2.34.1"), Some((2, 34)));
        assert_eq!(shell::parse_version("not git"), None);
    }

    #[test]
    fn merge_tree_output_reads_as_a_tree_and_stages() {
        let out = [
            "abc",
            "100644 b1 1\tf.txt",
            "100644 o1 2\tf.txt",
            "100644 t1 3\tf.txt",
            "100644 o2 2\tg.txt",
            "",
            "1",
            "f.txt",
            "Auto-merging",
            "Auto-merging f.txt\n",
            "",
        ]
        .join("\0");
        let out = out.as_str();
        let parsed = parse_merge_tree(out);
        assert_eq!(parsed.tree, "abc");
        assert_eq!(parsed.stages.len(), 2);
        assert_eq!(parsed.stages["f.txt"].len(), 3);
        assert_eq!(parsed.stages["g.txt"][&2], ("100644".into(), "o2".into()));
    }

    #[test]
    fn a_clean_merge_tree_has_no_stages() {
        let parsed = parse_merge_tree("abc\0\x001\0f.txt\0Auto-merging\0Auto-merging f.txt\n\0");
        assert_eq!(parsed.tree, "abc");
        assert!(parsed.stages.is_empty());
    }

    fn check_forecast(method: DryRun) {
        let fixture = split();
        let repo = fixture.open();
        let a = shell::commit_id(fixture.path(), "main").unwrap();
        let b = shell::commit_id(fixture.path(), "feat").unwrap();
        let outcome = dry_run(&repo, &a, &b, method).expect("the dry run");
        assert_eq!(outcome.method, method);
        assert_eq!(outcome.conflicted.len(), 1, "{outcome:?}");
        let shared = &outcome.conflicted[0];
        assert_eq!(shared.path, "shared.txt");
        assert_eq!(shared.kind, ConflictKind::BothModified);
        let text = String::from_utf8(shared.merged.clone().expect("markers")).unwrap();
        let regions = conflicts::regions(&text);
        assert_eq!(regions.len(), 1, "{text}");
        assert_eq!(regions[0].ours, "TWO from main\n");
        assert_eq!(regions[0].theirs, "TWO from feat\n");
        assert_eq!(
            regions[0].base.as_deref(),
            Some("two\n"),
            "diff3 markers carry the base"
        );
    }

    #[test]
    fn merge_tree_finds_the_conflict_and_the_base_in_it() {
        check_forecast(DryRun::MergeTree);
    }

    #[test]
    fn the_worktree_fallback_finds_the_same() {
        check_forecast(DryRun::Worktree);
    }

    #[test]
    fn the_fallback_leaves_no_worktree_behind() {
        let fixture = split();
        let repo = fixture.open();
        let a = shell::commit_id(fixture.path(), "main").unwrap();
        let b = shell::commit_id(fixture.path(), "feat").unwrap();
        dry_run(&repo, &a, &b, DryRun::Worktree).unwrap();

        let listed = fixture.git(&["worktree", "list", "--porcelain"]);
        assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
        let root = scratch_root(&repo);
        let left = std::fs::read_dir(&root).map_or(0, |dir| dir.count());
        assert_eq!(left, 0, "nothing left under {}", root.display());
    }

    #[test]
    fn the_forecast_says_where_they_split_and_what_each_has() {
        let fixture = split();
        let forecast = forecast(&fixture.open(), "main", "feat").expect("forecast");

        assert_eq!(forecast.base, fixture.rev("main~1"));
        assert_eq!(forecast.a.ahead, 1);
        assert_eq!(forecast.b.ahead, 2);
        assert_eq!(forecast.b.newest[0].summary, "Feat changes eight");
        assert_eq!(forecast.a.kind, RefKind::Local);
        assert!(forecast.a.checked_out.is_some(), "main is checked out");
        assert!(forecast.b.checked_out.is_none());
        assert!(!forecast.a_has_b && !forecast.b_has_a);
        assert_eq!(forecast.conflicts, 1);
        assert_eq!(forecast.a.touching, 1);
        assert_eq!(
            forecast.b.touching, 1,
            "only the first of feat's two touches shared.txt"
        );
    }

    #[test]
    fn every_file_is_grouped_by_what_happens_to_it() {
        let fixture = split();
        let forecast = forecast(&fixture.open(), "main", "feat").unwrap();
        let touch: Vec<(&str, Touch)> = forecast
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.touch))
            .collect();
        assert_eq!(
            touch,
            vec![
                ("shared.txt", Touch::Conflict),
                ("both.txt", Touch::Both),
                ("b-new.txt", Touch::B),
                ("a-only.txt", Touch::A),
            ]
        );
        assert!(forecast.files[2].added_by_b);
        assert_eq!(forecast.files[0].conflicts, 1);
    }

    #[test]
    fn a_branch_already_merged_says_so() {
        let fixture = split();
        fixture.git(&["branch", "behind", "main~1"]);
        let forecast = forecast(&fixture.open(), "main", "behind").unwrap();
        assert!(forecast.a_has_b);
        assert_eq!(forecast.b.ahead, 0);
        assert!(forecast.files.iter().all(|file| file.touch == Touch::A));
    }

    #[test]
    fn a_name_that_is_not_a_commit_is_named() {
        let fixture = split();
        let error = forecast(&fixture.open(), "main", "nope").unwrap_err();
        assert!(error.to_string().contains("nope"), "{error}");
    }

    #[test]
    fn a_deleted_file_conflicts_as_a_whole() {
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

        let forecast = forecast(&fixture.open(), "main", "feat").unwrap();
        assert_eq!(forecast.files[0].touch, Touch::Conflict);
        assert_eq!(forecast.files[0].kind, Some(ConflictKind::DeletedByThem));
        assert_eq!(forecast.files[0].conflicts, 1);
    }

    /// What the acceptance criteria call byte-for-byte unchanged: the index
    /// file, every ref, HEAD and every file in the working tree.
    type Snapshot = (Vec<u8>, String, String, Vec<(String, Vec<u8>)>);

    fn snapshot(fixture: &Fixture) -> Snapshot {
        let index = std::fs::read(fixture.at(".git/index")).unwrap();
        let refs = fixture.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
        let head = std::fs::read_to_string(fixture.at(".git/HEAD")).unwrap();
        let mut files = Vec::new();
        for entry in std::fs::read_dir(fixture.path()).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                files.push((
                    entry.file_name().to_string_lossy().into_owned(),
                    std::fs::read(entry.path()).unwrap(),
                ));
            }
        }
        files.sort();
        (index, refs, head, files)
    }

    #[test]
    fn a_forecast_writes_nothing_anybody_reads() {
        for method in [DryRun::MergeTree, DryRun::Worktree] {
            let fixture = split();
            let repo = fixture.open();
            let before = snapshot(&fixture);
            let index_time = std::fs::metadata(fixture.at(".git/index"))
                .unwrap()
                .modified()
                .unwrap();

            let a = shell::commit_id(fixture.path(), "main").unwrap();
            let b = shell::commit_id(fixture.path(), "feat").unwrap();
            dry_run(&repo, &a, &b, method).unwrap();
            forecast(&repo, "main", "feat").unwrap();

            assert_eq!(snapshot(&fixture), before, "{method:?}");
            let after = std::fs::metadata(fixture.at(".git/index"))
                .unwrap()
                .modified()
                .unwrap();
            assert_eq!(after, index_time, "{method:?}: the index was not rewritten");
        }
    }
}

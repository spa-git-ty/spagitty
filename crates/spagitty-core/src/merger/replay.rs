// SPDX-License-Identifier: GPL-3.0-or-later

//! Rebase, then fast-forward, in Merger (FEAT-103): the source's commits
//! replayed one at a time on the receiving tip, stopping wherever one of them
//! conflicts, and the receiving branch moved only once the last has landed.
//!
//! The replay runs in a worktree of Spagitty's own under the git directory,
//! detached at the source's tip, so neither branch moves and nothing the
//! person has checked out is touched while it runs or while it is stopped. The
//! worktree is named after the merge it is for, so leaving the screen and
//! coming back finds the same rebase where it stopped. Abort removes it, and
//! the repository is as it was.
//!
//! A stop is read the way a merge's conflicts are (`detail.rs`): each side
//! whole, the file with diff3 markers, each region located on each side. In a
//! rebase git's *ours* is the receiving side and *theirs* the commit being
//! replayed; they are mapped back to A and B here, so the screen draws A | B
//! the same way whichever branch receives.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::detail::{context_before, locate, FileConflict, RegionSource};
use super::land::{move_to, receiving, LandAsk, Landed, Resolution, Target};
use super::{scratch_root, short, CommitLine};
use crate::conflicts::{self, ConflictSides, Side};
use crate::error::{Error, Result};
use crate::repo::workdir;
use crate::shell;

/// Where a replay stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum Replay {
    /// Stopped on a commit, with what it conflicts over.
    #[serde(rename_all = "camelCase")]
    Stopped {
        /// The commit being replayed, counting from 1, of how many.
        step: usize,
        total: usize,
        commit: Option<CommitLine>,
        files: Vec<FileConflict>,
    },
    /// Every commit replayed; the receiving branch has not moved yet.
    #[serde(rename_all = "camelCase")]
    Done {
        tip: String,
        short: String,
        written: usize,
    },
}

/// The pieces of one replay: who receives, from what, and where it runs.
struct Session {
    dir: PathBuf,
    path: PathBuf,
    onto: String,
    source: String,
    /// The receiving side is B, so git's ours is B and theirs is A.
    swapped: bool,
}

fn session(repo: &gix::Repository, ask: &LandAsk) -> Result<Session> {
    let dir = workdir(repo)?.to_path_buf();
    if shell::commit_id(&dir, &ask.a)? != ask.a_tip {
        return Err(Error::Stale(ask.a.clone()));
    }
    if shell::commit_id(&dir, &ask.b)? != ask.b_tip {
        return Err(Error::Stale(ask.b.clone()));
    }
    let swapped = ask.target == Target::B;
    let (onto, source) = if swapped {
        (ask.b_tip.clone(), ask.a_tip.clone())
    } else {
        (ask.a_tip.clone(), ask.b_tip.clone())
    };
    Ok(Session {
        path: scratch_root(repo).join(format!("rebase-{}", key(ask))),
        dir,
        onto,
        source,
        swapped,
    })
}

/// Sixteen hex digits naming one replay, from everything that decides it.
fn key(ask: &LandAsk) -> String {
    let target = match ask.target {
        Target::A => "a",
        Target::B => "b",
        Target::New => "new",
    };
    let text = [
        ask.a.as_str(),
        &ask.b,
        &ask.a_tip,
        &ask.b_tip,
        target,
        ask.new_name.as_deref().unwrap_or(""),
    ]
    .join("\0");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Start the replay, or find the one already running for this merge.
pub fn rebase_open(repo: &gix::Repository, ask: &LandAsk) -> Result<Replay> {
    let session = session(repo, ask)?;
    receiving(repo, &session.dir, ask)?;
    if !session.path.join(".git").exists() {
        if shell::is_ancestor(&session.dir, &session.source, &session.onto)? {
            return Err(Error::NotStageable("nothing comes in".into()));
        }
        let base = shell::merge_base(&session.dir, &session.onto, &session.source)?
            .ok_or_else(|| Error::NotStageable("the branches share no history".into()))?;
        std::fs::create_dir_all(scratch_root(repo))?;
        shell::worktree_add(
            &session.dir,
            &session.path,
            Some(&session.source),
            None,
            true,
        )?;
        if let Err(error) = shell::rebase_replay(&session.path, &session.onto, &base) {
            remove(&session);
            return Err(error);
        }
    }
    state(&session)
}

/// Settle every conflicted file of the stop with what was chosen, and carry on.
pub fn rebase_continue(
    repo: &gix::Repository,
    ask: &LandAsk,
    resolutions: &[Resolution],
) -> Result<Replay> {
    let session = running(repo, ask)?;
    let stopped = crate::repo::open(&session.path)?;
    let left = conflicts::conflicted(&stopped)?;
    for file in &left {
        let resolution = resolutions
            .iter()
            .find(|r| r.path == file.path)
            .ok_or_else(|| Error::NotStageable(format!("{} is not resolved yet", file.path)))?;
        let side = resolution.take.map(|side| git_side(side, session.swapped));
        conflicts::settle(&stopped, &file.path, resolution.text.as_deref(), side)?;
    }
    // A commit whose changes were all resolved away has nothing left to
    // commit; git's answer to that is to skip it.
    let skip = !shell::has_staged(&session.path)?;
    shell::rebase_step(&session.path, skip)?;
    state(&session)
}

/// Drop the commit the replay stopped on, and carry on.
pub fn rebase_skip(repo: &gix::Repository, ask: &LandAsk) -> Result<Replay> {
    let session = running(repo, ask)?;
    shell::rebase_step(&session.path, true)?;
    state(&session)
}

/// Undo the replay and remove its worktree. Neither branch ever moved.
pub fn rebase_abort(repo: &gix::Repository, ask: &LandAsk) -> Result<()> {
    let session = session(repo, ask)?;
    if session.path.join(".git").exists() {
        let _ = shell::rebase_abort(&session.path);
    }
    remove(&session);
    Ok(())
}

/// Move the receiving branch to the replayed commits, and clear up.
pub fn rebase_finish(repo: &gix::Repository, ask: &LandAsk) -> Result<Landed> {
    let session = running(repo, ask)?;
    let Replay::Done { tip, written, .. } = state(&session)? else {
        return Err(Error::NotStageable("the rebase has not finished".into()));
    };
    let target = receiving(repo, &session.dir, ask)?;
    let reason = format!("merger: rebase into {target}");
    move_to(
        repo,
        &session.dir,
        ask.target,
        &target,
        &tip,
        &session.onto,
        &reason,
    )?;
    remove(&session);
    Ok(Landed {
        short: short(&tip),
        commit: tip,
        target,
        written,
    })
}

fn running(repo: &gix::Repository, ask: &LandAsk) -> Result<Session> {
    let session = session(repo, ask)?;
    if !session.path.join(".git").exists() {
        return Err(Error::NotStageable(
            "there is no rebase running for this merge".into(),
        ));
    }
    Ok(session)
}

fn remove(session: &Session) {
    let _ = shell::worktree_remove(&session.dir, &session.path, true);
    let _ = std::fs::remove_dir_all(&session.path);
    let _ = shell::worktree_prune(&session.dir);
}

/// A for ours when A receives, and the other way round when B does.
fn git_side(side: Side, swapped: bool) -> Side {
    match (side, swapped) {
        (side, false) => side,
        (Side::Ours, true) => Side::Theirs,
        (Side::Theirs, true) => Side::Ours,
    }
}

/// Where the replay in `session` stands now.
fn state(session: &Session) -> Result<Replay> {
    let git_dir = shell::absolute_git_dir(&session.path)?;
    if let Some(progress) = crate::rebase::progress_in(&git_dir) {
        let stopped = crate::repo::open(&session.path)?;
        let commit = shell::commit_id(&session.path, "REBASE_HEAD")
            .ok()
            .and_then(|id| commit_line(&session.path, &id));
        let files = conflicts::conflicted(&stopped)?
            .iter()
            .map(|file| {
                let sides = conflicts::sides(&stopped, &file.path)?;
                Ok(stop_file(sides, session.swapped, commit.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        return Ok(Replay::Stopped {
            step: progress.step,
            total: progress.total,
            commit,
            files,
        });
    }
    let tip = shell::commit_id(&session.path, "HEAD")?;
    if !shell::is_ancestor(&session.path, &session.onto, &tip)? {
        return Err(Error::NotStageable(
            "the rebase did not start; nothing was written".into(),
        ));
    }
    let written = shell::count_commits(&session.path, &format!("{}..{tip}", session.onto))?;
    Ok(Replay::Done {
        short: short(&tip),
        tip,
        written,
    })
}

fn commit_line(dir: &Path, id: &str) -> Option<CommitLine> {
    let out = shell::commit_records(dir, &format!("{id}^!"), 1).ok()?;
    let mut parts = out.trim().trim_end_matches('\u{1e}').split('\u{1f}');
    Some(CommitLine {
        id: parts.next()?.trim().to_string(),
        short: parts.next()?.to_string(),
        summary: parts.next()?.to_string(),
        time: parts.next()?.trim().parse().unwrap_or(0),
    })
}

/// One file of a stop, with ours and theirs put back as A and B.
fn stop_file(sides: ConflictSides, swapped: bool, replayed: Option<CommitLine>) -> FileConflict {
    let (a, b) = if swapped {
        (sides.theirs, sides.ours)
    } else {
        (sides.ours, sides.theirs)
    };
    let mut merged = sides.merged.filter(|side| !side.binary && !side.too_large);
    let mut found = merged
        .as_ref()
        .map(|side| conflicts::regions(&side.text))
        .unwrap_or_default();
    if found.is_empty() {
        merged = None;
    }
    if swapped {
        if let Some(side) = merged.as_mut() {
            side.text = swap_markers(&side.text);
            found = conflicts::regions(&side.text);
        }
    }
    let merged_lines: Vec<&str> = merged
        .as_ref()
        .map_or(Vec::new(), |side| side.text.lines().collect());
    let a_lines: Vec<&str> = a
        .as_ref()
        .map_or(Vec::new(), |side| side.text.lines().collect());
    let b_lines: Vec<&str> = b
        .as_ref()
        .map_or(Vec::new(), |side| side.text.lines().collect());
    let (mut a_from, mut b_from) = (0, 0);
    let regions = found
        .iter()
        .map(|region| {
            let before = context_before(&merged_lines, &found, region.index);
            let ours: Vec<&str> = region.ours.lines().collect();
            let theirs: Vec<&str> = region.theirs.lines().collect();
            let a_line = locate(&a_lines, &before, &ours, &mut a_from).filter(|_| !ours.is_empty());
            let b_line =
                locate(&b_lines, &before, &theirs, &mut b_from).filter(|_| !theirs.is_empty());
            RegionSource {
                index: region.index,
                a_line,
                b_line,
                a_commit: if swapped { replayed.clone() } else { None },
                b_commit: if swapped { None } else { replayed.clone() },
            }
        })
        .collect();
    FileConflict {
        path: sides.path,
        kind: sides.kind,
        base: sides.base,
        a,
        b,
        merged,
        regions,
    }
}

/// The same markers with their two sides exchanged, so the first is A's.
fn swap_markers(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if !lines[i].starts_with("<<<<<<<") {
            out.push(lines[i].to_string());
            i += 1;
            continue;
        }
        let open = lines[i];
        let (mut ours, mut base, mut theirs) = (Vec::new(), None::<Vec<&str>>, Vec::new());
        let (mut base_marker, mut split, mut close) = (None, None, None);
        let mut phase = 0;
        let mut j = i + 1;
        while j < lines.len() {
            let line = lines[j];
            if line.starts_with("|||||||") && phase == 0 {
                phase = 1;
                base = Some(Vec::new());
                base_marker = Some(line);
            } else if line.starts_with("=======") && phase < 2 {
                phase = 2;
                split = Some(line);
            } else if line.starts_with(">>>>>>>") && phase == 2 {
                close = Some(line);
                break;
            } else if phase == 0 {
                ours.push(line);
            } else if phase == 1 {
                base.as_mut().expect("base started").push(line);
            } else {
                theirs.push(line);
            }
            j += 1;
        }
        let (Some(split), Some(close)) = (split, close) else {
            out.extend(lines[i..].iter().map(|line| line.to_string()));
            break;
        };
        out.push(format!("<<<<<<<{}", &close[7..]));
        out.extend(theirs.iter().map(|line| line.to_string()));
        if let (Some(marker), Some(base)) = (base_marker, base) {
            out.push(marker.to_string());
            out.extend(base.iter().map(|line| line.to_string()));
        }
        out.push(split.to_string());
        out.extend(ours.iter().map(|line| line.to_string()));
        out.push(format!(">>>>>>>{}", &open[7..]));
        i = j + 1;
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::land::Strategy;
    use super::*;
    use crate::fixture::Fixture;

    /// feat's first commit conflicts with main; its second does not.
    ///
    /// ```text
    /// main: Base ─ M1 (shared.txt: TWO main)
    /// feat: Base ─ F1 (shared.txt: TWO feat) ─ F2 (feat.txt)
    /// ```
    fn stopping() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("shared.txt", "one\ntwo\nthree\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.write("shared.txt", "one\nTWO feat\nthree\n");
        fixture.commit_all("F1");
        fixture.write("feat.txt", "feat\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("F2");
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("shared.txt", "one\nTWO main\nthree\n");
        fixture.commit_all("M1");
        fixture
    }

    fn ask(fixture: &Fixture, target: Target) -> LandAsk {
        LandAsk {
            a: "main".into(),
            b: "feat".into(),
            a_tip: fixture.rev("main"),
            b_tip: fixture.rev("feat"),
            target,
            new_name: None,
            strategy: Strategy::Rebase,
            message: None,
            resolutions: Vec::new(),
        }
    }

    fn worktrees(fixture: &Fixture) -> usize {
        fixture
            .git(&["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count()
    }

    #[test]
    fn a_replay_stops_on_the_commit_that_conflicts_and_nothing_moves() {
        let fixture = stopping();
        let (main, feat) = (fixture.rev("main"), fixture.rev("feat"));
        let index = std::fs::read(fixture.at(".git/index")).unwrap();

        let replay = rebase_open(&fixture.open(), &ask(&fixture, Target::A)).unwrap();

        let Replay::Stopped {
            step,
            total,
            commit,
            files,
        } = replay
        else {
            panic!("it was meant to stop: {replay:?}");
        };
        assert_eq!((step, total), (1, 2));
        assert_eq!(commit.unwrap().summary, "F1");
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(
            file.a.as_ref().unwrap().text,
            "one\nTWO main\nthree\n",
            "A is main"
        );
        assert_eq!(
            file.b.as_ref().unwrap().text,
            "one\nTWO feat\nthree\n",
            "B is feat"
        );
        assert!(
            file.merged.as_ref().unwrap().text.contains("|||||||"),
            "diff3"
        );
        assert_eq!(file.regions[0].b_commit.as_ref().unwrap().summary, "F1");
        assert_eq!((fixture.rev("main"), fixture.rev("feat")), (main, feat));
        assert_eq!(std::fs::read(fixture.at(".git/index")).unwrap(), index);
        assert_eq!(fixture.git(&["branch", "--show-current"]).trim(), "main");
    }

    #[test]
    fn coming_back_finds_the_same_stop() {
        let fixture = stopping();
        let ask = ask(&fixture, Target::A);
        rebase_open(&fixture.open(), &ask).unwrap();
        let again = rebase_open(&fixture.open(), &ask).unwrap();
        assert!(matches!(again, Replay::Stopped { step: 1, .. }));
        assert_eq!(worktrees(&fixture), 2);
    }

    #[test]
    fn resolving_each_stop_then_finishing_moves_the_target_and_clears_up() {
        let fixture = stopping();
        let ask = ask(&fixture, Target::A);
        let main = fixture.rev("main");
        let feat = fixture.rev("feat");
        rebase_open(&fixture.open(), &ask).unwrap();

        let resolved = [Resolution {
            path: "shared.txt".into(),
            text: Some("one\nTWO both\nthree\n".into()),
            take: None,
        }];
        let replay = rebase_continue(&fixture.open(), &ask, &resolved).unwrap();
        assert!(
            matches!(replay, Replay::Done { written: 2, .. }),
            "{replay:?}"
        );
        assert_eq!(fixture.rev("main"), main, "nothing moved before finishing");

        let landed = rebase_finish(&fixture.open(), &ask).unwrap();

        assert_eq!(fixture.rev("main"), landed.commit);
        assert_eq!(fixture.rev("main~2"), main);
        assert_eq!(fixture.rev("feat"), feat, "the source is never moved");
        assert_eq!(fixture.read("shared.txt"), "one\nTWO both\nthree\n");
        assert_eq!(fixture.read("feat.txt"), "feat\n");
        assert_eq!(
            fixture.git(&["log", "-1", "--format=%s", "main~1"]).trim(),
            "F1",
            "each commit keeps its own message"
        );
        assert_eq!(worktrees(&fixture), 1);
    }

    #[test]
    fn into_b_the_columns_still_read_a_then_b() {
        let fixture = stopping();
        fixture.git(&["switch", "-q", "feat"]);
        let ask = ask(&fixture, Target::B);

        let Replay::Stopped { files, .. } = rebase_open(&fixture.open(), &ask).unwrap() else {
            panic!("it was meant to stop");
        };

        let file = &files[0];
        assert_eq!(file.a.as_ref().unwrap().text, "one\nTWO main\nthree\n");
        assert_eq!(file.b.as_ref().unwrap().text, "one\nTWO feat\nthree\n");
        let regions = conflicts::regions(&file.merged.as_ref().unwrap().text);
        assert_eq!(regions[0].ours, "TWO main\n", "A's lines come first");
        assert_eq!(regions[0].theirs, "TWO feat\n");

        let taken = [Resolution {
            path: "shared.txt".into(),
            text: None,
            take: Some(Side::Ours),
        }];
        let done = rebase_continue(&fixture.open(), &ask, &taken).unwrap();
        assert!(matches!(done, Replay::Done { .. }), "{done:?}");
        rebase_finish(&fixture.open(), &ask).unwrap();
        assert_eq!(
            fixture.git(&["show", "feat:shared.txt"]),
            "one\nTWO main\nthree\n",
            "taking A took main's lines"
        );
    }

    #[test]
    fn skipping_drops_the_commit_it_stopped_on() {
        let fixture = stopping();
        let ask = ask(&fixture, Target::A);
        rebase_open(&fixture.open(), &ask).unwrap();

        let done = rebase_skip(&fixture.open(), &ask).unwrap();

        assert!(matches!(done, Replay::Done { written: 1, .. }), "{done:?}");
    }

    #[test]
    fn aborting_leaves_the_repository_as_it_was() {
        let fixture = stopping();
        let ask = ask(&fixture, Target::A);
        let refs = fixture.git(&["for-each-ref"]);
        rebase_open(&fixture.open(), &ask).unwrap();

        rebase_abort(&fixture.open(), &ask).unwrap();

        assert_eq!(fixture.git(&["for-each-ref"]), refs);
        assert_eq!(worktrees(&fixture), 1);
        assert!(fixture.git(&["status", "--porcelain"]).is_empty());
        assert!(rebase_finish(&fixture.open(), &ask).is_err());
    }

    #[test]
    fn a_replay_with_nothing_in_its_way_is_done_at_once() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "a\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");
        fixture.git(&["switch", "-q", "-c", "feat"]);
        fixture.write("b.txt", "b\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("F1");
        fixture.git(&["switch", "-q", "main"]);
        fixture.write("c.txt", "c\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("M1");

        let replay = rebase_open(&fixture.open(), &ask(&fixture, Target::A)).unwrap();

        assert!(
            matches!(replay, Replay::Done { written: 1, .. }),
            "{replay:?}"
        );
    }

    #[test]
    fn markers_swap_sides_with_their_base() {
        let text = "x\n<<<<<<< HEAD\nours\n||||||| base\nold\n=======\ntheirs\n>>>>>>> pick\ny\n";
        assert_eq!(
            swap_markers(text),
            "x\n<<<<<<< pick\ntheirs\n||||||| base\nold\n=======\nours\n>>>>>>> HEAD\ny\n"
        );
    }
}

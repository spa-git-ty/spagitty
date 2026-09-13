// SPDX-License-Identifier: GPL-3.0-or-later

//! History walking and lane assignment.
//!
//! # Lanes
//!
//! A lane is a vertical column in the graph. Lane assignment happens
//! *incrementally* as the walk produces commits — there is no second pass over
//! the history, which is what lets the graph paint while it is still loading.
//!
//! [`LaneState`] holds one slot per lane, and each slot holds the commit that
//! lane is currently waiting for. Walking a commit means: take the lane that was
//! waiting for you, hand it to your first parent, and give every other parent a
//! lane of its own branching out of yours. Lanes that were waiting for the same
//! commit converge on it and are released.
//!
//! # Edges
//!
//! Rows are self-contained: a row carries the lane segments for the band
//! **above** it, from the previous row's center down to its own. That band is
//! exactly [`ROW_PITCH`] tall, so an edge only needs lane indices — the y
//! coordinates fall out of the row index on the drawing side.
//!
//! A segment with `from == to` is a straight vertical. Otherwise it is a cubic
//! elbow spanning that one row: a branch-out leaves its node and arrives in its
//! new lane one row down, then runs straight; a merge runs straight down its own
//! lane and bends into the node in the last row before it.

use std::collections::{HashMap, HashSet};

use gix::ObjectId;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::refs::{RefChip, RefIndex};

/// Height of one commit row in CSS pixels.
///
/// This mirrors `ROW_PITCH` in `src/lib/metrics.ts`, which is the source of
/// truth for the frontend. It exists here because lane geometry is described in
/// row units, and the two must not drift; `row_pitch_matches_the_frontend`
/// reads that file and fails if one side is changed without the other.
pub const ROW_PITCH: u32 = 30;

/// Lane colors cycle through this many values. A lane keeps the color it was
/// allocated for its whole lifetime, so a long-lived branch keeps one color.
pub const LANE_COLOR_COUNT: usize = 5;

/// How many rows are emitted per event. Small enough that the first paint is
/// immediate, large enough that a long walk isn't dominated by IPC overhead.
pub const BATCH: usize = 256;

/// One lane segment in the band above a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneEdge {
    /// Lane index at the top of the band.
    pub from: usize,
    /// Lane index at the bottom of the band.
    pub to: usize,
    /// Index into the lane color cycle.
    pub color: usize,
}

/// One commit row: everything needed to paint it, with no global state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRow {
    /// Absolute index in the walk. The row sits at `y = index * ROW_PITCH`.
    pub index: usize,
    pub id: String,
    pub short: String,
    pub summary: String,
    pub author_name: String,
    /// The author's email, lower-cased.
    ///
    /// Carried because it is what identifies a person across the several names
    /// one human commits under, and the graph draws a portrait generated from
    /// it. Empty when the signature has none, which git allows; the frontend
    /// falls back to the name.
    pub author_email: String,
    /// Up to two uppercase letters. The fallback where a portrait cannot be
    /// drawn, and what the Author column shows beside a message.
    pub initials: String,
    /// Author time, seconds since the unix epoch.
    pub time: i64,
    pub lane: usize,
    pub color: usize,
    /// The commit carries a signature (FEAT-019).
    ///
    /// Read off the object's `gpgsig` header as the walk passes it, which costs
    /// nothing — the commit is already decoded for its author and summary. It
    /// says the commit *was signed*, not that the signature is valid: verifying
    /// means a subprocess and a keyring per row, and the screens are careful to
    /// say "signed" rather than "verified".
    pub signed: bool,
    pub parents: Vec<String>,
    pub refs: Vec<RefChip>,
    pub edges: Vec<LaneEdge>,
}

/// What [`LaneState::step`] worked out for one commit.
#[derive(Debug, Clone)]
pub struct RowLanes {
    pub lane: usize,
    pub color: usize,
    pub edges: Vec<LaneEdge>,
}

/// Incremental lane assignment. Feed it commits in walk order.
#[derive(Debug, Default)]
pub struct LaneState {
    /// Lane -> the commit that lane is waiting for.
    active: Vec<Option<ObjectId>>,
    /// Lane -> color index, fixed when the lane is allocated.
    colors: Vec<usize>,
    /// Lane -> the lane it branched out of, for the band above the *next* row.
    /// Cleared every step.
    origin: Vec<Option<usize>>,
    /// Merge edges that route into a lane which already exists, to be drawn in
    /// the band above the *next* row alongside that lane's own segment.
    joins: Vec<LaneEdge>,
    /// Commits that have already been visited in the walk.
    visited: HashSet<ObjectId>,
    next_color: usize,
    row: usize,
}

impl LaneState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lanes currently in use. Useful for tests and for sizing the lane column.
    pub fn width(&self) -> usize {
        self.active.iter().filter(|s| s.is_some()).count()
    }

    /// Hold a lane open for `id` before the walk starts.
    ///
    /// This is what "pin to left" is. Lanes are otherwise handed out in the
    /// order commits are reached, so a long-lived branch like `main` drifts
    /// across the graph as other branches come and go. Reserving its tip's lane
    /// up front means the lane is occupied from row zero and every other branch
    /// allocates to the right of it — the branch stays where the eye left it.
    ///
    /// Reserve in the order the lanes should appear. Reserving a commit that
    /// the walk never reaches simply leaves that lane empty, which is a column
    /// of whitespace rather than a broken graph.
    pub fn reserve(&mut self, id: ObjectId) {
        self.active.push(Some(id));
        self.colors.push(self.next_color % LANE_COLOR_COUNT);
        self.origin.push(None);
        self.next_color = self.next_color.wrapping_add(1);
    }

    /// Take the lowest free lane, or add one. The new lane gets the next color
    /// in the cycle.
    fn alloc(&mut self) -> usize {
        let lane = match self.active.iter().position(Option::is_none) {
            Some(lane) => lane,
            None => {
                self.active.push(None);
                self.colors.push(0);
                self.origin.push(None);
                self.active.len() - 1
            }
        };
        self.colors[lane] = self.next_color % LANE_COLOR_COUNT;
        self.next_color = self.next_color.wrapping_add(1);
        self.origin[lane] = None;
        lane
    }

    /// Advance by one commit.
    pub fn step(&mut self, id: ObjectId, parents: &[ObjectId]) -> RowLanes {
        self.visited.insert(id);

        // Which lanes were waiting for this commit? The lowest one becomes the
        // node's lane; any others are duplicate edges that converge here.
        let mut mine: Option<usize> = None;
        let mut converging: Vec<usize> = Vec::new();
        for (lane, slot) in self.active.iter().enumerate() {
            if *slot == Some(id) {
                match mine {
                    None => mine = Some(lane),
                    Some(_) => converging.push(lane),
                }
            }
        }

        // No lane was waiting: this commit is a tip nothing has descended from
        // yet, so it starts a new lane and has nothing drawn above it.
        let is_tip = mine.is_none();
        let lane = match mine {
            Some(lane) => lane,
            None => {
                let lane = self.alloc();
                self.active[lane] = Some(id);
                lane
            }
        };
        let color = self.colors[lane];

        // The band above this row: one segment per lane that was active there.
        let mut edges = Vec::new();
        if self.row > 0 {
            for l in 0..self.active.len() {
                if self.active[l].is_none() {
                    continue;
                }
                if is_tip && l == lane {
                    // Allocated a moment ago; there is nothing above it to draw.
                    continue;
                }
                let from = self.origin[l].unwrap_or(l);
                let to = if self.active[l] == Some(id) { lane } else { l };
                edges.push(LaneEdge {
                    from,
                    to,
                    color: self.colors[l],
                });
            }
        }
        // Merge edges recorded by the previous row, which join a lane that
        // already existed rather than opening one of their own. If that lane is
        // the one arriving at this commit, the join lands on the node too.
        for join in std::mem::take(&mut self.joins) {
            let to = if self.active[join.to] == Some(id) {
                lane
            } else {
                join.to
            };
            edges.push(LaneEdge { to, ..join });
        }

        for slot in self.origin.iter_mut() {
            *slot = None;
        }

        // Converging lanes have arrived; release them.
        for l in converging {
            self.active[l] = None;
        }

        // The first parent inherits this lane and its color, which is what keeps
        // a long-lived branch on one color for its whole length.
        //
        // Every other parent either joins a lane that is already waiting for it,
        // or opens a new one. Reusing the existing lane is what keeps the graph
        // narrow: giving each merge edge a lane of its own is correct but makes
        // width grow with the number of *edges* in flight rather than the number
        // of distinct commits awaited. On a merge-heavy history that is the
        // difference between a graph 60 lanes wide and one 400 lanes wide.
        match parents.split_first() {
            None => self.active[lane] = None,
            Some((first, rest)) => {
                if self.visited.contains(first) {
                    self.active[lane] = None;
                } else {
                    self.active[lane] = Some(*first);
                }

                for parent in rest {
                    if self.visited.contains(parent) {
                        continue;
                    }
                    match self.active.iter().position(|slot| *slot == Some(*parent)) {
                        // Already awaited: draw an edge into that lane and let
                        // the two lines converge there.
                        Some(existing) => self.joins.push(LaneEdge {
                            from: lane,
                            to: existing,
                            color: self.colors[existing],
                        }),
                        None => {
                            let branched = self.alloc();
                            self.active[branched] = Some(*parent);
                            self.origin[branched] = Some(lane);
                        }
                    }
                }
            }
        }

        self.row += 1;
        RowLanes { lane, color, edges }
    }
}

/// What the sink wants the walk to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

/// How the walk orders commits, which is what makes two drawings of the same
/// DAG look like different trees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphOrder {
    /// Newest first by commit timestamp, like `git log`.
    ///
    /// Parallel histories interleave: a commit on `main` can sit between two
    /// commits of a feature branch if the clocks say so. Lanes are packed into
    /// the lowest free column.
    #[default]
    Date,
    /// `git log --topo-order`.
    ///
    /// No parent is shown before its children, and commits on one line of
    /// history stay together rather than mixing with another. That is the
    /// drawing GitKraken uses for the same DAG.
    Branch,
}

/// Walk history from `tips`, newest first, assigning lanes as we go.
///
/// `sink` is called once per commit and decides whether to keep going. It is
/// also the backpressure valve: a sink that has delivered everything the UI
/// asked for can block inside the call until more is requested, which is how
/// the windowing in `src-tauri` works. Nothing here ever walks to the end of
/// history before producing its first row.
pub fn walk<F>(
    repo: &gix::Repository,
    tips: Vec<ObjectId>,
    refs: &RefIndex,
    sink: F,
) -> Result<usize>
where
    F: FnMut(GraphRow) -> Flow,
{
    walk_pinned(repo, tips, refs, &[], GraphOrder::Date, sink)
}

/// [`walk`], with lanes held open on the left for `pinned`.
///
/// `pinned` is the graph's "pin to left": each id gets a lane reserved before
/// the first row, in the order given, so those branches occupy the leftmost
/// columns for the whole walk instead of drifting as history interleaves. An id
/// the walk never reaches costs one empty column and nothing else.
///
/// `order` is how the same DAG is sequenced: date order interleaves parallel
/// histories; branch order keeps a line of work together.
pub fn walk_pinned<F>(
    repo: &gix::Repository,
    tips: Vec<ObjectId>,
    refs: &RefIndex,
    pinned: &[ObjectId],
    order: GraphOrder,
    mut sink: F,
) -> Result<usize>
where
    F: FnMut(GraphRow) -> Flow,
{
    if tips.is_empty() {
        return Err(Error::EmptyRepository);
    }

    let mut lanes = LaneState::new();
    for id in pinned {
        lanes.reserve(*id);
    }
    let mut index = 0usize;
    let mut initials_cache: HashMap<String, String> = HashMap::new();

    match order {
        GraphOrder::Date => {
            // Newest first, by commit time. This is what `git log` shows by
            // default and what the graph's row order means: down the screen is
            // back in time.
            let walk = repo
                .rev_walk(tips)
                .sorting(gix::revision::walk::Sorting::ByCommitTime(
                    gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
                ))
                .all()
                .map_err(|e| Error::Walk(e.to_string()))?;

            for info in walk {
                let info = info.map_err(|e| Error::Walk(e.to_string()))?;
                let parents: Vec<ObjectId> = info.parent_ids.iter().copied().collect();
                if emit_row(
                    repo,
                    refs,
                    &mut lanes,
                    &mut initials_cache,
                    &mut index,
                    info.id,
                    &parents,
                    info.commit_time.unwrap_or(0),
                    &mut sink,
                )? == Flow::Stop
                {
                    break;
                }
            }
        }
        GraphOrder::Branch => {
            // Building the walker explores enough of the DAG to know indegrees,
            // which is the cost of not interleaving lines of history. Rows
            // still stream after that, so a large history does not have to sit
            // in memory before the first paint.
            let walk = gix::traverse::commit::topo::Builder::from_iters(
                repo.objects.clone(),
                tips,
                None::<std::iter::Empty<ObjectId>>,
            )
            .sorting(gix::traverse::commit::topo::Sorting::TopoOrder)
            .build()
            .map_err(|e| Error::Walk(e.to_string()))?;

            for info in walk {
                let info = info.map_err(|e| Error::Walk(e.to_string()))?;
                let parents: Vec<ObjectId> = info.parent_ids.iter().copied().collect();
                if emit_row(
                    repo,
                    refs,
                    &mut lanes,
                    &mut initials_cache,
                    &mut index,
                    info.id,
                    &parents,
                    info.commit_time.unwrap_or(0),
                    &mut sink,
                )? == Flow::Stop
                {
                    break;
                }
            }
        }
    }

    Ok(index)
}

/// One commit of the walk, painted into a row and handed to the sink.
///
/// The arguments are the walk's own locals. A context struct would exist only
/// to keep this helper under clippy's argument cap.
#[allow(clippy::too_many_arguments)]
fn emit_row<F>(
    repo: &gix::Repository,
    refs: &RefIndex,
    lanes: &mut LaneState,
    initials_cache: &mut HashMap<String, String>,
    index: &mut usize,
    id: ObjectId,
    parents: &[ObjectId],
    fallback_time: i64,
    sink: &mut F,
) -> Result<Flow>
where
    F: FnMut(GraphRow) -> Flow,
{
    let commit = repo
        .find_commit(id)
        .map_err(|e| Error::Walk(e.to_string()))?;

    // An unparseable signature is not worth dropping a commit over; fall
    // back to the walk's own commit time, which is already known.
    let (author_name, author_email, time) = match commit.author() {
        Ok(sig) => (
            sig.name.to_string(),
            sig.email.to_string().to_lowercase(),
            sig.time().map(|t| t.seconds).unwrap_or(fallback_time),
        ),
        Err(_) => (String::new(), String::new(), fallback_time),
    };

    let summary = commit
        .message()
        .map(|m| m.summary().to_string())
        .unwrap_or_default();

    let initials = initials_cache
        .entry(author_name.clone())
        .or_insert_with(|| initials(&author_name))
        .clone();

    let RowLanes { lane, color, edges } = lanes.step(id, parents);

    let row = GraphRow {
        index: *index,
        id: id.to_string(),
        short: short_id(&id),
        summary,
        author_name,
        author_email,
        initials,
        time,
        lane,
        color,
        signed: crate::signing::signed(&commit),
        parents: parents.iter().map(ObjectId::to_string).collect(),
        refs: refs.chips_for(&id),
        edges,
    };

    *index += 1;
    Ok(sink(row))
}

/// The tips to walk from: every local and remote branch, plus HEAD.
///
/// This is the graph's "all branches" mode, which is the default in the
/// handoff. Walking from HEAD alone would hide every branch not merged into it.
pub fn all_tips(repo: &gix::Repository) -> Result<Vec<ObjectId>> {
    let mut tips = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Ok(head) = repo.head_id() {
        let id = head.detach();
        if seen.insert(id) {
            tips.push(id);
        }
    }

    let platform = repo.references().map_err(|e| Error::Refs(e.to_string()))?;
    let iter = platform
        .prefixed("refs/heads/")
        .map_err(|e| Error::Refs(e.to_string()))?;
    for reference in iter.filter_map(std::result::Result::ok) {
        if let Some(id) = reference.try_id().map(|id| id.detach()) {
            if seen.insert(id) {
                tips.push(id);
            }
        }
    }

    let iter = platform
        .prefixed("refs/remotes/")
        .map_err(|e| Error::Refs(e.to_string()))?;
    for reference in iter.filter_map(std::result::Result::ok) {
        if let Some(id) = reference.try_id().map(|id| id.detach()) {
            if seen.insert(id) {
                tips.push(id);
            }
        }
    }

    Ok(tips)
}

/// The tips to walk from, given the refs the graph is currently showing.
///
/// An empty list means "all branches", which is [`all_tips`] and the default.
/// A non-empty list is the graph's Hide, Solo and Smart Branch Visibility
/// controls: all three reduce to the same question — *which refs are the walk
/// rooted at* — so there is one function rather than three modes.
///
/// A name that no longer resolves is skipped rather than failing the walk: a
/// branch can be deleted between the screen listing it and the walk starting,
/// and losing one lane is better than losing the graph. If **nothing** in the
/// list resolves, the caller gets every tip instead of an empty screen, because
/// an empty graph is indistinguishable from a broken one.
pub fn tips_for(repo: &gix::Repository, names: &[String]) -> Result<Vec<ObjectId>> {
    if names.is_empty() {
        return all_tips(repo);
    }

    let mut tips = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for name in names {
        let Ok(id) = repo.rev_parse_single(name.as_str()) else {
            continue;
        };
        let id = id.detach();
        if seen.insert(id) {
            tips.push(id);
        }
    }

    if tips.is_empty() {
        return all_tips(repo);
    }
    Ok(tips)
}

/// Resolve ref names to the commits they point at, skipping what no longer exists.
///
/// Used for the pinned branches: unlike [`tips_for`] an empty result is
/// perfectly fine here — nothing pinned means nothing reserved.
pub fn ids_for(repo: &gix::Repository, names: &[String]) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in names {
        let Ok(id) = repo.rev_parse_single(name.as_str()) else {
            continue;
        };
        let id = id.detach();
        if seen.insert(id) {
            ids.push(id);
        }
    }
    ids
}

/// Seven hex characters, matching what git shows by default.
pub fn short_id(id: &ObjectId) -> String {
    let hex = id.to_string();
    hex[..hex.len().min(7)].to_string()
}

/// Up to two uppercase letters for the node glyph.
///
/// "Ada Lovelace" -> AL, "torvalds" -> TO, "" -> ?.
pub(crate) fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let letters: String = match words.len() {
        0 => return "?".to_string(),
        1 => words[0]
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(2)
            .collect(),
        _ => words
            .iter()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
            .take(2)
            .collect(),
    };
    if letters.is_empty() {
        "?".to_string()
    } else {
        letters.to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> ObjectId {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        ObjectId::from_bytes_or_panic(&bytes)
    }

    /// The doc comment on [`ROW_PITCH`] has claimed since FEAT-001 that the Rust
    /// mirror is asserted against the frontend's value. It never was, and the
    /// two drifted apart in FEAT-029 until both were fixed by hand. This is that
    /// assertion: it reads the declaration out of the TypeScript source, so
    /// changing one side alone fails the workspace test run rather than shipping
    /// lane elbows drawn to a pitch the rows are not laid out on.
    #[test]
    fn row_pitch_matches_the_frontend() {
        let metrics = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/lib/metrics.ts")
            .canonicalize()
            .expect("src/lib/metrics.ts is part of the repository");
        let source = std::fs::read_to_string(&metrics).expect("metrics.ts is readable");

        let declared = source
            .lines()
            .find_map(|line| line.trim().strip_prefix("export const ROW_PITCH = "))
            .and_then(|rest| rest.trim_end_matches(';').trim().parse::<u32>().ok())
            .expect("metrics.ts declares ROW_PITCH as a whole number of pixels");

        assert_eq!(
            declared, ROW_PITCH,
            "src/lib/metrics.ts says the row pitch is {declared}px, this crate says {ROW_PITCH}px"
        );
    }

    /// Straight history: everything stays in lane 0 with one color.
    #[test]
    fn linear_history_uses_one_lane() {
        let mut lanes = LaneState::new();
        let a = lanes.step(oid(1), &[oid(2)]);
        let b = lanes.step(oid(2), &[oid(3)]);
        let c = lanes.step(oid(3), &[]);

        assert_eq!((a.lane, b.lane, c.lane), (0, 0, 0));
        assert_eq!((a.color, b.color, c.color), (0, 0, 0));
        assert!(a.edges.is_empty(), "nothing is drawn above the first row");
        assert_eq!(
            b.edges,
            vec![LaneEdge {
                from: 0,
                to: 0,
                color: 0
            }]
        );
        assert_eq!(lanes.width(), 0, "a root releases its lane");
    }

    /// A merge branches out, runs in its own lane, and converges again.
    #[test]
    fn merge_branches_out_and_converges() {
        let (merge, main, side, base) = (oid(1), oid(2), oid(3), oid(4));
        let mut lanes = LaneState::new();

        let m = lanes.step(merge, &[main, side]);
        assert_eq!(m.lane, 0);
        assert_eq!(lanes.width(), 2, "both parents are now expected");

        // The second parent's lane arrives one row down, elbowing out of lane 0.
        let r1 = lanes.step(main, &[base]);
        assert_eq!(r1.lane, 0);
        assert!(
            r1.edges.contains(&LaneEdge {
                from: 0,
                to: 1,
                color: 1
            }),
            "the branch-out elbow spans exactly the row below its node: {:?}",
            r1.edges
        );

        let r2 = lanes.step(side, &[base]);
        assert_eq!(
            r2.lane, 1,
            "the side commit sits in the lane reserved for it"
        );
        assert_eq!(r2.color, 1, "and keeps that lane's color");

        // Both lanes now wait for `base`; it converges them.
        let r3 = lanes.step(base, &[]);
        assert_eq!(r3.lane, 0, "convergence lands in the lowest waiting lane");
        assert!(
            r3.edges.contains(&LaneEdge {
                from: 1,
                to: 0,
                color: 1
            }),
            "the merge-in elbow keeps the incoming lane's color: {:?}",
            r3.edges
        );
        assert_eq!(lanes.width(), 0);
    }

    /// A freed lane is reused rather than the graph growing without bound.
    #[test]
    fn lanes_are_reused() {
        let mut lanes = LaneState::new();
        lanes.step(oid(1), &[oid(2), oid(3)]);
        lanes.step(oid(2), &[]); // releases lane 0
        let r = lanes.step(oid(3), &[oid(4), oid(5)]);
        assert_eq!(r.lane, 1);
        assert_eq!(lanes.width(), 2, "lane 0 was free and got picked up again");
    }

    /// A merge whose second parent is already awaited must not open a lane for
    /// it. This is what keeps width proportional to distinct commits in flight
    /// rather than to merge edges, and it is the difference between a readable
    /// graph and an unreadable one on a merge-heavy history.
    #[test]
    fn merge_edges_reuse_an_awaited_lane() {
        let (a, b, c, d) = (oid(1), oid(2), oid(3), oid(4));
        let mut lanes = LaneState::new();

        // `a` is a merge: lane 0 continues to `b`, lane 1 opens for `c`.
        lanes.step(a, &[b, c]);
        assert_eq!(lanes.width(), 2);

        // `b` is also a merge, and its second parent `c` is the commit lane 1
        // is already waiting for. Opening a third lane here is what used to
        // make merge-heavy histories hundreds of lanes wide.
        lanes.step(b, &[d, c]);
        assert_eq!(
            lanes.width(),
            2,
            "no lane opened for an already-awaited commit"
        );
    }

    /// Reusing a lane must not mean losing the edge: a merge into an existing
    /// lane still draws a line leaving its node.
    #[test]
    fn a_joining_merge_edge_is_drawn() {
        let (a, b, c, d) = (oid(1), oid(2), oid(3), oid(4));
        let mut lanes = LaneState::new();

        lanes.step(a, &[b, c]);
        lanes.step(b, &[d, c]); // second parent joins lane 1

        // The join is drawn in the band above the row it arrives at, keeping
        // the colour of the lane it merges into.
        let row = lanes.step(c, &[]);
        assert!(
            row.edges.contains(&LaneEdge {
                from: 0,
                to: 1,
                color: 1
            }),
            "the join edge must reach lane 1: {:?}",
            row.edges
        );
    }

    #[test]
    fn colors_cycle_and_stick_to_a_lane() {
        let mut lanes = LaneState::new();
        // One commit with six parents allocates six lanes.
        let parents: Vec<ObjectId> = (10..16).map(oid).collect();
        lanes.step(oid(1), &parents);

        let first = lanes.step(parents[0], &[]);
        assert_eq!(first.color, 0);
        // The sixth lane wraps around to the first color.
        let sixth = lanes.step(parents[5], &[]);
        assert_eq!(sixth.color, 5 % LANE_COLOR_COUNT);
    }

    #[test]
    fn initials_are_two_letters() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("torvalds"), "TO");
        assert_eq!(initials("Jean-Luc Picard Jr"), "JP");
        assert_eq!(initials(""), "?");
        assert_eq!(initials("   "), "?");
    }
}

#[cfg(test)]
mod walk_tests {
    use super::*;
    use crate::fixture::Fixture;
    use crate::refs::RefIndex;

    /// Walk the whole fixture and collect the rows.
    fn rows(fixture: &Fixture) -> Vec<GraphRow> {
        let repo = fixture.open();
        let refs = RefIndex::build(&repo).expect("index");
        let tips = all_tips(&repo).expect("tips");

        let mut out = Vec::new();
        let total = walk(&repo, tips, &refs, |row| {
            out.push(row);
            Flow::Continue
        })
        .expect("walk");

        assert_eq!(
            total,
            out.len(),
            "the walk's return value counts what it delivered"
        );
        out
    }

    #[test]
    fn every_commit_is_delivered_once_in_index_order() {
        let fixture = Fixture::woven();
        let rows = rows(&fixture);

        // Branches, remotes and HEAD — the same set `all_tips` walks from.
        // Not `--all`, which would drag in the commits `git stash` writes and
        // which the graph deliberately does not show as history.
        let expected: usize = fixture
            .git(&["rev-list", "--count", "--branches", "--remotes", "HEAD"])
            .trim()
            .parse()
            .expect("a count");
        assert_eq!(rows.len(), expected);

        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.index, i, "row indices are the walk's own order");
        }

        let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "no commit is delivered twice");
    }

    #[test]
    fn the_newest_commit_comes_first() {
        // Down the screen is back in time; that is what the row order means.
        let fixture = Fixture::woven();
        let rows = rows(&fixture);

        assert_eq!(rows[0].summary, "Merge feature/split-view");
        assert_eq!(rows.last().expect("a last row").summary, "Initial import");
    }

    #[test]
    fn date_order_interleaves_parallel_histories() {
        // Commit time, not topology: `Rewrite line 38` landed on main around
        // the same moment as the feature's commits, so it sits *between* them
        // rather than after the feature has been drawn in full.
        let fixture = Fixture::woven();
        let walked = rows(&fixture);
        let summaries: Vec<&str> = walked.iter().map(|r| r.summary.as_str()).collect();
        let start = summaries
            .iter()
            .position(|s| *s == "Start the split view")
            .expect("the feature tip");
        let on_main = summaries
            .iter()
            .position(|s| *s == "Rewrite line 38")
            .expect("the main-line rewrite");
        let parent = summaries
            .iter()
            .position(|s| *s == "Rewrite line 3")
            .expect("the feature's parent");
        assert!(
            start < on_main && on_main < parent,
            "date order mixes the main-line commit into the feature: {summaries:?}"
        );
    }

    #[test]
    fn branch_order_keeps_a_line_of_history_together() {
        let fixture = Fixture::woven();
        let repo = fixture.open();
        let refs = RefIndex::build(&repo).expect("index");
        let tips = all_tips(&repo).expect("tips");

        let mut out = Vec::new();
        walk_pinned(&repo, tips, &refs, &[], GraphOrder::Branch, |row| {
            out.push(row);
            Flow::Continue
        })
        .expect("walk");

        let summaries: Vec<&str> = out.iter().map(|r| r.summary.as_str()).collect();
        assert_eq!(summaries[0], "Merge feature/split-view");

        let start = summaries
            .iter()
            .position(|s| *s == "Start the split view")
            .expect("the feature tip");
        let rewrite = summaries
            .iter()
            .position(|s| *s == "Rewrite line 3")
            .expect("the feature's parent");
        assert_eq!(
            rewrite,
            start + 1,
            "the feature's commits stay adjacent rather than mixing with main: {summaries:?}"
        );
        assert_eq!(
            out.len(),
            rows(&fixture).len(),
            "both orders walk the same commits"
        );
    }

    #[test]
    fn a_row_carries_everything_needed_to_paint_it() {
        let fixture = Fixture::woven();
        let row = &rows(&fixture)[0];

        assert_eq!(row.id, fixture.head());
        assert_eq!(row.short, fixture.head()[..7]);
        assert_eq!(row.author_name, "Ada Lovelace");
        assert_eq!(
            row.author_email, "ada@example.com",
            "the portrait is generated from this, so it has to survive the walk"
        );
        assert_eq!(row.initials, "AL");
        assert_eq!(row.parents.len(), 2, "the tip of the fixture is a merge");
        assert!(row.time > 0);
    }

    #[test]
    fn the_summary_is_the_first_line_of_the_message() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "a\n");
        fixture.git(&["add", "-A"]);
        fixture.git(&["commit", "-q", "-m", "A subject", "-m", "A body paragraph."]);

        let rows = rows(&fixture);

        assert_eq!(rows[0].summary, "A subject");
    }

    #[test]
    fn ref_chips_are_attached_to_the_commits_they_point_at() {
        let fixture = Fixture::woven();
        let rows = rows(&fixture);

        let tip = &rows[0];
        let names: Vec<&str> = tip.refs.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"v0.2.0"));

        let root = rows.last().expect("a last row");
        assert!(root.refs.is_empty(), "the initial import carries no ref");
    }

    #[test]
    fn a_merge_puts_its_second_parent_in_its_own_lane() {
        let fixture = Fixture::woven();
        let rows = rows(&fixture);

        let widest = rows.iter().map(|r| r.lane).max().expect("a lane");
        assert!(
            widest >= 1,
            "a branch that was merged has to occupy a second lane"
        );

        let merge = &rows[0];
        assert_eq!(merge.lane, 0);
        assert!(
            rows.iter().skip(1).any(|r| !r.edges.is_empty()),
            "the band below the merge has to carry its elbow"
        );
    }

    #[test]
    fn the_sink_can_stop_the_walk_before_the_end_of_history() {
        // This is the backpressure the windowing in src-tauri relies on: no
        // walk ever runs to the end of history to produce its first row.
        let fixture = Fixture::woven();
        let repo = fixture.open();
        let refs = RefIndex::build(&repo).expect("index");
        let tips = all_tips(&repo).expect("tips");

        let mut seen = 0;
        let total = walk(&repo, tips, &refs, |_| {
            seen += 1;
            if seen == 2 {
                Flow::Stop
            } else {
                Flow::Continue
            }
        })
        .expect("walk");

        assert_eq!(seen, 2);
        assert_eq!(total, 2);
    }

    #[test]
    fn walking_an_empty_repository_says_it_is_empty() {
        let fixture = Fixture::empty();
        let repo = fixture.open();
        let refs = RefIndex::build(&repo).expect("index");
        let tips = all_tips(&repo).expect("tips");

        assert!(tips.is_empty());
        let error = walk(&repo, tips, &refs, |_| Flow::Continue).unwrap_err();
        assert!(matches!(error, Error::EmptyRepository));
    }

    #[test]
    fn all_tips_includes_a_branch_that_is_not_merged_into_head() {
        // Walking from HEAD alone would hide it, which is the whole point.
        let fixture = Fixture::woven();
        fixture.git(&["switch", "-q", "-c", "chore/tooling", "main"]);
        fixture.write("tools.txt", "tooling\n");
        fixture.git(&["add", "tools.txt"]);
        let unmerged = fixture.commit("Add a tooling note");
        fixture.git(&["switch", "-q", "main"]);

        let rows = rows(&fixture);

        assert!(rows.iter().any(|r| r.id == unmerged));
    }

    #[test]
    fn all_tips_does_not_repeat_a_commit_several_refs_point_at() {
        let fixture = Fixture::woven();
        fixture.git(&["branch", "another-name-for-main", "main"]);

        let repo = fixture.open();
        let tips = all_tips(&repo).expect("tips");

        let mut sorted = tips.clone();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before);
    }

    #[test]
    fn short_ids_are_seven_characters() {
        let fixture = Fixture::woven();
        let id = gix::ObjectId::from_hex(fixture.head().as_bytes()).expect("id");
        assert_eq!(short_id(&id).len(), 7);
    }

    #[test]
    fn already_visited_parents_do_not_leave_dangling_edges() {
        let mut state = LaneState::new();
        let c1 = gix::ObjectId::from_hex(b"1111111111111111111111111111111111111111").unwrap();
        let c2 = gix::ObjectId::from_hex(b"2222222222222222222222222222222222222222").unwrap();
        let c3 = gix::ObjectId::from_hex(b"3333333333333333333333333333333333333333").unwrap();

        // c1 is a tip that branches into c2
        state.step(c1, &[c2]);
        // c2 is visited
        state.step(c2, &[c3]);
        // c4 is a merge commit visited later whose second parent is c1 (which was already visited)
        let c4 = gix::ObjectId::from_hex(b"4444444444444444444444444444444444444444").unwrap();
        state.step(c4, &[c3, c1]);
        // c3 is root commit
        let row = state.step(c3, &[]);

        // c3 has no parents, all lanes should be closed
        assert_eq!(state.width(), 0);
        assert!(!row.edges.iter().any(|e| e.from != 0 && e.to != 0));
    }
}

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-101 — Plan

**Item:** [`agile/items/FEAT-101-merge-without-conflicts.md`](../items/FEAT-101-merge-without-conflicts.md)

## Approach

`merger::land` (`crates/spagitty-core/src/merger/land.rs`) takes what the plan
read — both names, both tips, the target, the strategy, the message and the
resolutions — and refuses first if either tip moved (`Error::Stale`).

**The tree** is `merged_tree`: the dry run again, on the same `merge-tree` or
scratch-worktree path FEAT-100 chose, with each conflicted path replaced.
Every conflicted path needs exactly one resolution and no other path may have
one; either mismatch means the screen is showing a different merge. On the
`merge-tree` path the replacements go through an index file of Spagitty's own
(`GIT_INDEX_FILE` under `<git dir>/spagitty/merger/`, removed after):
`read-tree` the merged tree, `update-index --index-info` the chosen blobs (text
through `hash-object -w --stdin --path=`, so filters and line endings apply as
for a file at that path; a side whole by its stage's blob; mode 0 to delete),
`write-tree`. On the fallback the files are written in the scratch worktree,
then `add -A` and `write-tree` there. A test checks both paths build the same
tree.

**The commit**: `commit-tree` with the receiving tip and the source tip as
parents (merge) or the receiving tip alone (squash). A rebase is `rebase
--onto <target> <base>` in a detached scratch worktree at the source's tip; a
stop aborts it and says so (FEAT-103 takes over). A fast-forward is the
source's tip, refused unless the receiving tip is in its history.

**The move** is last. A new branch: `branch --no-track`. A branch checked out
in some worktree (from `worktree list`): its `HEAD` is checked against the tip
read, then `merge --ff-only` there. Otherwise `update-ref -m <reason> <ref>
<new> <old>`, which git refuses if the ref is not at `<old>`.

**The screen**: Merge now opens `LandDialog.svelte`, which also draws the done
state. The store's `phase` moves plan → commit → done; refs moving under the
screen do not re-prime it while the dialog is up, because the merge itself
moves them.

## Risks

- A target checked out with uncommitted changes in the way: git's `--ff-only`
  refuses, the commit object is left unreferenced, nothing moves. Tested.
- Hooks do not run on this path. Recorded in the item's non-scope.

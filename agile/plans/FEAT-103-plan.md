<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-103 — Plan

**Item:** [`agile/items/FEAT-103-a-rebase-that-stops-in-merger.md`](../items/FEAT-103-a-rebase-that-stops-in-merger.md)

## Approach

`merger/replay.rs` keeps no state of its own: the worktree is the state. Its
path is `<git dir>/spagitty/merger/rebase-<key>`, the key an FNV-1a hash of
both names, both tips, the target and the new name, so the same merge always
finds the same worktree and a merge whose branches moved finds none. Every
call checks the tips first (`Error::Stale`).

- `rebase_open` creates it if it is not there (`worktree add --detach` at the
  source's tip, then `shell::rebase_replay`: `-c merge.conflictStyle=diff3
  rebase --onto <target> <base>`, exit 1 being a stop) and reports the state.
- The state is read from the worktree's own git directory: a rebase in
  progress (`rebase::progress_in`) is a stop, with `REBASE_HEAD`'s commit and
  each conflicted file read by `conflicts::sides` against the worktree;
  otherwise it is done, and done only if the receiving tip is in its history.
- A stop's files are mapped to A and B: when B receives, git's ours is B, so
  the sides are swapped and the markers rewritten with A's lines first
  (`swap_markers`), and a resolution's side is swapped back before it is
  settled.
- `rebase_continue` settles each file with `conflicts::settle` in the
  worktree, then `rebase --continue` with the accept-as-is message editor, or
  `--skip` when nothing is staged. `rebase_skip`, `rebase_abort` (`--abort`,
  then `worktree remove --force` and `prune`) and `rebase_finish` (FEAT-101's
  `move_to`, then the same removal).

The screen: the resolve store's `openRebase`, `follow`, `continueRebase`,
`skipRebase` and `abortRebase`; `MergerResolve.svelte` draws the Rebasing
header and Continue / Skip this commit / Abort in place of Complete merge; the
dialog's button calls `merger_rebase_finish` through `merger.land`.

## Risks

- A worktree left behind by a crash is found again by the same merge and
  resumed; Abort removes it. One for a merge whose branches have since moved is
  orphaned under the git directory until `git worktree prune` — named in the
  item's record so it can be swept later.

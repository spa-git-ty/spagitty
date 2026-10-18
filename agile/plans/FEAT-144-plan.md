<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-144 — Plan

**Item:** [FEAT-144](../items/FEAT-144-the-landing-lane.md)

Written on 2026-10-18, before the code. Follows the proposal's order of work,
step 1: the lane for one task, with compare-and-swap and the ledger.

## The pieces

1. **Two pieces of Merger's `land`, made public** in
   `spagitty-core/src/merger/land.rs`: `candidate` (the merge commit of a source
   tip into a receiving tip, written by `merge-tree` and `commit-tree`, on no
   branch; a conflict is an error naming the path) and `advance` (move a local
   branch from the tip that was read to the candidate: `merge --ff-only` where
   it is checked out, `update-ref <new> <old>` where it is not). Neither checks
   anything out.
2. **The lane** in `spagitty-farm/src/lane.rs`: read the destination's tip, build
   the candidate, check it when the tip is not already in the task's commit,
   advance; a moved tip is a rebuild, three times at most.
3. **Checking a candidate** in a scratch worktree of the lane's own,
   `.spagitty/farm/lane/<destination>/`, detached at the candidate, with the
   task's verification commands; it is removed afterwards.
4. **The ledger** in `persistence/store.rs`: `ledger.jsonl`, appended one line per
   move and read with a torn last line dropped. `kind` is `land`, `retry-lost`
   or `merger`; each names the task, its agent and its reviewer.
5. **The service** — `merge_checked` uses the lane, so the person no longer has
   to be on the destination and nothing is merged in their working copy;
   `mark_merged` writes a `merger` entry.
6. **The screens** — `farm_ledger` lists the moves, and Activity shows them.

## What is decided, and why

- **A merge commit, always,** as `merge_checked` made: a farm branch that fast-
  forwards disappears from the history.
- **Checks only when the tip moved.** When the destination's tip is already in
  the task's commit, the candidate's tree is the task's tree, which was checked.
- **A red candidate is a send-back** through the same path a red check on the
  task's own commit takes, so the tries are counted once.
- **A conflict blocks the task** and touches no files: Merger resolves it, as it
  does for a person (FEAT-117).

## Not in this plan

- Landing several tasks at once, the two tiers of checks and the revert walk.

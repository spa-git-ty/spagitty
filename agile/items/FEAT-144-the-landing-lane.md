<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-144 — The landing lane: merging without a queue, and a log of every branch move

**Status:** Done — on `release/3.2.0`; unmerged. Covered by Rust tests in WSL and component tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.2.0`.
**Screens:** 1Q.
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), steps 2 and 3, worked out in [the landing lane and the local hub](../../docs/proposals/landing-lane-and-local-hub.md), §5.1 and §5.5.

## Change

When the farm merges a task by itself (Automatic and Unattended), it ran
`git merge --no-ff` in the person's working copy, refused unless the person was
standing on the destination branch, and left a conflict in their files. What it
merged was the task's commit, verified alone; the merge result was never
checked. The lane merges the way Merger lands (FEAT-101): it builds the merge
commit without checking anything out, checks it, and moves the branch by
compare-and-swap. Every branch move it makes is written to a ledger.

## Acceptance criteria

- **No working copy is written** by a farm merge, except `merge --ff-only` on a
  destination that is checked out, which refuses rather than overwrite
  uncommitted work.
- **The person may be on any branch.** *Switch back to <x> before merging this
  task* goes away.
- **Land what was tested.** The lane reads the destination's tip T and builds
  the candidate C = T merged with the task. When T is already in the task's
  commit, C's tree is the one already verified and the checks are not run again.
  Otherwise the task's checks run on C, in a scratch worktree, before anything
  moves.
- **Compare-and-swap.** The branch moves from T to C only if it is still at T.
  A tip that moved is a rebuild, up to three times, never an overwrite.
- **A conflict blocks the task** with *Conflicts with <destination>*, touching
  no files; it is resolved in Merger, as a person-made merge is (FEAT-117).
- **A red candidate** sends the task back to its agent with the output, as a red
  check on its own commit does.
- **The ledger:** `.spagitty/farm/ledger.jsonl`, one line per move — sequence,
  time, ref, old, new, kind (`land`, `retry-lost`), the task, its agent and
  reviewer, and the checks. Appended, never rewritten; a partial last line is
  dropped on read. Merges made in Merger for a task are recorded too
  (`mark_merged`).
- **Read by the screens:** a command lists the ledger, and the Farm's Activity
  shows each land with who did it.
- **Tests:** Rust tests on fixture repositories: a land while the person is on
  another branch; a land when the destination is checked out; a moved tip
  rebuilt; a conflict blocked with the files untouched; a red candidate sent
  back; the ledger's round trip and a torn last line.

## Not in this item

- Landing several tasks as one move, the two tiers of checks and the revert
  walk (the proposal's §5.2 and §5.3).
- Own copies and intake (FEAT-147).

## What was built

- **Merger's `candidate` and `advance`**, public in `spagitty-core/src/merger/land.rs`: the merge commit written with `merge-tree` and `commit-tree` on no branch, and the move by `--ff-only` or `update-ref <new> <old>`.
- **The lane** in `spagitty-farm`'s `service.rs` (`land`, `check_candidate`, `LaneStop`): `merge_checked` reads the tip, builds the candidate, checks it in `.spagitty/farm/lane/<branch>/` when the tip moved, and advances, rebuilding up to three times.
- **The ledger** in `model/ledger.rs` and `persistence/store.rs` (`ledger.jsonl`), written by every land, every lost race and every Merger merge, read by `farm_ledger` and shown in Activity's *Branch moves*.

## Differences from the item

- **The ledger is not on Home yet.** The handoff's HUB.md says the log feeds Home and the notification center as well; both show the farm's merges through its events today, and reading the ledger there is left for when there are batches to show.
- **The merge message** is `Merge branch '<task branch>' into <destination>`, where `git merge` of a commit id wrote `Merge commit '<id>'`.

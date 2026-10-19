<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-147 — Plan

**Item:** [FEAT-147](../items/FEAT-147-own-copies-and-intake.md)

Written on 2026-10-19, as the work began.

## The pieces

1. **The copy** — `workspace/copy.rs` `create`: `clone --local --no-tags --no-checkout`, `checkout -B <farm branch> <start>`, `remote remove origin`, every other local branch deleted, the person's name and email copied in, signing off. The start is written beside the copy.
2. **Intake** — `intake`: fetch the copy's branch into the inbox ref, check ancestry, merges, paths (with `lock::overlaps`), protected paths, submodule pointers and blob sizes (`ls-tree -l`), then move the farm branch with `update-ref <new> <old>` (or an empty old value to create it).
3. **The seam** — rather than the proposal's `Workspaces` trait, the farm branch: once intake has moved it, verification, review and the lane read it as they read a worktree's. Nothing above `workspace` learns which kind it was.
4. **The service** — `start_task` cuts a copy on a copy farm; `verify_with_stop` calls `take_from_copy` first; a refusal blocks the task.
5. **Cleanup** — `worktree::remove` removes a copy too; `cleanup::sweep` sweeps unclaimed copies whose commit is in HEAD.
6. **The screen** — a third card in *Where each task works*.

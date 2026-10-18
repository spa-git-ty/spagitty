<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-143 — A slice per task

**Status:** Done — on `release/3.2.0`; unmerged. Covered by Rust tests on Windows and WSL; not looked at in the running application (the author is checking it).
**Branch:** `release/3.2.0`.
**Screens:** 1Q.
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), step 1. The author chose the local hub over a server on 2026-10-18.

## Change

Every task already gets a worktree of its own (`workspace/worktree.rs`). A
worktree is the whole repository, checked out, and an agent given the whole
repository reads and edits more than its task needs. A **slice** is the task's
worktree with `git sparse-checkout` limited to the paths the task may touch and
the paths the planner says it must read. The agent edits in the slice; the
checks still run on the whole tree.

## Acceptance criteria

- **Behind a trait.** `SliceProvider` (`open`, `close`) in `spagitty-farm`'s
  `workspace`, as `HUB.md` sketches it, with `WorktreeSlice` the one
  implementation. A server-backed provider later is a new implementation, not a
  rewrite.
- **Per farm.** `Farm.workspace` is `worktree` (today, the default) or `slice`.
  A farm written before reads as `worktree`.
- **A task's reading list.** `Task.reads`: paths the task needs to read but may
  not change, carried from the planner's plan and editable in the task editor.
- **The slice:** `git sparse-checkout set --no-cone` over the task's *May touch*
  and *Reads* paths, plus the repository's top-level files (manifests, lock
  files, `AGENTS.md`). A task with no paths at all gets the whole tree, since an
  empty slice is a task with nothing to work on.
- **Checks on the whole tree.** Before a task's verification commands run, the
  worktree's sparse checkout is turned off, so the build sees every crate; it is
  put back when the task is sent back to its agent.
- **The prompt says so.** The implementation prompt names the slice: which
  paths are checked out and that the rest is not missing, only not here.
- **The wizard** offers *A slice per task* in step 3 (FEAT-141), tagged `new`.
- **Tests:** Rust tests on a fixture repository: a slice holds the declared
  paths and the top-level files and not the rest; the checks see the whole tree;
  a send-back restores the slice; a farm without the field reads as worktrees;
  the planner's `reads` are kept.

## Not in this item

- Leases on what an agent reads. Leases stay on what it may touch.
- A provider over the network (HUB.md's option 2).

## What was built

- **`workspace/slice.rs`**: the `SliceProvider` trait, `WorktreeSlice`, the patterns (`/*`, `!/*/`, then each declared path anchored), and `narrow`, `widen` and `is_sparse` over `git sparse-checkout` in non-cone mode.
- **The farm:** `Farm.workspace` (FEAT-141 carries it in the remembered setup), `Task.reads` from the planner's `reads` and the task editor, and the service narrowing at each run start and widening before verification.
- **The prompt's `YOUR SLICE`** section on a slice farm.
- **The screens:** *Where each task works* in the wizard's step 3 and the Rules sheet; *Reads* in the task editor.

## Differences from the item

- **The reviewer** works in the widened tree: verification runs before review, so a task in review is whole.

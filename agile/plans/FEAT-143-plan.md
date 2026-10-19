<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-143 — Plan

**Item:** [FEAT-143](../items/FEAT-143-a-slice-per-task.md)

Written on 2026-10-18, before the code.

## The pieces

1. **The farm's choice** — `Farm.workspace` (`worktree` or `slice`), added with
   FEAT-141's remembered setup so the wizard can carry it. `farm_configure`
   takes it.
2. **What a task reads** — `Task.reads`, from the planner's `reads` (the plan
   contract gains the field) and from the task editor, beside *May touch*.
3. **The provider** in `workspace/slice.rs`: the `SliceProvider` trait from
   `HUB.md` (`open`, `close`) and `WorktreeSlice`, which cuts the task's worktree
   as today and narrows it with `git sparse-checkout set --no-cone`. Two free
   functions do the switching: `narrow` (the patterns for a task) and `widen`
   (`git sparse-checkout disable`).
4. **The patterns** — `patterns(task)`: the top-level files (`/*`, `!/*/`), then
   each *May touch* and *Reads* path anchored at the root. A task with neither
   gets no slice, and the worktree is whole.
5. **The service** narrows the worktree each time a run starts on a slice farm,
   and widens it before verification, so the checks see every crate. A task
   sent back is narrowed again when its next run starts.
6. **The prompt** says the worktree is a slice, which paths are in it, and that
   what is missing is not missing from the repository.
7. **The screens** — the wizard's step 3 and the Rules sheet offer *Where each
   task works*; the task editor gains *Reads*.

## What is decided, and why

- **Non-cone patterns.** A task's paths are globs (`src/auth/**`) as often as
  folders; cone mode takes folders only.
- **The top-level files always.** Manifests, lock files and `AGENTS.md` are
  what a build and an agent read first, and they are small.
- **Narrowed at every run, widened before checks,** rather than kept in two
  worktrees: one worktree per task stays the rule, and the agent's uncommitted
  work survives the switch because `sparse-checkout` only adds or removes files
  the index says are clean.

## Not in this plan

- Leases on reads, a remote provider, and sparse clones (HUB.md's later steps).

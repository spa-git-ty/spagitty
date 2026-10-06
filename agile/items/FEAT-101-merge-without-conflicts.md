<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-101 — Merge without conflicts

**Status:** Backlog
**Screens:** 1S.
**Raised by:** the author, 2026-10-06. Merge now, for every direction and strategy, into a branch that may not be checked out, including Squash. Slice 2 of `design_handoff_merger/`.

## Problem

Merger (FEAT-100) shows what a merge would do and cannot do it. A branch that is not checked out cannot be merged into at all today without checking it out.

## Change

- **Merge now** lands a merge the dry run found clean: a merge commit, a squash, a rebase then fast-forward, or a fast-forward.
- Into a branch that is not checked out without checking it out: the result is built from the dry run's tree with `commit-tree`, and the branch moved with `update-ref` against the tip that was read. A branch checked out in a worktree is brought forward there with `merge --ff-only`, which refuses rather than overwrite uncommitted work.
- Into a new branch, created at the result.
- A done state: *<target> now includes <source>*, Open in Graph, Merge another.

## Acceptance criteria

- Merging into a branch that is not checked out never changes the checked-out branch or its working tree.
- A branch that moved since the plan was read is refused, not overwritten.

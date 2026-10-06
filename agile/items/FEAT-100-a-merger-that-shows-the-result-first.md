<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-100 — A merger that shows the result first

**Status:** Open — built and merged into `main`; the manual sweep is not yet run.
**Branch:** `feature/FEAT-100-a-merger-that-shows-the-result-first`
**Screens:** 1S (new), chrome (the rail).
**Raised by:** the author, 2026-10-06. Merging today is a graph action
(drag a branch onto another, `ops::integrate`): it only merges into the
checked-out branch, says nothing of the result before it happens, and
conflicts appear only once git has stopped. The design is
`design_handoff_merger/` (Claude Design, the same date); this is its slice 1.

## Problem

There is no place to take two branches and see, before anything is written,
which one receives the result, what comes in, which files change, and whether
and where they conflict.

## Change

- **Merger (1S)**, a new screen at `/merge`, on the rail after Conflicts and
  before Branches, always shown, with a new icon: two lanes meeting and
  flowing down.
- **The stage**: branch A on the left, B on the right, the raised Result card
  between them. Each branch card has a picker (any local branch,
  remote-tracking branch or tag), its commits since the merge base and the
  newest three. The arrow from the side whose commits come in is solid, in
  its colour, with the count; the other is dashed — *continues as*, or
  *starts from* for a new branch. The receiving card is outlined and says
  *Lands here*.
- **Where it lands**: Into A, Into B, Into a new branch (named
  `merge/<a>-<b>`, starting from A), and Swap. Only a branch here can
  receive one: a remote branch or a tag says so and points at a new branch.
- **How it lands**: Merge commit, Squash, Rebase then fast-forward,
  Fast-forward only. Fast-forward is disabled with its reason when both sides
  have their own commits. *History after* redraws for each.
- **The forecast**: one sentence for the strategy, three numbers, and the
  conflict box — *N conflicts in M files*, a meter of conflicted against clean
  files, and where it was measured. For a rebase it says how many commits it
  may stop on: those that touch a conflicted file.
- **What changes**: every file either branch touched, conflicts first, then
  both changed and merging cleanly, then coming in, then already on the target.
- **The dry run writes nothing anybody reads.** `git merge-tree --write-tree`
  (git 2.38) with diff3 markers; for an older git, a `merge --no-commit` in a
  scratch worktree under the git directory, removed afterwards. The index, the
  working tree and every ref are byte-for-byte unchanged.

## Non-scope

The later slices, each its own item: merging without conflicts (FEAT-101),
resolving (FEAT-102), a rebase that stops (FEAT-103) and the rail as the
author decided it (TASK-056). The buttons that start them are shown, disabled.

## Acceptance criteria

- Choosing two branches and a direction shows, before anything is written,
  which branch receives the result, how many commits come in, which files
  change, and whether and where there will be conflicts.
- Changing the direction or the strategy re-derives the plan without asking
  git again; picking another branch asks again.
- The index, working tree and refs are unchanged by a forecast, by either dry
  run (`merger::tests::a_forecast_writes_nothing_anybody_reads`).
- Both themes and keyboard-only use work: every control is a button or an
  input, the pickers are the shared `Menu`.

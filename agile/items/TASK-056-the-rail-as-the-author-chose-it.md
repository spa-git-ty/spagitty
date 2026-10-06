<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-056 — The rail as the author chose it

**Status:** Open — built and merged into `main`; the manual sweep is not yet run.
**Branch:** `task/TASK-056-the-rail-as-the-author-chose-it`
**Screens:** chrome (the rail), 1A (the drag).
**Raised by:** the author, 2026-10-06, as slice 5 of `design_handoff_merger/`.
The handoff left three questions for the author, answered the same day:
restore Stash, Tags and Reflog as rows; always show Rebase, Log and All
repositories; and let the graph's drag of one branch onto another open
Merger's plan instead of merging straight away. Badges was left as it is.

## Problem

TASK-045 took Stash, Tags and Reflog off the rail, as tabs Branches stood for,
and showed Rebase, Log and All repositories only while open. The author found
them missing. And the graph's drag merged into the checked-out branch from a
menu, with none of the forecast Merger now gives.

## Change

- **Stash, Tags and Reflog are `tools` rows again**, always shown, after
  Rebase and Log; the Branches row no longer stands for them. Each refs
  screen keeps its segmented control naming all four.
- **Rebase, Log and All repositories are always shown.** Badges stays as it
  was: on the rail only while open, and only with the delight layer on.
  Conflicts stays as it was: shown while there is something to resolve.
- **Dragging a branch label onto another opens Merger** with the one dropped
  on as A, receiving, and the dragged one as B, coming in. The right-click
  menu's merge entries, which act on the checked-out branch, are unchanged.
- The reversal is recorded in TASK-045's item.

## Acceptance criteria

- The rail shows the tools always, and Stash, Tags and Reflog each as its own
  row, active on its own screen.
- A drag opens Merger with the dragged branch coming into the one it was
  dropped on, and nothing is merged until Merger is told to.

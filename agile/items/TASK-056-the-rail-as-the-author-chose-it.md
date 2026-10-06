<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-056 — The rail as the author chose it

**Status:** Backlog
**Screens:** chrome, 1A.
**Raised by:** the author, 2026-10-06. Slice 5 of `design_handoff_merger/`. The author answered the handoff's open questions on 2026-10-06: restore Stash, Tags and Reflog as rows, always show Rebase, Log and All repositories, and let the graph's drag of one branch onto another open Merger's plan.

## Problem

TASK-045 took Stash, Tags and Reflog off the rail and made Rebase, Log and All repositories show only while open. The author feels them missing. The graph's drag merges straight away, without the forecast Merger gives.

## Change

- Stash, Tags and Reflog back as `tools` rows, always shown; the reversal recorded in TASK-045's item.
- Rebase, Log and All repositories always shown.
- Dragging a branch onto another in the graph opens Merger's plan for that pair.

## Acceptance criteria

- The rail shows the restored rows; Branches no longer stands for Stash, Tags and Reflog.
- A drag opens Merger with the dragged branch coming into the one it was dropped on.

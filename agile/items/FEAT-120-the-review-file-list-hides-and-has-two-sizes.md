<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-120 — The review room's file list can be hidden, and is compact or detailed

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

The touched-files panel on the left of the review room (`src/lib/review/RoomFiles.svelte`) is always there and always in one form. Each file has:

- a viewed tick;
- its name;
- a facts line: counts, *author*, *conflict fix*, threads, *agent read*.

It cannot be put away to give the diff the width, and there is no shorter form for a pull request with many files.

## Change

- **Hide:** the panel can be hidden and comes back from an edge tab, as the conversation panel already does (BUG-054). The choice is remembered.
- **Two sizes:** a toggle switches between:
  - *Detailed*, today's rows;
  - *Compact*, one line per file with the tick, name and counts, and the marks as small dots.
  
  The choice is remembered.
- The filters and the legend follow the chosen size; in *Compact* the legend can be shown on hover.

## Acceptance criteria

- The panel hides and shows with a control and a keyboard shortcut, and the diff takes the freed width.
- *Compact* and *Detailed* both keep the viewed tick, the agent marks and the selected file.
- Both choices survive a restart.
- Tests in `src/routes/review/room.test.ts` cover both.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/review/RoomFiles.svelte`, `src/lib/review/ReviewRoom.svelte` (the layout), `src/lib/review/room.svelte.ts`, and the conversation panel's edge tab for the pattern.
- **Branch:** `feature/FEAT-120-the-review-file-list-hides-and-has-two-sizes`, with plan, testing documents and a changelog entry when the work starts.

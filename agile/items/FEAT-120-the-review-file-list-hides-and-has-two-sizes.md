<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-120 — The review room's file list can be hidden, and is compact or detailed

**Status:** Done — on `feature/FEAT-120-the-review-file-list-hides-and-has-two-sizes`; unmerged. Not yet looked at in the running app.
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

## What was done

- **Hide.** The file list has a button (*Put the files away*) and the **f** key, which puts it away and brings it back; away, an edge tab down the room's left side says *Files* with the number still to view and brings it back, as the conversation's tab does on the right. The diff takes the freed width.
- **Two sizes.** A *Detailed* / *Compact* choice at the top of the list. Compact is a line per file: the tick, the name and the counts, with *author*, *conflict fix*, open threads and *agent read* as small dots, each with its words as a title, and the agent's findings count and the *reading now* ring as before. Both keep the viewed tick, the selected file and the agent marks. The filters take shorter words in Compact (*Fixes*), and its legend waits behind *Legend* until the pointer or the keyboard is on it.
- Both choices are remembered on this machine (`files-view.svelte.ts`, `spagitty.review.files`).

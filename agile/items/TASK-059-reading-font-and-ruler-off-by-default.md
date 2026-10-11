<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-059 — The reading font and the focus ruler are off by default, and off the review bar

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R, 1K.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

The review room opens with the reading font set (`Aa`) and the focus ruler (the ruler icon) both on, and both buttons sit in the review bar beside the controls used on every file. The author does not want either by default, and does not want them on the bar.

- `src/lib/review/room.svelte.ts`: `ruler` and `readingSet` start `true` (lines 97–98) and are set back on each time a pull request opens (lines 239–240).
- `src/lib/review/RoomPill.svelte`: the two buttons, lines 66–82.
- `src/lib/reading.svelte.ts`: `DEFAULTS` is Atkinson Hyperlegible Mono with the ruler on one line (FEAT-090).

## Change

- Both are off by default: the room opens in plain code with no ruler.
- Both buttons leave the review bar.
- Settings › Reading is where they are turned on. It gets a choice for whether the review room uses the reading set, so anyone who wants it keeps it.
- Someone who chose a reading font or a ruler in Settings keeps that choice. Only the defaults change.

## Acceptance criteria

- A fresh install opens the review room in the plain code font with no ruler.
- The review bar has no `Aa` and no ruler button.
- Turning either on in Settings › Reading applies it in the review room.
- Tests in `src/routes/review/room.test.ts` and the reading store cover the defaults.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** the three files above and `src/lib/settings/` (the Reading section).
- **Branch:** `task/TASK-059-reading-font-and-ruler-off-by-default`, with plan, testing documents and a changelog entry when the work starts.

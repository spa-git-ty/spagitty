<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-059 — The reading font and the focus ruler are off by default, and off the review bar

**Status:** Done — on `task/TASK-059-reading-font-and-ruler-off-by-default`; unmerged.
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

## What was done

`DEFAULTS.ruler` is `off` and a new `reviewRoom` choice (default off) says whether the review room uses the reading set. The room reads both from Settings › Reading (`room.ruler` and `room.readingSet` are getters over `reading.current`), so the per-room state and its two toggles are gone, and `Aa` and the ruler button leave the review bar. Settings › Reading gains *Review room · Use the reading set*; the ruler's own choice (*Off / One line / Whole chunk*) was already there. A line is still `focused` (its comment `+` shows) whatever the ruler says; only `ruled` paints the band. Someone who has saved Reading settings keeps them: only what is missing from storage takes the new defaults, so a person who changed any Reading choice before will also have the ruler they had saved, and one who never opened Settings › Reading gets the new defaults.

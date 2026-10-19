<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-092 — Nothing can be dragged into place on Windows

**Status:** Fixed — on `release/3.3.0`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Screens:** Settings › Rail, 1E (Rebase), 1X (Hooks), 1A (Graph columns)
**Raised by:** the author, 2026-10-19, on 3.2.0: "rail ordering not working on settings".

## What happens

Dragging a row in Settings › Rail by its grip shows the drag, and nothing moves
when it is let go. The same is true of the rebase plan's rows, Hooks' blocks and
the Graph's column headings. Alt with the arrows works.

## Cause

All four used HTML drag and drop (`draggable`, `dragover`, `drop`). Tauri's
window handles drags itself unless `dragDropEnabled` is turned off, so that a
file dragged in from the desktop arrives with its path — Releases uses it to
upload files. While it does, WebView2 on Windows never sends the page
`dragover` or `drop`. The component tests fired the events by hand, so they
passed.

Turning `dragDropEnabled` off would fix the rows and lose Releases' file drops.

## Acceptance criteria

- A row dragged by its grip onto another row takes that row's place, on the
  Rail, the rebase plan, Hooks and the Graph's headings, on Windows as well.
- A row may only be dropped where it could go before (the Rail: its own group).
- A press that does not travel is still a click; Escape gives a drag up.
- Files dragged onto Releases still upload.

## Fix

- `ui/sortable.ts`: a Svelte action that moves rows with pointer events, which
  the window leaves alone. Rows carry `data-sort-key`, grips `data-sort-grip`,
  resize handles `data-sort-ignore`.
- `RailSection.svelte`, `RebasePlan.svelte`, `HooksScreen.svelte` and
  `GraphHeader.svelte` use it in place of HTML drag and drop.
- Tests: `testing/pointer.ts` drags with pointer events; the Rail and rebase
  plan tests drag that way, including a drop refused across groups, a press
  outside the grip, Escape, and a locked plan.

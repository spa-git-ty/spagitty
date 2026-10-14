<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-134 — Pin or hide what is on the rail

**Status:** Done — on `release/3.0.0`, the first of the 3.0 items; unmerged. Covered by unit and mounted tests, and looked at in the built frontend in headless Chrome with a stand-in for the backend; not looked at in the running application.
**Branch:** `feature/FEAT-134-pin-or-hide-what-is-on-the-rail`.
**Screens:** 1K (Settings › Rail, a new section), chrome (the rail, its More button).
**Raised by:** the author, 2026-10-12: a settings screen to pin or hide non-essential elements of the left bar.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §9. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `SettingsRail.dc.html` (interactive).

## Problem

3.0 adds four rows: Terminal, Files, Recipes and Investigate. TASK-056 put every tool back on the rail at the author's request. Together that is a rail some people will want shorter, and only they know which rows they never use.

## Change

Settings gains a **Rail** chip, after Appearance. Each screen is pinned or hidden, within its group. Hidden screens go under a **⋯ More** button at the end of Tools, or to the palette only. The default keeps everything pinned.

## Acceptance criteria

- **The section is wired properly**: `rail` is added to `SECTIONS` and gets its own branch in `routes/settings/+page.svelte`, and `sections.test.ts` covers it. `/settings#rail` lands on it.
- **Each row** shows the icon, label and code, with *Pin* or *Hide*:
  - **Farm and Settings** read *Always* and cannot be hidden.
  - **Conflicts and Badges** show their existing rules (*When there are any*, *While open*) instead of the choice.
- **Hidden screens go** *Under ⋯ More* or to *Palette only*:
  - The More button opens `Menu` with the hidden screens and their codes, and is keyboard reachable.
  - The palette lists every screen whatever is chosen.
- **The current screen is always on the rail**, hidden or not, while it is open: the `open` rule `isShown` already has.
- **Reorder** by dragging the grip or with `Alt+↑/↓`, within a group only.
- **Stored** in Spagitty's settings as `rail: { hidden, order, where }`. *Reset* returns to the default: everything pinned, the `NAV_ITEMS` order, More.
- **The section's preview** is the real `NavRail` rendered from the draft settings.
- **Tests**: `nav.test.ts` covers hidden rows, the More row, the open-screen exception and reorder within groups. The settings store round-trips `rail`.

## Not in this item

- **Hiding rows per repository.**
- **Moving a row to another group.**

## What was built

- **The model** is in `src/lib/nav.ts`: `RailSettings`, `railChoice` (always, rule or free), `isHidden`, `inOrder`, `moveBy`, `moveOnto`, `hiddenItems` and `railRows`, which adds the More row. `isShown` reads the choice after the open-screen rule, so the screen that is open is always shown.
- **The rail** (`NavRail.svelte`) draws `railRows`. More opens `Menu` with the hidden screens and their codes. The component takes the choice as a property and has a preview mode, which Settings uses.
- **The section** is `src/lib/settings/RailSection.svelte`, with its chip after Appearance and its own branch in the route.
- **Stored** as `rail: { hidden, order, where }` in `settings.json` (`src-tauri/src/settings.rs`). Screens are stored by code. A code this build does not know is kept in the file and ignored by the rail. The built-in order is stored as an empty list.
- **The palette** gained *Go to* commands for Merger, Review, Tags and Reflog, which it did not have. A test fails if a rail screen has none.

## Differences from the artboard

- ***Open labels on hover*** is drawn as a switch on the artboard. It is not in this item's criteria or in the stored shape, so it was not built. It needs the author's word.
- **The *3.0* mark** on a new screen's row is drawn in the agent's colour on the artboard. It is built in a muted outline, because the agent's colour means an agent (the handoff's own rule).
- **Farm and Settings have no grip**, because a group of one row has nothing to reorder.
- **The counts line** counts what is on the rail whatever happens: Conflicts and Badges are in neither number, as on the artboard.

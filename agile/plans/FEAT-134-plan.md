<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-134 — Plan

**Item:** [FEAT-134](../items/FEAT-134-pin-or-hide-what-is-on-the-rail.md)

Written with the code, on 2026-10-13, not ahead of it.

The rail already decides which rows it draws in one place: `isShown` and `navRows` in `src/lib/nav.ts`, read by `NavRail.svelte`. The choice a person makes is one more input to those two, so it goes into `NavContext` and nothing else about the rail's rules moves.

1. **The shape** is `RailSettings { hidden, order, where }`, as the item gives it. Screens are named by code. An empty `order` means the order of `NAV_ITEMS`, so the default is stored as nothing and a build that adds a screen does not need to migrate anybody's file.
2. **What can be chosen** is one function, `railChoice`: Farm and Settings are `always`, a row with a `shows` rule is `rule`, the rest are `free`. `isHidden` is true only for a `free` row, so a stored list that names Settings cannot take it away.
3. **Hiding** is one line in `isShown`, placed after the test for the open screen. That placement is the whole of "the current screen is always on the rail".
4. **Order** is `inOrder`: each group keeps its place and is re-sequenced inside itself. A screen the stored order does not name goes after the row it follows in `NAV_ITEMS`. `moveBy` and `moveOnto` refuse a move that leaves the group.
5. **More** is a row `railRows` adds at the end of Tools when something is hidden and `where` is `more`. The rail opens `Menu` from it. The menu is mounted outside the pill, because the pill's blur makes it the containing block of anything `position: fixed` inside it.
6. **The section** renders `NavRail` itself for its preview, with the choice passed as a property and a `preview` flag that makes it inert and keeps it closed.
7. **Storage** is one more key in `settings.json`. `Settings` in Rust stops being `Copy`, because the rail holds lists. The codes are strings there: which codes exist is the frontend's list, and a file from a newer build must keep a code this one does not know.
8. **The palette** must reach every screen, since *Palette only* promises it. Four rail screens had no *Go to* command. They are added, and a test compares the palette with `NAV_ITEMS`.

Not done as a copy of the rail drawn for the preview: two drawings of one thing fall out of step, and the item asks for the real component.

## What decides it is right

- With nothing chosen, the rail's rows are exactly what they were.
- A hidden screen is off the rail, under More, and back on the rail while it is open.
- Farm and Settings cannot be hidden by any stored value.
- A row moves within its group and never out of it.
- What is chosen is read back after a restart, and a file that cannot be read costs only the rail.

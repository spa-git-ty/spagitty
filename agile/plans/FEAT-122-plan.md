<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-122 — Plan

**Item:** [FEAT-122](../items/FEAT-122-one-branch-menu-that-can-be-searched.md)

## Steps

1. **The rules first, as plain functions.** `src/lib/branches/pick.ts`: `words`, `rank`, `search`, `sections` (groups, in a fixed order, cut to what a menu can hold), `marks` and `drawn` (where the match is, across the slash).
2. **The component.** `BranchMenu.svelte`: a field with the role of a combobox over a listbox of options. The focus stays in the field; a press on a row is stopped from taking it. The closing rules are `Menu`'s: Escape, a press elsewhere, the focus leaving, a resize, and the control's own second click (BUG-018).
3. **Where you are.** Two different facts, drawn differently: the name this control holds (ticked, accent, heavier, not picked again) and the branch checked out (`current`). In the toolbar they are the same branch; on a Merger card they need not be.
4. **Placement.** Above the control when there is no room under it. Height held once the names are in. Measured by laid-out size, since it is read while the menu is still growing in.
5. **Use it** in the toolbar and in `BranchPicker`, which Merger's cards and the Rebase screen share. The toolbar reads its list on every opening.
6. **The surface** is the shared `.floating` class, so `flat.test.ts`'s count of blurred components does not rise.

## Left for later

The two modals with a native `<select>`: TASK-069.

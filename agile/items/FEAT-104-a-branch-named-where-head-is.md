<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-104 — A branch named where HEAD is

**Status:** Open — on its branch; the manual sweep is not yet run.
**Branch:** `feature/FEAT-104-a-branch-named-where-head-is`
**Screens:** chrome, 1A.
**Raised by:** the author, 2026-10-07: the bottom bar's Branch went to the Branches screen; it should open a text field where HEAD points and take the branch from there, as GitKraken does.

## Change

- **Branch** in the bottom bar opens a name field in the graph, in HEAD's row, focused (opening the graph first if another screen was showing, and scrolling to the row).
- Enter creates the branch there and checks it out; spaces become dashes. Escape, or leaving the field, puts it away. An empty name does nothing.
- The graph's *Create branch here* on any commit opens the same field in that commit's row instead of a dialog.
- With nothing committed yet, Branch does nothing and says so on hover.

## Acceptance criteria

- One click and a typed name make and check out a branch at HEAD, without a dialog or another screen.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-102 — Resolve every conflict, every way

**Status:** Backlog
**Screens:** 1S, 1D.
**Raised by:** the author, 2026-10-06. Slice 3 of `design_handoff_merger/`: three columns, every choice, Base, the pill, syntax colours, choices kept per merge, Abort and the commit dialog. The resolver is shared with Conflicts (1D), as the author decided on 2026-10-06.

## Problem

Conflicts only offers a whole side, one marker region by side, or the whole file by hand, and the result does not say where each line came from.

## Change

- Three columns per conflict, A | Result | B, with the context, why it conflicts, and the commit on each side that made the change.
- Take A, Take B, Both A first, Both B first, Pick lines, Edit by hand, Reset, and All from A / All from B per file.
- Every result line carries its badge and side marker; lines that will not land fade.
- Base strip, line numbers that follow the choices, the pill, the commit dialog.
- Choices kept per (A, B, merge base) in application data.
- One three-column resolver component, used by Merger and by Conflicts.

## Acceptance criteria

- Every conflict can be resolved as A, B, A then B, B then A, a line-by-line pick, or free text, and every result line says where it came from.
- Abort at any point leaves the repository as it was.

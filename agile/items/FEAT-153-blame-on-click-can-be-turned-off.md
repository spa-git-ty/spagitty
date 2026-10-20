<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-153 — Blame on click can be turned off

**Status:** Done — on `feat/FEAT-153-blame-on-click`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Branch:** `feat/FEAT-153-blame-on-click`.
**Screens:** 1U (Files).
**Raised by:** the author, 2026-10-20: "in files view , make a toggle on top to enable or disable (on line click to see blame or who commit this )".

## Change

- **A *Blame on click* toggle** at the right of the path above the file. On, as before, a click on a line's text opens the card under it that says who wrote the line, when and why. Off, a click only puts the cursor on the line, which suits reading and selecting; the card that was open closes.
- **Enter still opens the card** with the toggle off: the toggle is about the mouse, not about whether the card can be asked for.
- **Kept per repository** with the other view switches (the blame column, wrapping).

## Acceptance criteria

- The toggle shows at the top of a text file and says whether it is on.
- Off, clicking a line opens no card and moves the cursor there; Enter opens the card.
- The choice survives leaving Files and a restart.

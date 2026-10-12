<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-128 — Sweep

**Item:** [FEAT-128](../items/FEAT-128-a-conflicts-columns-scroll-together-or-each-alone.md)

Row 01's heights were measured on 2026-10-12 on a page that copies the card's layout rules, in headless Chrome. That is the rules and not the application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT128-01 | A repository stopped on a conflict whose sides are longer than the window, such as two changelog entries | Open Conflicts | The card fits in the window: its header at the top, its choices at the bottom, above the pill. Each column has a scrollbar | P1 | Measured on the copy: card 448 in a list of 560, columns scrolling. Not run in the application. |
| SWEEP-FEAT128-02 | As above | Scroll one side with the wheel | The result and the other side move with it, reaching their ends together. The header, the headings and the choices stay still | P1 | Not run. |
| SWEEP-FEAT128-03 | As above | Press **Scroll together** in the pill, then scroll one side | Only that column moves | P1 | Not run. |
| SWEEP-FEAT128-04 | As above | Leave Conflicts and come back; restart the application | **Scroll together** is as it was left | P2 | Not run. |
| SWEEP-FEAT128-05 | As above | Scroll a column to its end and keep turning the wheel | The list behind it does not start to move | P2 | Not run. |
| SWEEP-FEAT128-06 | A file with three short conflicts | Open it | Three cards as before, each as tall as its text, the list scrolling between them | P1 | Not run. |
| SWEEP-FEAT128-07 | A long conflict | Press **Base** | The base strip is shown and scrolls on its own; the columns keep most of the card | P2 | Not run. |
| SWEEP-FEAT128-08 | A long conflict, **Edit by hand** | Type many lines | The result column scrolls; the other two follow while together is on | P2 | Not run. |
| SWEEP-FEAT128-09 | A merge with a long conflict, in Merger | Repeat 01 to 03 | The same | P1 | Not run. |
| SWEEP-FEAT128-10 | Any of the above, on macOS and on Linux | Repeat 01 and 02 | The WebKit views lay the card out and link the columns the same way | P1 | Not run. |

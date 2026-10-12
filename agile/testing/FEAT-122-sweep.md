<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-122 — Sweep

**Item:** [FEAT-122](../items/FEAT-122-one-branch-menu-that-can-be-searched.md)

Rows 01 to 04 were looked at on 2026-10-12 in the built frontend against fixtures, drawn by headless Chrome at 1440 by 900, with twenty local branches, four remote ones and four tags. Not the application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT122-01 | A repository with many branches | Press the branch in the toolbar | The list opens above the toolbar, the field has the focus, the current branch is first, ticked and marked `current` | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT122-02 | As above | Type `feat 12` | Only the branches with both words are left, the matches are marked, the field says *2 of 20*, and the list keeps its size | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT122-03 | As above | Type a word nothing has | *Nothing matches* and the word | P2 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT122-04 | Merger open | Press the name on card B | The list is inside the window, opened at the branch B holds, which is ticked; A's branch is listed with *already A* | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT122-05 | The toolbar's list open | Arrow down twice, Enter | The branch under the keys is switched to | P1 | Not run. |
| SWEEP-FEAT122-06 | As above | Press Escape | The list closes and the focus is back on the control | P2 | Not run. |
| SWEEP-FEAT122-07 | Rebase open | Press the branch to replay onto, type, pick | The plan is made for the branch picked | P1 | Not run. |
| SWEEP-FEAT122-08 | A repository with more than a hundred branches | Open the list and scroll to the end | A hundred rows and how many more there are; typing finds one of the others | P2 | Not run. |
| SWEEP-FEAT122-09 | The lightest theme, then the darkest | Open the list | The tick, `current` and the marked match are readable in both | P2 | Seen in the default dark theme only. |
| SWEEP-FEAT122-10 | macOS | Open the list and click a row | The row is picked; the list does not close before the click lands | P1 | Not run. |

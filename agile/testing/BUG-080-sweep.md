<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-080 — Sweep

**Item:** [BUG-080](../items/BUG-080-the-review-pill-runs-out-of-a-narrow-diff.md)

Rows 01 and 03 were measured on 2026-10-12 on a page that copies the room's layout rules, in headless Chrome. That is the rules and not the application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG080-01 | A pull request open in Review, in a window about 1270 wide | Drag the conversation as wide as it goes | The diff stays at least 560 wide; the files and the conversation narrow instead. The pill is inside the diff column, in two rows, with nothing drawn outside it | P1 | Measured on the copy: diff 560, pill inside. Not run in the application. |
| SWEEP-BUG080-02 | As above | Open a file with a long name | The name in the pill ends in an ellipsis; *n of m* under it is whole | P1 | Not run. |
| SWEEP-BUG080-03 | As above | Widen the window | The files and the conversation return to the widths they were dragged to | P2 | Measured on the copy at 1600. Not run in the application. |
| SWEEP-BUG080-04 | As above, with the files hidden (`f`) and the conversation put away | Look at the pill | One row, centred under the diff | P2 | Not run. |
| SWEEP-BUG080-05 | A merge with conflicts in Merger, in a narrow window | Look at the resolver's pill | Inside the conflict column, wrapped onto two rows if it must be | P2 | Not run. |
| SWEEP-BUG080-06 | Any of the above, on macOS and on Linux | Repeat 01 | The WebKit views lay it out the same way | P2 | Not run. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-070 — Sweep

**Item:** [TASK-070](../items/TASK-070-the-plans-file-list-scrolls-in-a-card-the-height-of-its-neighbours.md)

Rows 01 and 03 were looked at on 2026-10-12 in the built frontend, served with fixed answers in place of the core (a forecast of 34 files) and drawn by headless Chrome. That is the real page and not the real application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK070-01 | Two branches that differ in thirty files or more | Open them in Merger, at a window 1440 wide | The three cards under the plan are one height, set by *How it is merged* or *History after*. *What changes* scrolls | P1 | Seen against fixtures: 365 pixels each, the list scrolling. Not run in the application. |
| SWEEP-TASK070-02 | As above | Scroll *What changes* to its end; press a file that conflicts | The last file can be reached. A conflicted file still opens the resolver on it | P1 | Not run. |
| SWEEP-TASK070-03 | As above | Narrow the window until *What changes* wraps under the other two | It is 240 pixels tall and scrolls | P2 | Seen against fixtures at 900 wide. Not run in the application. |
| SWEEP-TASK070-04 | Two branches that differ in two files | Open them in Merger | The card is the height of its neighbours, with the two rows at its top | P2 | Not run. |
| SWEEP-TASK070-05 | Any merge, on macOS and on Linux | Open it in Merger with many files | As 01: the WebKit views lay the card out the same way | P2 | Not run. |

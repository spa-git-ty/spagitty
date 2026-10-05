<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-047 — Manual sweep

**Item:** [`agile/items/BUG-047-whole-file-scrolls-back-to-the-top.md`](../items/BUG-047-whole-file-scrolls-back-to-the-top.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG047-01 | Release build; a pull request touching a file of 150+ lines | 1. Start review 2. Whole file 3. Scroll to the bottom | The view stays where it is scrolled | P1 | |
| SWEEP-BUG047-02 | As 01 | 1. Next file | The next file opens at its top | P1 | |
| SWEEP-BUG047-03 | A pull request with a thread | 1. Click the thread in the Conversation card | The diff jumps to its line, once | P2 | |

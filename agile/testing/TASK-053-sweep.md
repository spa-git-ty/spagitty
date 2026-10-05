<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-053 — Manual sweep

**Item:** [`agile/items/TASK-053-the-review-room-after-first-use.md`](../items/TASK-053-the-review-room-after-first-use.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK053-01 | Release build; a pull request opened in the room | 1. Look at the files list | Each file the author changed says `author`; the chips show counts | P1 | |
| SWEEP-TASK053-02 | As 01 | 1. Read an edited file | One number per line; a removed line's number is quieter | P1 | |
| SWEEP-TASK053-03 | As 01 | 1. Whole file | The file is one card with its changes in place | P1 | |
| SWEEP-TASK053-04 | A pull request with a conflict fix | 1. Whole file | The fix is its own sky-framed card; the rest is one card either side | P2 | |
| SWEEP-TASK053-05 | As 01 | 1. Press Viewed on a file not viewed | It ticks and the next file opens; the ring is hollow there | P1 | |
| SWEEP-TASK053-06 | As 05 | 1. Go back to the ticked file 2. Press Viewed | The ring shows ticked in green; pressing it unticks the file | P1 | |
| SWEEP-TASK053-07 | As 01 | 1. View every file | The pill's button is Finish review; pressing it opens the Finish card | P1 | |

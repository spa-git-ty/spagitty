<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-129 — Sweep

**Item:** [FEAT-129](../items/FEAT-129-review-shows-a-file-before-and-after-side-by-side.md)

None of these has been run. The view was checked by its tests, not in the running application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT129-01 | A pull request open in Review, with a file that has lines changed, added and removed | Press **Before and after** in the pill | The file is shown twice under *Before* and *After*. Removed lines are red on the left, added lines green on the right, changed lines level with each other, and the other side empty beside lines only one file has | P1 | Not run. |
| SWEEP-FEAT129-02 | As above | Read the numbers down each side | Each side counts 1, 2, 3… with none missing or repeated | P1 | Not run. |
| SWEEP-FEAT129-03 | As above | Scroll | Both sides move as one; a long file scrolls smoothly | P1 | Not run. |
| SWEEP-FEAT129-04 | As above | Press `+` on an added line, write, **Add to review**; do the same on a removed line | The box opens under the row, across both sides, and the pending comment stays there. Sent with Finish review, each lands on the right line and side on the host | P1 | Not run. |
| SWEEP-FEAT129-05 | A pull request with an open thread on a changed line | Open it in **Before and after**; press the thread in the Conversation card | The thread is under its row, and the room scrolls to it | P2 | Not run. |
| SWEEP-FEAT129-06 | As 01 | Press `j` and `k`; press a line number on each side | The ruler moves a row at a time, and to the line pressed | P2 | Not run. |
| SWEEP-FEAT129-07 | As 01 | Press **All** | Every file is shown side by side, one after another | P2 | Not run. |
| SWEEP-FEAT129-08 | A pull request that could not be fetched, read from the host's patch | Look at the pill | **Before and after** is disabled, with **Whole file**, and says why | P2 | Not run. |
| SWEEP-FEAT129-09 | A new file, and a deleted one | Open each in **Before and after** | One side is empty the whole way down | P2 | Not run. |
| SWEEP-FEAT129-10 | A pull request with a conflict fix (FEAT-092) | Open that file in **Before and after** | The fix is a card of its own, in its own colour, with its two sides still offered | P2 | Not run. |
| SWEEP-FEAT129-11 | As 01, with the files and the conversation both shown in a window 1280 wide | Look at the two halves | Each is wide enough to read; long lines wrap inside their half | P2 | Not run. |

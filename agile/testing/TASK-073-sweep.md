<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-073 — Sweep

**Item:** [TASK-073](../items/TASK-073-the-sweeps-refreshed-and-run-from-a-portal.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK073-01 | The repository, `bun install` done | `bun run sweep` | The browser opens on the portal; the terminal prints its address | Not run; the author is checking it |
| SWEEP-TASK073-02 | The portal open | Choose a sweep, set a ticket to Passed with a note, Save | The ticket shows Passed; the sweep document has the result in that ticket's cell or line and nowhere else (`git diff`) | Not run |
| SWEEP-TASK073-03 | Same | Take a screenshot, click a ticket, paste | The screenshot shows under the ticket; saving a result links it | Not run |
| SWEEP-TASK073-04 | Same, in Chrome or Edge | *Record the screen*, do something, *Stop recording* | A video is kept and shown under the ticket | Not run |
| SWEEP-TASK073-05 | Same | Drop an image and a video on a ticket | Both are kept | Not run |
| SWEEP-TASK073-06 | Same | Filter by *Failed or blocked*; search for a word in a ticket's steps | The list narrows as it says | Not run |
| SWEEP-TASK073-07 | Light and dark desktop | Open the portal | It follows the desktop | Not run |
| SWEEP-TASK073-08 | A ticket saved | `git status` | The sweep document and `agile/testing/sweeps.md` changed; nothing under `evidence/` is listed | Not run |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-155 — Sweep

**Item:** [FEAT-155](../items/FEAT-155-untrack.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT155-01 | A repository with a file that should not be tracked | Open Untrack from the rail | The checked-out branch is chosen and its files are listed | Not run; the author is checking it |
| SWEEP-FEAT155-02 | Same | Tick the file and a folder | Both are listed on the right with their `.gitignore` lines and a suggested message | Not run |
| SWEEP-FEAT155-03 | Same, nothing else staged | *Untrack, commit and push* | A commit removes them; they are still on disk; `.gitignore` has the lines; the push runs | Not run |
| SWEEP-FEAT155-04 | Another local branch that is not on the remote | Choose it, untrack a file, commit and push | The working copy does not change; the branch is created on the remote with the commit; no login window opens | Not run |
| SWEEP-FEAT155-05 | A change staged on the checked-out branch | Untrack a file | Refused, saying the staged change would go in too | Not run |

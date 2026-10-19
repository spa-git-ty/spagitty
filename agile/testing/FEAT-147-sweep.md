<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-147 — Sweep

**Item:** [FEAT-147](../items/FEAT-147-own-copies-and-intake.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT147-01 | New farm | Step 3, *Where each task works* | Three cards; *A copy per task* is tagged `new`. | P1 | Not run. |
| SWEEP-FEAT147-02 | A copy farm, a task with *May touch* `docs/**` | Run it; open `.spagitty/farm/copies/` | One folder per task; `git remote` in it prints nothing. | P1 | Not run. |
| SWEEP-FEAT147-03 | The same | Let it finish | The farm branch appears in the Graph at the copy's commit; the task is checked and reviewed. | P1 | Not run. |
| SWEEP-FEAT147-04 | A task whose agent edits outside its paths | Let it finish | Blocked, with the path named; the copy is kept. | P1 | Not run. |
| SWEEP-FEAT147-05 | This repository, on Windows | Time a copy against a worktree | No more than twice as long. | P2 | Not run. |

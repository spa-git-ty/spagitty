<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-144 — Sweep

**Item:** [FEAT-144](../items/FEAT-144-the-landing-lane.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT144-01 | A farm at Automatic, a task reviewed and approved | Switch to another branch in the toolbar; let the farm merge | The task lands on its branch; your branch and files do not change. | P1 | Not run. |
| SWEEP-FEAT144-02 | The same, with uncommitted work on the destination | Let it merge | The task waits blocked with git's reason; your changes are untouched. | P1 | Not run. |
| SWEEP-FEAT144-03 | Commit to the destination while a task is in review | Let it merge | The checks run again, *merged with main*; Activity shows them; the land follows. | P1 | Not run. |
| SWEEP-FEAT144-04 | A task that conflicts with the destination | Let it merge | Blocked with *Conflicts with main*; no conflict in your files; Merger resolves it. | P1 | Not run. |
| SWEEP-FEAT144-05 | Lands and a Merger merge | Farm › Activity | *Branch moves*, newest first, naming the task, the agent, the reviewer and old → new. | P2 | Not run. |
| SWEEP-FEAT144-06 | Several lands | Open `.spagitty/farm/ledger.jsonl` | One line per move, in sequence. | P2 | Not run. |

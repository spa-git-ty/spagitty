<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-066 — Sweep

**Item:** [TASK-066](../items/TASK-066-style-notes-from-the-2-0-audit.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK066-01 | Windows; a repository on an ordinary drive | Open it, then commit or switch branch in a terminal | The graph and the status strip follow, as before | Not run |
| SWEEP-TASK066-02 | Windows | `node tools/release-notes.mjs 3.3.0` | Prints 3.3.0's notes | Passed, 2026-10-19 |

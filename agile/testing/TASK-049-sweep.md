<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-049 — Manual sweep

**Item:** [`agile/items/TASK-049-the-record-catches-up-with-what-merged.md`](../items/TASK-049-the-record-catches-up-with-what-merged.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK049-01 | — | 1. For each item closed here, `git merge-base --is-ancestor` its branch against `main` | All ancestors | P1 | Pass, 2026-09-30 |
| SWEEP-TASK049-02 | — | 1. Read the index's Open rows | Each is being worked on, or says on its item what is still owed | P1 | Pass, 2026-09-30 |

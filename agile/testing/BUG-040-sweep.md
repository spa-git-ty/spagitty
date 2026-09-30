<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-040 — Manual sweep

**Item:** [`agile/items/BUG-040-the-shells-pane-class-leaks-into-five-components.md`](../items/BUG-040-the-shells-pane-class-leaks-into-five-components.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG040-01 | A repository with a stash | 1. Stash 2. Select the entry | The diff has no border or corner of its own | P1 | |
| SWEEP-BUG040-02 | No farm yet | 1. Farm | No second panel inside the pane | P1 | |
| SWEEP-BUG040-03 | Changes | 1. Working copy | The hunks sit in the pane, not in a box inside it | P1 | |
| SWEEP-BUG040-04 | A conflicted merge | 1. Conflicts | The three sides keep their own dividers, no shadowed boxes | P2 | |
| SWEEP-BUG040-05 | Any repository | 1. Graph | The pane itself is unchanged: its edge, corner and shadow | P1 | |

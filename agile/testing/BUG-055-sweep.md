<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-055 — Manual sweep

**Item:** [`agile/items/BUG-055-branch-where-you-point.md`](../items/BUG-055-branch-where-you-point.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG055-01 | Graph, nothing selected | 1. Branch 2. Type 3. Enter 4. OK | Field at HEAD; asked; branch made and checked out | P1 | |
| SWEEP-BUG055-02 | Graph, an older commit clicked | 1. Branch | Field in that commit's row | P1 | |
| SWEEP-BUG055-03 | A changed file | 1. Stash 2. Edit message 3. Stash now | Stashed with that message; working copy clean | P1 | |
| SWEEP-BUG055-04 | Spagitty open | 1. Edit a tracked file in another editor | Rail count and the graph's uncommitted row update within a second | P1 | |
| SWEEP-BUG055-05 | As 04 | 1. Run a build writing ignored files | No flicker of refreshes | P2 | |
| SWEEP-BUG055-06 | No account for the host | 1. Review → All my repos | *No account is connected*, centred | P2 | |

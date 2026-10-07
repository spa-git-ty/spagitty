<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-104 — Manual sweep

**Item:** [`agile/items/FEAT-104-a-branch-named-where-head-is.md`](../items/FEAT-104-a-branch-named-where-head-is.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT104-01 | On Merger or another screen | 1. Click Branch in the bottom bar | The graph, with a focused name field in HEAD's row | P1 | |
| SWEEP-FEAT104-02 | As 01 | 1. Type `my topic` 2. Enter | `my-topic` is created at HEAD and checked out | P1 | |
| SWEEP-FEAT104-03 | As 01 | 1. Escape, or click elsewhere | The field goes; nothing is created | P2 | |
| SWEEP-FEAT104-04 | Graph | 1. Right-click an older commit → Create branch here | The field opens in that commit's row | P2 | |

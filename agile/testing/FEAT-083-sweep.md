<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-083 — Manual sweep

**Item:** [`agile/items/FEAT-083-the-content-in-the-spatial-language.md`](../items/FEAT-083-the-content-in-the-spatial-language.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT083-01 | A repository with branches | 1. Graph screen | No header bar, no column rules, no graph band; chips are capsules, tags half-capsules | P1 | |
| SWEEP-FEAT083-02 | As -01 | 1. Hover rows 2. Click one 3. Shift-click another | Rounded inset highlights; the lanes and nodes do not move | P1 | |
| SWEEP-FEAT083-03 | Columns wider than the window | 1. Scroll the graph sideways | It passes under the messages, with the seam's shadow; at rest, no seam | P1 | |
| SWEEP-FEAT083-04 | Uncommitted changes | 1. Look above the newest commit | The working copy is a card; clicking it opens Working copy | P2 | |
| SWEEP-FEAT083-05 | Detail panel shown | 1. Select a commit 2. Drag its splitter | A card with its own corner; resizing moves its edge | P1 | |
| SWEEP-FEAT083-06 | Every screen | 1. Visit each | No filled header bars; nothing shows through a header | P2 | |

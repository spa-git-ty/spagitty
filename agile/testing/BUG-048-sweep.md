<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-048 — Manual sweep

**Item:** [`agile/items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md`](../items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG048-01 | Release build; `git/git` open on Graph, by date | 1. Drag the Graph column's right edge well to the right | Lanes fill the wider column up to its edge; the lanes on the left do not move | P1 | |
| SWEEP-BUG048-02 | As 01 | 1. Drag the edge back to the left, slowly | Lanes fold onto the edge one by one, with no jump | P1 | |
| SWEEP-BUG048-03 | A repository with two or three lanes | 1. Widen the Graph column | The lanes stay where they are | P2 | |
| SWEEP-BUG048-04 | Settings › Appearance › Graph: Compact; `git/git` | 1. Reset the column width 2. Widen it | Undragged it looks as before; widened, more lanes appear | P2 | |

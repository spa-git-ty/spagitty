<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-041 — Manual sweep

**Item:** [`agile/items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md`](../items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG041-01 | A commit selected, detail open | 1. Look beside the card's top-left corner | No pale strip; one tint | P1 | Pass, 2026-09-30, rule injected into the release build |
| SWEEP-BUG041-02 | As -01 | 1. Scroll the graph sideways | The graph passes under the pinned header, with the seam | P1 | |

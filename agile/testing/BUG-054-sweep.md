<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-054 — Manual sweep

**Item:** [`agile/items/BUG-054-review-loader-conversation-panel-and-scrollbars.md`](../items/BUG-054-review-loader-conversation-panel-and-scrollbars.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG054-01 | A forge account | 1. Open Review 2. Start a review | The main loader both times | P1 | |
| SWEEP-BUG054-02 | A narrow Conversation card | 1. Look at its header | Chips wrap inside the card | P1 | |
| SWEEP-BUG054-03 | As 02 | 1. Hide it 2. Bring it back 3. Restart | Tab at the edge; card back; the choice survives | P2 | |
| SWEEP-BUG054-04 | Any long list | 1. Rest 2. Scroll 3. Stop | No bar at rest; bar while scrolling, gone a moment after | P1 | |

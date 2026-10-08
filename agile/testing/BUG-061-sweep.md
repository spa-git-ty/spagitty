<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-061 — Sweep

**Item:** [BUG-061](../items/BUG-061-merger-shows-the-previous-pairs-forecast.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG061-01 | Merger open on a pair with a conflict | Choose another B | The result card says it is working out the merge until the new plan lands; no *Resolve* for the old pair | P1 | Not run. |
| SWEEP-BUG061-02 | Merger open | Change the strategy, then the direction | The plan follows at once, with no loader | P2 | Not run. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-060 — Sweep

**Item:** [BUG-060](../items/BUG-060-merger-forecast-lost-on-first-open.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG060-01 | Windows 11, a fresh launch | Open Merger | The forecast shows within the backend's own time | P1 | Not run. Needs Windows. |
| SWEEP-BUG060-02 | Merger open | Choose another B, several times | Each forecast lands | P1 | Not run. |
| SWEEP-BUG060-03 | Merger open | Commit to B from a terminal | The forecast is asked for again and follows | P2 | Not run. |

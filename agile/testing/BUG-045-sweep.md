<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-045 — Manual sweep

**Item:** [`agile/items/BUG-045-the-name-leaves-the-title-bar.md`](../items/BUG-045-the-name-leaves-the-title-bar.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG045-01 | Release build; no repository open | 1. Look at the title bar | Mark and "spagitty" centred | P1 | |
| SWEEP-BUG045-02 | Release build | 1. Open a repository | The tab appears on the left; the name stays centred | P1 | |
| SWEEP-BUG045-03 | Release build | 1. Open repositories until the tabs reach the name | The tabs scroll sideways; nothing overlaps the name; the window still drags by the empty part of the bar | P2 | |

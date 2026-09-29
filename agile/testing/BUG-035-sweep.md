<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-035 — Manual sweep

**Item:** [`agile/items/BUG-035-native-scrollbars-and-a-smudged-tab.md`](../items/BUG-035-native-scrollbars-and-a-smudged-tab.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG035-01 | Windows, a wide diff | 1. Working copy 2. Look at the diff's bottom right | Thin rounded thumbs, no arrows, no square corner | P1 | |
| SWEEP-BUG035-02 | Two repositories open | 1. Look at the tab strip | The open tab is a pill with an edge and no shadow | P2 | |
| SWEEP-BUG035-03 | Linux | 1. Scroll any list | Unchanged | P2 | |

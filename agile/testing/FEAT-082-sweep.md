<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-082 — Manual sweep

**Item:** [`agile/items/FEAT-082-a-spatial-shell.md`](../items/FEAT-082-a-spatial-shell.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT082-01 | Two repositories open | 1. Look at the window | One row of tab pills above; rail pill, pane; toolbar pill centred below with state left and licence right | P1 | |
| SWEEP-FEAT082-02 | Any | 1. Hover the rail and rest 2. Move off | It widens over the pane with labels and counts, and closes at once; the pane does not move | P1 | |
| SWEEP-FEAT082-03 | Any | 1. Tab into the rail with the keyboard | It opens while focused | P1 | |
| SWEEP-FEAT082-04 | Graph screen | 1. Scroll, select, resize the graph column | Exactly as before FEAT-082 | P1 | |
| SWEEP-FEAT082-05 | Each palette family, light and dark | 1. Settings → Appearance, cycle | Environment lit in the family's own hues; text legible on every ornament | P2 | |
| SWEEP-FEAT082-06 | 900px wide | 1. Narrow the window | Toolbar labels go, icons stay; counts go before the licence | P2 | |
| SWEEP-FEAT082-07 | Linux, software rendering | 1. Scroll the graph with the rail open and closed | No frame-time regression against 0.8.1 | P1 | |
| SWEEP-FEAT082-08 | Windows and macOS | 1. Maximize and restore | Corners and shadow follow the window state as before | P2 | |

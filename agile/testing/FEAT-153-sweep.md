<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-153 — Sweep

**Item:** [FEAT-153](../items/FEAT-153-blame-on-click-can-be-turned-off.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT153-01 | A text file open in Files | Look at the row above the file | *Blame on click* at the right of the path, highlighted | Not run; the author is checking it |
| SWEEP-FEAT153-02 | Same, toggle on | Click a line | The card says who wrote it, when and why | Not run |
| SWEEP-FEAT153-03 | Same | Turn the toggle off, click a few lines, drag to select | No card; the cursor follows the click; the selection works | Not run |
| SWEEP-FEAT153-04 | Toggle off | Press Enter | The card opens for the cursor's line | Not run |
| SWEEP-FEAT153-05 | Toggle off | Restart Spagitty and open Files | The toggle is still off | Not run |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-152 — Sweep

**Item:** [FEAT-152](../items/FEAT-152-the-files-tree-widens-and-copies.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT152-01 | A repository open in Files | Drag the edge between the tree and the file | The tree widens and narrows; long names show in full | Not run; the author is checking it |
| SWEEP-FEAT152-02 | Same | Restart, then double-click the edge | The width is kept; the double-click puts it back to 240 px | Not run |
| SWEEP-FEAT152-03 | Same | Right-click a file, try each copy, paste each | Its name; its path from the top of the repository; its text at the revision shown | Not run |
| SWEEP-FEAT152-04 | Same | Right-click a folder | Name and path only | Not run |
| SWEEP-FEAT152-05 | A file of a few thousand lines open | Select from the top, drag or scroll to the bottom, copy, paste | Every line in between | Not run |

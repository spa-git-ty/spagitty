<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-154 — Sweep

**Item:** [FEAT-154](../items/FEAT-154-usages-of-a-name.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT154-01 | A Rust or TypeScript repository open in Files, a source file open | Hold Ctrl (⌘ on a Mac) and move over the code | Names underline under the pointer | Not run; the author is checking it |
| SWEEP-FEAT154-02 | Same | Ctrl-click a function's name | The tree shows *Usages*: Defined, Used, and Mentions folded, with counts | Not run |
| SWEEP-FEAT154-03 | Same | Click a place under *Used* | The file opens at that line with the name marked | Not run |
| SWEEP-FEAT154-04 | Same | Unfold *Mentions* | Comments, strings and documents that name it | Not run |
| SWEEP-FEAT154-05 | Same | Type another name in the *Usages* field, then Enter | Its usages | Not run |
| SWEEP-FEAT154-06 | A large repository | Ctrl-click a common name | The list comes back in a few seconds; past 2,000 places it says there are more | Not run |
| SWEEP-FEAT154-07 | Usages shown | Change the revision in Files | The usages are read again at the new revision | Not run |

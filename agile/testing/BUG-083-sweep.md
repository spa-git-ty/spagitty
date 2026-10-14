<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-083 — Sweep

**Item:** [BUG-083](../items/BUG-083-hiding-the-blame-in-files-breaks-every-line.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG083-01 | Files, a text file open, blame shown | Press **Blame** or `b` | Numbers in their column, the code beside them at full width | Mounted test; the author is checking the desktop app |
| SWEEP-BUG083-02 | Same | Drag the blame's edge right, then left; restart | The blame follows the pointer and keeps its width | Mounted test; the author is checking the desktop app |
| SWEEP-BUG083-03 | Graph, a commit chosen | Right-click a file in its detail → **Open in Files** | Files at that commit, on that file | Mounted test of the menu; the author is checking the desktop app |
| SWEEP-BUG083-04 | Diff of a commit that deleted a file | Right-click the deleted file → **Open in Files** | Files at the commit's parent, on that file | Mounted test of the revision; the author is checking the desktop app |
| SWEEP-BUG083-05 | Working copy with a modified and an untracked file | Right-click each | The modified one opens at the branch; the untracked one says *Not in a commit yet* | Mounted test of the reason; the author is checking the desktop app |
| SWEEP-BUG083-06 | Review room, a pull request fetched | Right-click a file in the list and in the diff | Files at the fetched head | Not run |

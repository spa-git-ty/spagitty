<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-068 — Sweep

**Item:** [BUG-068](../items/BUG-068-ordinary-commits-marked-as-conflict-fixes.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG068-01 | The pull request the author saw *conflict fix* on | Open it in Review and look at the file list | Its ordinary commits carry no *conflict fix* | P1 | Not run. |
| SWEEP-BUG068-02 | A pull request that merged `main`, resolved a real conflict and then committed normally | Open it in Review | Only the files with the resolution carry *conflict fix* | P1 | Not run. |

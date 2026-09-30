<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-033 — Manual sweep

**Item:** [`agile/items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md`](../items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG033-01 | Windows, a repository checked out with `core.autocrlf=true` | 1. Change one line of a text file 2. Working copy | One hunk, one line removed and one added | P1 | |
| SWEEP-BUG033-02 | As -01, two separate changes | 1. Stage one hunk 2. Look at Staged | Only that hunk is staged | P1 | |
| SWEEP-BUG033-03 | As -02 | 1. Discard the other hunk 2. Open the file in an editor | That change is gone; the file still has CRLF endings | P1 | |
| SWEEP-BUG033-04 | Linux or macOS, any repository | 1. Change, stage and discard lines | Exactly as before | P2 | |

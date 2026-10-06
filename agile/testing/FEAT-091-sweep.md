<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-091 — Manual sweep

**Item:** [`agile/items/FEAT-091-the-review-room.md`](../items/FEAT-091-the-review-room.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT091-01 | — | 1. The room with the handoff's sample pull request | Three columns, the pill over the diff's foot, the ruler on the first change, threads under their lines | P1 | Pass, 2026-10-04, dev server in headless Chrome with the backend answered by fixtures |
| SWEEP-FEAT091-02 | A GitHub pull request with some files ticked | 1. Review → Continue review | Lands on the first file not ticked | P1 | |
| SWEEP-FEAT091-03 | The author's work GitLab, an open merge request | 1. Open it 2. Whole file 3. All | Every file reads; the same as on GitHub | P1 | |
| SWEEP-FEAT091-04 | As 02 | 1. Tick a file 2. The author pushes a change to it 3. Refresh, open again | That file is unticked; the others stay ticked | P1 | |
| SWEEP-FEAT091-05 | A pull request of 300 files or more | 1. All 2. Scroll top to bottom | Smooth; the line being read does not jump as files arrive | P1 | |
| SWEEP-FEAT091-06 | As 02 | 1. `j` `j` `k` 2. Click a line number | The band follows; Whole chunk in Settings fades the other chunks | P2 | |
| SWEEP-FEAT091-07 | No network | 1. Open a pull request whose head was never fetched | The reason, and nothing hangs | P2 | |
| SWEEP-FEAT091-08 | Light and dark | 1. The room in each | The cards, tints and ruler read in both | P2 | |

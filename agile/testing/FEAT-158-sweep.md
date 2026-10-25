<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-158 — Sweep

**Item:** [FEAT-158](../items/FEAT-158-type-to-search-any-list.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT158-01 | Pull requests with several open | Type `fix` without clicking anything | The pill shows `fix` and a count; only matching pull requests remain; Enter opens the first | Not run; the author is checking it |
| SWEEP-FEAT158-02 | Graph | Type part of a branch name, then press ↓ and ↑ | The selection jumps to the first match, the rest are dimmed, the arrows step between matches; Escape undims | Not run |
| SWEEP-FEAT158-03 | Branches | Type `fb` | The letters land in *Filter by name*, which keeps the focus | Not run |
| SWEEP-FEAT158-04 | Stash, with a diff open | Press `j`, then type `w`, then `j` | The first `j` steps a hunk; after `w` the search reads `wj` | Not run |
| SWEEP-FEAT158-05 | Working copy, commit message focused | Type a word | It goes into the message; no search opens | Not run |
| SWEEP-FEAT158-06 | Terminal | Type in the shell; then click the sessions panel and type | The shell gets the letters; after the click the sessions narrow | Not run |
| SWEEP-FEAT158-07 | Worktrees dialog open | Type a branch name | Only the dialog's list narrows | Not run |
| SWEEP-FEAT158-08 | Any list being searched | Click a match, then click outside the list | The search stays after the first click and ends after the second | Not run |
| SWEEP-FEAT158-09 | Light and dark, each accent | Search any list | The pill and the marks are readable | Not run |

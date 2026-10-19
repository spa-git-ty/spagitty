<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-151 — Sweep

**Item:** [FEAT-151](../items/FEAT-151-search-the-text-of-files.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT151-01 | A repository open in Files | *Text*, type a word used in several files | Lines listed under their files, the word marked, a count of lines and files | Not run; the author is checking it |
| SWEEP-FEAT151-02 | Same | Click a line | The file opens at that line with the word marked; `n` goes to the next one in the file | Not run |
| SWEEP-FEAT151-03 | Same | Turn on `.*` and type `fn \w+\(` | Lines matching the expression; type `(` alone and it says it is not a pattern | Not run |
| SWEEP-FEAT151-04 | Same | Turn on `W`, search `total` where `subtotal` also appears | Only `total` on its own | Not run |
| SWEEP-FEAT151-05 | Same | Search `todo`, then `TODO`, then `todo` with `Aa` on | Any case; only capitals; only lower case | Not run |
| SWEEP-FEAT151-06 | Same | Change the revision with the search on screen | The results are the other revision's | Not run |
| SWEEP-FEAT151-07 | A file open | `/`, type a regex with `.*` on, Enter, `n`, `N` | It moves through the matches both ways | Not run |
| SWEEP-FEAT151-08 | A large repository | Search a common word | It answers in a few seconds and says when there are more than 2,000 lines | Not run |
| SWEEP-FEAT151-09 | Light and dark | Look at the results and the toggles | Marks and toggles read in both | Not run |

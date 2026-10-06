<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-102 — Manual sweep

**Item:** [`agile/items/FEAT-102-resolve-every-conflict-every-way.md`](../items/FEAT-102-resolve-every-conflict-every-way.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT102-01 | Two branches with several conflicts in several files | 1. Merger 2. Resolve N conflicts | The first file's conflicts in three columns, each side by its own numbers, with why and the commits; the rail's Merger icon has a red dot | P1 | |
| SWEEP-FEAT102-02 | As 01 | 1. Take A, Take B, Both each way, on different conflicts | The result and its badges follow; later conflicts in the file renumber | P1 | |
| SWEEP-FEAT102-03 | As 01 | 1. Pick lines 2. Untick a line on each side | The unticked lines fade; the result is A's ticked lines then B's | P1 | |
| SWEEP-FEAT102-04 | As 01 | 1. Edit by hand 2. Type | The box starts from the result or both sides; what is typed is what lands | P1 | |
| SWEEP-FEAT102-05 | As 01 | 1. Base | The base's lines above each conflict; *nothing* where both added | P2 | |
| SWEEP-FEAT102-06 | As 01 | 1. Pill: previous, next, Take, Next unresolved | Steps across files; Next unresolved finds the next; it reads All resolved at the end | P2 | |
| SWEEP-FEAT102-07 | As 01, some resolved | 1. Leave for Graph 2. Back to Merger, Resolve | The choices are as left | P1 | |
| SWEEP-FEAT102-08 | As 01, all resolved | 1. Complete merge 2. Read the dialog 3. Create merge commit | Every conflict listed with its choice; the merge lands with the chosen text; `git show` agrees | P1 | |
| SWEEP-FEAT102-09 | As 01 | 1. Resolve some 2. Abort, confirm | Back to the plan; `git status` and every ref unchanged | P1 | |
| SWEEP-FEAT102-10 | A `git merge` stopped on conflicts | 1. Conflicts 2. Resolve a file every way 3. Mark resolved 4. Continue | The file is written and staged only on Mark resolved; Continue finishes the merge | P1 | |
| SWEEP-FEAT102-11 | As 10, a file deleted on one side | 1. Take the side that deleted it, Mark resolved | The file is removed and resolved | P2 | |
| SWEEP-FEAT102-12 | Light and dark, two palette families | 1. A resolved card in each | A's blue, B's amber and ✎ purple read; function names on A's rows and numbers on B's are distinct from the tint | P2 | |
| SWEEP-FEAT102-13 | Keyboard only | 1. Tab through a card, choose, tick, type | Everything is reachable and works | P2 | |

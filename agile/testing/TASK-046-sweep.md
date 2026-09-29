<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-046 — Manual sweep

**Item:** [`agile/items/TASK-046-lists-you-can-read.md`](../items/TASK-046-lists-you-can-read.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK046-01 | Changes in deep folders | 1. Working copy | Each row starts with the file's name; the folder follows and is what is cut | P1 | |
| SWEEP-TASK046-02 | As -01 | 1. Hover a row 2. Tab to it 3. Select it | Stage and discard appear each time; not otherwise | P1 | |
| SWEEP-TASK046-03 | One of each change | 1. Look at the badges | M amber, A and U green, D red, R blue | P2 | |
| SWEEP-TASK046-04 | A commit selected | 1. Cherry-pick 2. Cancel 3. Revert 4. Cancel | Each asks first and changes nothing when cancelled | P1 | |
| SWEEP-TASK046-05 | A stash with a message | 1. Stash screen | The message leads the row; no "On <branch>:" prefix; no count after the tabs | P2 | |
| SWEEP-TASK046-06 | A repository with a long history | 1. Open it | The strip says "Repository ready" once commits show | P1 | |

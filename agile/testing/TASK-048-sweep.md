<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-048 — Manual sweep

**Item:** [`agile/items/TASK-048-commands-off-the-main-thread.md`](../items/TASK-048-commands-off-the-main-thread.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK048-01 | A large repository (tens of thousands of commits, a big working tree), release build | 1. Open it 2. Move the window and hover the rail while it opens | The window keeps painting; the repository opens | P1 | |
| SWEEP-TASK048-02 | As -01 | 1. Select commits quickly 2. Open a large commit's diff | Selection and hover keep up; no frozen frame | P1 | |
| SWEEP-TASK048-03 | Two tabs, one a large repository | 1. Click the large tab 2. At once click the other | The second is open, on screen and in the status line | P1 | |
| SWEEP-TASK048-04 | One tab, a large repository | 1. Click its tab 2. At once close it | Nothing is open | P2 | |
| SWEEP-TASK048-05 | Any repository | 1. Stage, unstage, commit, stash, checkout | Each works as before | P1 | |
| SWEEP-TASK048-06 | Settings | 1. Change several settings quickly 2. Restart | The last values are kept | P2 | |

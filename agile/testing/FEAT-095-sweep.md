<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-095 — Manual sweep

**Item:** [`agile/items/FEAT-095-check-out-a-pull-request.md`](../items/FEAT-095-check-out-a-pull-request.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT095-01 | Release build; a pull request whose branch is not here; clean working copy | 1. Review 2. Choose it 3. Check out branch | "On <branch>"; the toolbar shows that branch; Graph shows its head checked out | P1 | |
| SWEEP-FEAT095-02 | As 01, a pull request from the same repository | 1. Check out branch 2. Pull | The branch follows the remote's; the pull brings the author's later commits | P1 | |
| SWEEP-FEAT095-03 | A local branch with the pull request's name, elsewhere | 1. Check out branch | "On pr-N"; your branch is where it was | P1 | |
| SWEEP-FEAT095-04 | An uncommitted edit to a file the pull request changes | 1. Check out branch | Git's message says what would be overwritten; nothing changed | P1 | |
| SWEEP-FEAT095-05 | The review room | 1. Check out branch in the header | As 01 | P2 | |

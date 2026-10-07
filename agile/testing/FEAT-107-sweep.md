<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-107 — Manual sweep

**Item:** [`agile/items/FEAT-107-hooks-you-can-see-and-skip.md`](../items/FEAT-107-hooks-you-can-see-and-skip.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT107-01 | A repository using Husky | 1. Settings → Hooks | Husky named; each hook opens to `.husky/<name>`'s script | P1 | |
| SWEEP-FEAT107-02 | As 01, something staged | 1. Commit 2. Run hooks | The log window streams the hooks' output; Passed · committed | P1 | |
| SWEEP-FEAT107-03 | A failing pre-commit | 1. Commit 2. Run hooks | Failed · nothing committed, output kept, message kept | P1 | |
| SWEEP-FEAT107-04 | As 02 | 1. Commit 2. Skip hooks | Committed; no hook ran (post-commit included) | P1 | |
| SWEEP-FEAT107-05 | As 02 | 1. skip hooks chip 2. Commit | No question; committed without hooks | P2 | |
| SWEEP-FEAT107-06 | As 02 | 1. Switch hooks off in Settings 2. Commit | No question, no hooks; `.git/config` has `spagitty.hooks = false` | P1 | |

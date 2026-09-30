<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-038 — Manual sweep

**Item:** [`agile/items/BUG-038-interactive-rebase-cannot-start-on-windows.md`](../items/BUG-038-interactive-rebase-cannot-start-on-windows.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG038-01 | Windows, a branch three commits ahead of main | 1. Rebase onto main 2. Squash two, drop one 3. Start | The branch is rewritten as planned | P1 | |
| SWEEP-BUG038-02 | Windows, a plan that conflicts | 1. Start 2. Resolve 3. Continue | Stops on the conflict; continues after resolving | P1 | |
| SWEEP-BUG038-03 | As -02 | 1. Start 2. Abort | The branch is back where it was | P2 | |
| SWEEP-BUG038-04 | Linux or macOS | 1. As -01 | Unchanged | P2 | |

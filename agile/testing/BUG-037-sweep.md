<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-037 — Manual sweep

**Item:** [`agile/items/BUG-037-core-tests-read-the-machines-git-config.md`](../items/BUG-037-core-tests-read-the-machines-git-config.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG037-01 | Windows, Git for Windows defaults | 1. `cargo test -p spagitty-core` | No line-ending failures | P1 | Pass, 2026-09-30 |
| SWEEP-BUG037-02 | Linux | 1. `cargo test --workspace` | All pass | P1 | Pass, 2026-09-30, WSL |

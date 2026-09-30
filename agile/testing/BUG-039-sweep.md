<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-039 — Manual sweep

**Item:** [`agile/items/BUG-039-a-graph-test-that-reads-the-clock.md`](../items/BUG-039-a-graph-test-that-reads-the-clock.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG039-01 | Windows | 1. `cargo test -p spagitty-core` three times | 536 of 536 each time | P1 | Pass, 2026-09-30 |
| SWEEP-BUG039-02 | Linux | 1. `cargo test --workspace` | All pass | P1 | Pass, 2026-09-30, WSL |

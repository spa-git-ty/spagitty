<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-032 — Manual sweep

**Item:** [`agile/items/BUG-032-a-path-test-that-only-passes-on-unix.md`](../items/BUG-032-a-path-test-that-only-passes-on-unix.md)

A sweep of a test fix is a run of the suite.

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG032-01 | A checkout of this branch on Windows | 1. `bun run test` | Green | P1 | |
| SWEEP-BUG032-02 | A checkout of this branch on Linux | 1. `bun run test` | Green | P1 | |

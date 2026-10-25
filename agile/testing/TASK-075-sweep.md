<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-075 — Sweep

**Item:** [TASK-075](../items/TASK-075-remove-coderabbit.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK075-01 | 3.10.0 installed over 3.9.0 | Open Settings › Extensions | No CodeRabbit, no Official extension | Not run; the author is checking it |
| SWEEP-TASK075-02 | A farm | Open its rules, then a task in review | No CodeRabbit rule, no CodeRabbit button | Not run |
| SWEEP-TASK075-03 | A farm saved in 3.9.0 with the extra review Required | Merge a reviewed task | It merges; nothing waits for CodeRabbit | Not run |
| SWEEP-TASK075-04 | A clean checkout | `bun run tauri dev`, then a release build | Both start without staging an extension | Not run |

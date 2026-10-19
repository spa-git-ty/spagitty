<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-145 — Sweep

**Item:** [FEAT-145](../items/FEAT-145-history-as-tools-for-agents.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT145-01 | A release build | Run `spagitty --history-mcp <repo>` and type an `initialize` line | One JSON line back, `serverInfo.name` is `spagitty`; no window opens. | P1 | Not run. |
| SWEEP-FEAT145-02 | A farm with Claude Code | Run a task whose prompt asks why a line exists | The transcript shows `mcp__spagitty__blame` called without a permission prompt. | P1 | Not run. |
| SWEEP-FEAT145-03 | A farm with Codex | The same | Codex lists and calls the `spagitty` tools. | P1 | Not run. |
| SWEEP-FEAT145-04 | After 02 | Look in `.spagitty/farm/history/` | A folder named by the commit, one file per question. | P2 | Not run. |

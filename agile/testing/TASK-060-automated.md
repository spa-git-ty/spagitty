<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-060 — Automated test record

**Item:** [TASK-060](../items/TASK-060-agent-permissions-with-standard-names.md)

| Suite | Cases |
| --- | --- |
| `src/lib/agents/levels.test.ts` | The four labels and short labels; the stored values are unchanged; each sentence per job; the maximum still caps a level. |
| `src/lib/agents/AgentCard.test.ts`, `AgentsSection.test.ts`, `sheets.test.ts` | The menu, the repository's *Maximum permission* row and the Assign popover show the new names, and a level above the maximum says why. |
| `src/routes/review/room.test.ts`, `src/routes/merge/resolve.test.ts` | The Assign popover's sentence and the Merger's *applied before you merged* wording. |
| `crates/spagitty-farm/src/assign/level.rs` | The level tests still pass with the new labels. |

## Results — 2026-10-11

- `bunx vitest run`: 3,757 passed across 179 files. `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-farm level`: 14 passed.

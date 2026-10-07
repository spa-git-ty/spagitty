<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-111 — Automated test record

**Item:** [FEAT-111](../items/FEAT-111-farm-plan-journey.md)

| Suite | Cases |
| --- | --- |
| `src/lib/farm/journey.test.ts` | Planner line kinds and counts, what an agent is good at, Worth a look (longest wait, overlapping paths, no false overlap on a shared prefix). |
| `src/routes/farm/page.test.ts` | Setup creates, configures, then plans, in that order; Enter in the goal plans and does nothing while empty; no goal disables both buttons; missing agents on one quiet line; AGENTS.md offered only when absent; planning output and Stop planning cancel only the planner; accept kept, discard the rest, then start; an omitted prerequisite blocks starting; a failed accept starts nothing; a plan with no checks says so. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

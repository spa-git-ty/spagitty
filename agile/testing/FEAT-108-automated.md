<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-108 — Automated test record

**Item:** [FEAT-108](../items/FEAT-108-farm-shared-journey.md)

| Suite | Cases |
| --- | --- |
| `src/lib/farm/journey.test.ts` | Six-step track for every status, ready-to-land only with real checks and approval, waves with cycles and missing prerequisites, phase for each farm state, monograms and lane colours, the now sentence (a reviewed task waiting on a merge counted once), relative and elapsed time in the board’s words. |
| `src/lib/ui/flat.test.ts` | Every token a Farm component reads exists; `style:--agent-colour` counts as a declaration. |
| `src/routes/farm/page.test.ts` | Five phases in the header and a six-step track on each card. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

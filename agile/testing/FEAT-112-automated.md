<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-112 — Automated test record

**Item:** [FEAT-112](../items/FEAT-112-farm-crew-activity.md)

| Suite | Cases |
| --- | --- |
| `src/lib/farm/journey.test.ts` | A run whose checks failed before the next attempt is striped; review runs never are; event filters hide transcript lines. |
| `src/routes/farm/page.test.ts` | Crew detection, missing agents named, custom CLI saved; Activity Failures filter and task links. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

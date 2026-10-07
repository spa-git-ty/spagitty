<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-110 — Automated test record

**Item:** [FEAT-110](../items/FEAT-110-farm-task-journey.md)

| Suite | Cases |
| --- | --- |
| `src/lib/farm/journey.test.ts` | Stepper heading per state, transcript line colours, who asked. |
| `src/routes/farm/page.test.ts` | Deep link without a reload; full stepper and Stop; a stuck task opens on Checks and retries with a free agent; a ready task opens on Review even when its evidence arrives after the screen, and a picked tab stays; real change statistics; delete asks first. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

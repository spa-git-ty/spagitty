<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-109 — Automated test record

**Item:** [FEAT-109](../items/FEAT-109-farm-building.md)

| Suite | Cases |
| --- | --- |
| `src/lib/farm/journey.test.ts` | Bar segments, commit and line counts, the stuck line (hand-off first, then the failing check’s last line), the running check, what a free agent takes next, quiet runs after three minutes and never a finished one. |
| `src/lib/farm/store.test.ts` | `prime` opens nothing without a farm on disk, opens one that has a farm, never replaces a farm with a run in flight, and treats a failed look as no farm. |
| `src/routes/farm/page.test.ts` | Dependency waits stay Up next and failures go to Needs you; Land it only for checked and approved work, never unverified; a refused merge restores the controls; pause leaves agents running; resume; containers expand; cancelled work hidden until asked; Rules saves once, keeps a rejected sheet open, and grants merge with Automatic; new tasks through the editor. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

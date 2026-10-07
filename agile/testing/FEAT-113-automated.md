<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-113 — Automated test record

**Item:** [FEAT-113](../items/FEAT-113-farm-wrap-up.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/src/workspace/cleanup.rs` | Merged farm branches are swept after their worktree; a live task’s branch is kept; an unmerged clean worktree and its branch survive; uncommitted work survives. |
| `crates/spagitty-farm/src/model/event.rs` | Merge events written before the hash was recorded still load. |
| `crates/spagitty-core/src/compare.rs` | Task stats count the range and real line changes, and still work once the merged branch is gone. |
| `src/routes/farm/page.test.ts` | Wrap up shows hashes, reads every hand-off, and Start a new goal returns to Setup. |

## Results — 2026-10-07

- `bun run test`: 3,526 passed across 169 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.17%, branches 71.9%, functions 79.68%, lines 85.01%).
- `bun run build`: production build passed.
- `cargo fmt --all -- --check` and `cargo check -p spagitty`: clean.
- `cargo test -p spagitty-farm --lib -- workspace:: model::event`: 45 passed. `cargo test -p spagitty-core --lib compare`: 7 passed.
- `cargo test -p spagitty-farm --lib` natively on Windows: 16 failures, all in `execution::process` (13) and `verification::verifier` (3), whose tests spawn `/bin/sh`, which native Windows does not have. Neither module changed here. The full Rust suite was not run on Linux for this change.

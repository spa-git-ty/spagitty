<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-061 — Automated test record

**Item:** [TASK-061](../items/TASK-061-plain-words-for-merging-and-approval.md)

| Suite | Cases |
| --- | --- |
| `tools/copy.test.ts` | No string or markup text under `src/` says land, lands, landed, landing, gate, gates or gated; `docs/branding.md` does not say lands. |
| Frontend suites that named the old words | The Merger, the commit dialog, Farm, the agent popover and the resolver tests expect the new wording. |
| `crates/spagitty-farm/src/assign/tests.rs` | The end of a merge reads *Merged into main*. |

## Results — 2026-10-11

- `bunx vitest run`: 3,762 passed. `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-farm assign`: 84 passed; `guard::a_path_is_put_back_by_its_name_not_as_a_pattern` fails on Windows as it does on `main`.

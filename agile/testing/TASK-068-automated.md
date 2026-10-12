<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-068 — Automated test record

**Item:** [TASK-068](../items/TASK-068-what-coderabbit-found-on-63-and-64.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/origin.test.ts` | Opened for a Farm task, squash and rebase carry a reason and cannot be chosen, and the direction cannot be turned or swapped. |
| `src/lib/network/store.test.ts` | A push whose done event arrives before its token is still heard: it is no longer running, the wait answers, and the worker is released. |
| `tools/attribution.test.ts` | A *Generated with* or *by* line is refused for ChatGPT, GPT-4o, Codex and Aider; prose about something being generated is not. |
| `crates/spagitty-farm/src/agent/adapters/pi.rs` | Plain `pi`, by any path, gets `--print` alone on an unattended run; Oh My Pi gets `--auto-approve`. |
| `crates/spagitty-core/src/models.rs` | An id Google gave that begins `call-` is sent back on the call and on its answer. |
| `crates/spagitty-extensions/tests/bundled_manifests.rs` | A source manifest that cannot be read fails the test; only the generated bundle may be absent. |

Not covered by a test: the merge limit in `remerge.rs` (twenty-one merges in a fixture), the tooltip, and the two background images.

## Results — 2026-10-12

- `bunx vitest run`: 185 files, 3865 tests, all passed. `bun run check`: zero errors, zero warnings.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p spagitty-core --lib -- remerge models`: 24 passed. `cargo test -p spagitty-farm --lib -- adapters::pi`: 5 passed. `cargo test -p spagitty-extensions --test bundled_manifests`: 2 passed.

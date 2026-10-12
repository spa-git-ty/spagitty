<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-071 — Automated test record

**Item:** [TASK-071](../items/TASK-071-remove-agy.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/src/agent/registry.rs` | A saved registry with an agy definition beside Claude Code's and Codex's reads back as those two. Every provider still has an adapter. |
| `crates/spagitty-farm/src/model/agent.rs` | The slugs of the four built-in providers are unique and safe in a ref. |
| `crates/spagitty-farm/src/agent/adapter.rs`, `execution/process.rs` | A many-line argument to a `.cmd` or `.bat` is still refused before anything starts, shown with `agent.cmd` where it was shown with agy's. |
| `src-tauri/src/agents.rs` | An `agents.json` with `agyAutoApprove` in it loads and keeps Codex's choice. Codex's **Full Access** and Oh My Pi's model reach their own command and no other provider's. |
| `src/lib/agents/AgentsSection.test.ts` | A changed access choice is saved before **Test** can run and the old result is cleared, on Codex's **Full Access**. |
| `src/lib/farm/describe.test.ts` | Every provider has a label, a colour and a monogram. |

Gone with what they tested: agy's command line in print mode and with saved auto-approval, its plan mode for reviews, its permission denial read as a failed step and as a failed Settings test, the two chips in Settings, and the two live tests that needed agy signed in.

## Results — 2026-10-12

On Windows 11.

- `bunx vitest run`: 189 files, 4102 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets`: nothing reported.
- `cargo build -p spagitty`: succeeded.
- `cargo test --workspace --no-fail-fast`: every suite passes but three targets.
  - `spagitty-farm`, its unit tests and `tests/pipeline.rs`: 61 tests fail, each starting `/bin/sh`, which this machine does not have. The same 61, by name, fail on `main` at `ab39d1a`; none is one this change touched, and the ones it did touch pass.
  - `spagitty-extensions`, `tests/bundled_manifests.rs`: one test fails on a CodeRabbit bundle built on 2026-10-08 that is in this working tree and ignored by git. It asks for Spagitty below 2.0.0. A fresh checkout of `main` has no such file and passes.

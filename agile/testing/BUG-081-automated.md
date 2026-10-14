<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-081 — Automated test record

**Item:** [BUG-081](../items/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why.md)

- `src/lib/graph/actions.test.ts`: first push, configured upstream with a different name, fallback remote, missing remote, failed metadata reads, detached HEAD and existing activity refusals.
- `crates/spagitty-core/src/shell.rs`: real first and second pushes set tracking and move the remote; fetch streams progress and updates refs; failing fetch and push expose stderr.
- `src-tauri/src/network_worker.rs`: refusal survives trailing hints; remote errors and unterminated lines are read; successful summaries and fallback messages remain; a real non-fast-forward push emits the reason and leaves the remote unchanged.
- Existing network store and Merger tests verify worker completion, failure notices and release of the network lane.

## Results — 2026-10-12

- `bunx vitest run`: 190 files, 4155 tests passed.
- `bun run check`: zero errors and zero warnings.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace --quiet`: the core suite passed (696 unit tests plus integration tests), and the Tauri suite passed (129 passed, one existing ignored test), including all new push and worker regressions. The run stopped at 17 existing Farm failures on Windows: `/bin/sh` is missing and one fixture uses the invalid Windows filename `*.txt`. The unchanged Farm tests contain these Unix assumptions on the base commit too. The full workspace suite must run in Linux CI.
- `bun run build` and `cargo build --workspace --quiet`: succeeded on Windows.
- Diff reviewed; no unresolved conflicts. No dependency added.

The desktop sweep against a real hosting service has not run.

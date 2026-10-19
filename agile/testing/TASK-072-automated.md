<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-072 — Automated test record

**Item:** [TASK-072](../items/TASK-072-the-declared-minimum-rust-is-older-than-a-dependency-needs.md)

- `tools/rust-version.test.ts`: versions of two and three parts compare as numbers (1.100 is newer than 1.99); a dependency ahead of the workspace is named and the workspace's own crates are not; the root `Cargo.toml` reads 1.90.

## Results — 2026-10-19

- `bunx vitest run tools/rust-version.test.ts`: 4 passed.
- `node tools/rust-version.mjs` on Windows: exit 0 at 1.90; with the line set to 1.89 it names the three tree-sitter crates and exits 1.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL, at 1.90: passed.
- `cargo test --workspace` in WSL: every suite passed (core 938, farm 474, Tauri 197, and the integration suites).

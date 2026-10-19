<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-072 — Plan

**Item:** [TASK-072](../items/TASK-072-the-declared-minimum-rust-is-older-than-a-dependency-needs.md)

1. Find the highest `rust-version` any dependency declares with `cargo metadata`: 1.90, for `tree-sitter`, `tree-sitter-highlight` and `tree-sitter-language`.
2. Raise `[workspace.package] rust-version` from 1.77 to 1.90.
3. Take what clippy then suggests, which is what FEAT-131 met: `as_chunks` for constant `chunks_exact` in `avatars.rs`, `is_none_or` for `map_or(true, …)` in `pipelines`, `spagitty-extensions` and `spagitty-farm`, and `is_multiple_of` in the engine. Applied with `cargo clippy --fix`, then formatted; no behaviour changes.
4. `tools/rust-version.mjs` reads `cargo metadata` and fails, naming them, when a dependency needs a newer Rust than `Cargo.toml` says. Gate 2 runs it after clippy; `tools/rust-version.test.ts` tests the comparison and reads the root manifest.

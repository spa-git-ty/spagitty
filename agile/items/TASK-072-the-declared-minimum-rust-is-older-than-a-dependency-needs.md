<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-072 — The declared minimum Rust is older than a dependency needs

**Status:** Backlog — raised while building FEAT-131, 2026-10-13.
**Screens:** —.
**Raised by:** FEAT-131: `tree-sitter` and `tree-sitter-highlight` 0.27 declare `rust-version = "1.90"`, and the workspace declares `1.77`.

## What is wrong

`Cargo.toml` says the workspace builds with Rust 1.77. Since FEAT-131 it does not: Files' highlighter needs 1.90. Nothing fails, because cargo compares a dependency's `rust-version` with the toolchain in use and not with the workspace's, and every gate builds with `stable`. The line is simply no longer true.

## Why it waits

Raising the line to `1.90` was tried in FEAT-131 and taken back. Clippy reads the workspace's `rust-version` to decide which newer standard-library calls to suggest, and with `-D warnings` the new line made it refuse code in crates FEAT-131 does not touch: `chunks_exact` in `spagitty-core/src/avatars.rs`, `map_or(true, …)` in `spagitty-extensions` and `spagitty-farm`. Those are mechanical, and they are a change of their own.

## Acceptance criteria

- `rust-version` in the workspace is the highest any dependency declares, or higher.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean with it.
- A check, in a gate or a test, fails when a dependency declares a higher `rust-version` than the workspace.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `Cargo.toml` (`[workspace.package]`); the clippy findings name the lines.
- **How to find the highest:** `cargo metadata --format-version 1` gives `rust_version` for every package.

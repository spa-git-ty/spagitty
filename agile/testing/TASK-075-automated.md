<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-075 — Automated test record

**Item:** [TASK-075](../items/TASK-075-remove-coderabbit.md)

- `crates/spagitty-farm/tests/supplemental.rs`: `without_a_provider_a_required_review_does_not_hold_the_merge` is new; the restart test now checks that a reopened farm with a reviewer validates its evidence again, refusing while the reviewer is unavailable.
- `crates/spagitty-extensions/tests/bundled_manifests.rs`: the shipped manifests that remain admit this version.

## Results — 2026-10-25

- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- `cargo test -p spagitty-farm --test supplemental`: 15 passed. `cargo test -p spagitty-extensions`: passed.
- `cargo test -p spagitty-farm --lib`: the `execution::process` tests need `/bin/sh` and cannot run on this Windows machine; CI runs them on Linux.
- `bun run check`: 0 errors, 0 warnings. `bunx vitest run`: as in the release's record.

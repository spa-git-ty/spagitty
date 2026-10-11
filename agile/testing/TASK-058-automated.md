<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-058 — Automated test record

**Item:** [Restore the patch release](../items/TASK-058-restore-the-patch-release.md)

Before the correction, tools/record.test.ts reproduced the same four failures
as run 37857852830, with 904 cases passing.

Validated on Windows on 2026-10-11:

- `bun run check`: no Svelte errors or warnings; extension SDK types pass.
- `bun run coverage`: 178 files and 3,696 tests pass, including the existing
  record, version, release-note and macOS signing-policy tests.
- Coverage: statements 82.38%, branches 72.57%, functions 79.54%, lines 85.13%;
  all exceed the existing 70% thresholds.
- `cargo fmt --all --check` and `git diff --check`: pass.
- Cargo.lock changes only the versions of the five workspace packages from
  2.0.0 to 2.0.1. Dependency versions are unchanged.

Windows release compilation is pending. Rust workspace tests and coverage,
platform process tests and security checks also run on the pull request in CI.

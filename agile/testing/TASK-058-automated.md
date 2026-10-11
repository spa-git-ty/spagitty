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

- `bun run tauri build --no-bundle`: passes, including the official extension
  worker and production frontend build. The Windows release executable reports
  ProductVersion and FileVersion 2.0.1. Installer packaging is left to Gate 5.
- Final record/version/release-note/macOS policy check: 966 cases pass.

Rust workspace tests and coverage, platform process tests and security checks
also run on pull request #61 in CI.

Cache timeout follow-up on 2026-10-11:

- Parsed all three build workflow YAML files and compared them with the prior
  commit: all eight cache steps have five-minute limits and optional fallback,
  with two-minute segment limits. Triggers, permissions, matrices, cache keys
  and every required check are unchanged.
- Existing record/version/release-note/macOS/update tests: 978 cases pass.
- `git diff --check`: passes. This follow-up changes no application code or
  dependencies; the successful Windows build above remains applicable.
- Cancelled the stalled PR run `38125434703`; requested force-cancellation when
  ordinary cancellation did not stop the cache step. The next push restarts
  CI with the updated workflow.

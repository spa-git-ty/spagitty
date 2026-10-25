<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-075 — Remove CodeRabbit

**Status:** Done — on `feature/FEAT-158-type-to-search`, in release 3.10.0; unmerged. Covered by the workspace's clippy, the farm's supplemental tests and the extension crate's tests; the installed application has not been looked at (the author is checking it).
**Branch:** `feature/FEAT-158-type-to-search`.
**Screens:** Farm (1Q, 1A): the rules and the task card; Settings › Extensions, which no longer lists an official extension.
**Raised by:** the author, 2026-10-25: "drop code rabbit remove it".

## Change

- **The extension is gone.** `extensions/coderabbit/` (manifest, README, fixtures and the Rust worker crate `coderabbit-extension`) is deleted, and so are the workspace member, its `Cargo.lock` entry, `tools/extensions/bundle.ts` and its test, the bundled resource in `tauri.conf.json`, the macOS sidecar in `tauri.macos.conf.json`, the staging step in `beforeDevCommand`/`beforeBuildCommand`, CI's two staging steps and its `-p coderabbit-extension` test, and the `.gitignore` lines.
- **The farm's extra review rule is gone** (FEAT-098). The *CodeRabbit* rule in the farm's settings, the task card's *CodeRabbit* button, the `farm_review_supplemental` command and the application's reviewer adapter (`src-tauri/src/supplemental.rs`) are removed. `FarmService::supplemental_mode` counts the rule as off while no reviewer is set, and the application sets none, so a farm saved with the review *Required* is not left waiting for a reviewer that cannot come.
- **Kept on purpose:** the extension host and SDK (FEAT-096), including review contributions, PR snapshots and PR comments, which any extension may use. Also the farm's saved policy, evidence and events, so farms and logs written before 3.10.0 still open, and the AI-attribution lists that name CodeRabbit, which are about who wrote a commit, not about the integration.
- Docs: `docs/architecture.md`, `docs/extensions/*`, `docs/screens.md` and `docs/testing.md` no longer describe CodeRabbit, and the proposal says what became of it.

## Acceptance criteria

- No build step, resource, sidecar or workflow mentions the CodeRabbit extension; `cargo clippy --workspace --all-targets -- -D warnings` passes.
- The farm's settings and task card show no CodeRabbit control.
- A farm saved with the review required merges a reviewed task without a reviewer (`without_a_provider_a_required_review_does_not_hold_the_merge`).

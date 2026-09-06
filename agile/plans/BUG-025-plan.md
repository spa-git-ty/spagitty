<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-025 — Plan

**Item:** [One completion watcher per run](../items/BUG-025-single-run-watcher.md).

## Approach and ordered steps

1. Register sessions by run id with task and phase metadata. Claim each once, retaining cancellation separately.
2. Move watcher ownership into the service and let Tauri request pending watchers, including review.
3. Pass the exact run id into completion and ignore stale results; retain handles until finalization.
4. Add headless multi-run, duplicate-claim, review and retry regressions.

## Validation

Write regression assertions that fail on the reported behavior, then run the
affected Rust suites, the frontend record tests, formatting and Clippy. Run full
workspace checks and coverage before landing the reliability release.

## Risks and rollback

Keep process waits outside service locks. Preserve saved farms and authored work.
Revert this item's PR to roll back; never remove task branches or move release tags.

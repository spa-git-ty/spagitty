<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-027 — Plan

**Item:** [Require current evidence before automatic merging](../items/BUG-027-verified-merges.md).

## Approach and ordered steps

1. Record source commit and clean worktree evidence around verification and review.
2. Require successful checks and independent successful review for automatic merging, with explicit farm permission.
3. Record the intended destination on task creation, validate state and destination before mutation, and serialize merges.
4. Invalidate absent or stale evidence after retry/restart and preserve manual approval as an explicit path.
5. Add real-repository regressions for unchecked, failed, changed and redirected merges.

## Validation

Write regression assertions that fail on the reported behavior, then run the
affected Rust suites, the frontend record tests, formatting and Clippy. Run full
workspace checks and coverage before landing the reliability release.

## Risks and rollback

Keep process waits outside service locks. Preserve saved farms and authored work.
Revert this item's PR to roll back; never remove task branches or move release tags.

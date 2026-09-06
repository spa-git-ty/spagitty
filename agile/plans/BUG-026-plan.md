<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-026 — Plan

**Item:** [Verification keeps draining and can be stopped](../items/BUG-026-verification-output.md).

## Approach and ordered steps

1. Drain stdout and stderr concurrently into a bounded byte tail.
2. Use the existing process-tree containment for verification and terminate it on deadline or cancellation.
3. Register verification cancellation with the service, emit start events only for commands actually run, and stop after failure.
4. Test noisy success/failure, short deadlines, cancellation and no later commands.

## Validation

Write regression assertions that fail on the reported behavior, then run the
affected Rust suites, the frontend record tests, formatting and Clippy. Run full
workspace checks and coverage before landing the reliability release.

## Risks and rollback

Keep process waits outside service locks. Preserve saved farms and authored work.
Revert this item's PR to roll back; never remove task branches or move release tags.

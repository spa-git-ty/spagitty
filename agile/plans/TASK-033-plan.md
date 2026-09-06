<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-033 — Plan

1. Include routes in coverage and measure uncovered behavior.
2. Test route lifecycle, guarded actions, errors and recovery with mounted components.
3. Run portable process-tree and verification regressions on all supported platforms.
4. Enforce 70% thresholds and run full local checks plus ordered CI gates.

Rollback through a revert PR; never weaken the floor to obtain a passing run.

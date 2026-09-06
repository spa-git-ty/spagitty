<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-027 — Manual sweep

## FARM-01 — Require current evidence before automatic merging

**Priority:** High. **Preconditions:** Disposable repository and configured agents.
1. Reproduce the trigger described in the item.
2. Exercise the corrected path and observe task state, output and worktree contents.
3. Repeat while another task is running; reopen the repository afterward.
**Expected:** Automatic merges require passing current checks and independent successful review, merge permission and the original destination. Failed or missing evidence prevents mutation. Invalid task states are rejected before Git runs; concurrent merges serialize and conflicts preserve work. Existing farms load with no trusted evidence until rechecked.
**Pass/fail:** ______

Desktop interaction requires a human tester or explicit handover. Automated
headless evidence is recorded separately and does not imply a UI sweep passed.

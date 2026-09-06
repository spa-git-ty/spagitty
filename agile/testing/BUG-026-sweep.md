<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-026 — Manual sweep

## FARM-01 — Verification keeps draining and can be stopped

**Priority:** High. **Preconditions:** Disposable repository and configured agents.
1. Reproduce the trigger described in the item.
2. Exercise the corrected path and observe task state, output and worktree contents.
3. Repeat while another task is running; reopen the repository afterward.
**Expected:** Noisy checks finish; retained output stays bounded and valid UTF-8. Timeout and cancellation stop descendants. Cancelled tasks cannot advance into review, and commands after a failed or stopped check never start.
**Pass/fail:** ______

Desktop interaction requires a human tester or explicit handover. Automated
headless evidence is recorded separately and does not imply a UI sweep passed.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-024 — Manual sweep

## FARM-01 — Stop a run while its waiter owns it

**Priority:** High. **Preconditions:** Disposable repository and configured agents.
1. Reproduce the trigger described in the item.
2. Exercise the corrected path and observe task state, output and worktree contents.
3. Repeat while another task is running; reopen the repository afterward.
**Expected:** A watched task and a collected planner stop promptly, descendants cannot keep writing, terminal state is preserved, and service locks are never held while joining pipes. Existing farm data stays readable. Verification command cancellation is completed with the verification runner item.
**Pass/fail:** ______

Desktop interaction requires a human tester or explicit handover. Automated
headless evidence is recorded separately and does not imply a UI sweep passed.

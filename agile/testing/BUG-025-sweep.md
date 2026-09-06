<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-025 — Manual sweep

## FARM-01 — One completion watcher per run

**Priority:** High. **Preconditions:** Disposable repository and configured agents.
1. Reproduce the trigger described in the item.
2. Exercise the corrected path and observe task state, output and worktree contents.
3. Repeat while another task is running; reopen the repository afterward.
**Expected:** Every implementation and review run has one completion owner. An empty claim does not reschedule watchers. Completion applies only to its own run, errors are visible, and cancelled attempts cannot alter a retry.
**Pass/fail:** ______

Desktop interaction requires a human tester or explicit handover. Automated
headless evidence is recorded separately and does not imply a UI sweep passed.

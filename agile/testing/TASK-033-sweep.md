<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-033 — Manual sweep

## COVERAGE-01 — Route navigation and platform process stops

**Priority:** High. **Preconditions:** Disposable repo, packaged app on each OS.
1. Navigate Farm, branches, rebase, requests and settings while tasks run.
2. Cancel an agent that spawned a descendant and confirm files stop changing.
3. Cancel noisy verification, reopen the repo and inspect the retained logs.
**Expected:** Navigation remains responsive, no late child writes, accurate state.
**Pass/fail:** Pending human test or explicit desktop handover.

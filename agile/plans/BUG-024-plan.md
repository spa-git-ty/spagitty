<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-024 — Plan

**Item:** [Stop a run while its waiter owns it](../items/BUG-024-cancellable-runs.md).

## Approach and ordered steps

1. Separate the cloneable cancellation handle from the single-owner pipe readers.
2. Keep handles registered while waiters own sessions; route task, farm and planner cancellation through them.
3. Prevent late completions and immediate retries from reviving cancelled work.
4. Use Unix process groups and Windows jobs to stop descendants, and add bounded regression tests.

## Platform decision

Use Unix process groups and Windows job objects. On Windows spawn suspended,
assign the job with kill-on-close, then resume the primary thread. This prevents
a descendant escaping between process creation and job assignment. The standard
library does not expose the primary thread handle on stable Rust, so a Toolhelp
thread snapshot finds the suspended thread by its process id.

Reference: https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject

## Validation

Write regression assertions that fail on the reported behavior, then run the
affected Rust suites, the frontend record tests, formatting and Clippy. Run full
workspace checks and coverage before landing the reliability release.

## Risks and rollback

Keep process waits outside service locks. Preserve saved farms and authored work.
Revert this item's PR to roll back; never remove task branches or move release tags.

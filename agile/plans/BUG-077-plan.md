<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-077 — Plan

**Item:** [BUG-077](../items/BUG-077-a-farm-task-merged-in-merger-skips-the-farms-checks.md)

`merge_checked` opens with a check that the task is in review and that none of
its runs is going, and later asks `check_supplemental`. `mark_merged` did
neither.

- Move the opening check into `awaiting_merge`, which returns the task and its
  provider, and use it from both.
- `mark_merged` keeps its early return for a task that is already done, then
  asks `awaiting_merge` and `check_supplemental` before it looks at where the
  commit is.
- A refusal after the merge is written is too late to stop the merge, so the
  same checks are public as `check_mergeable`, behind the command
  `farm_merge_check`. `openInMerger` awaits it and shows the Farm's reason
  instead of opening Merger.

The supplemental rig (`tests/supplemental.rs`) has a task in review whose
branch is already in `main` and a required review with no evidence, which is
the case that was wrongly finished. It runs on every platform, unlike the
pipeline tests.

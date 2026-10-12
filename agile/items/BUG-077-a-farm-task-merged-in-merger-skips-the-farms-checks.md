<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-077 — A Farm task merged in Merger skips the Farm's own checks

**Status:** Open — fixed on `bugfix/BUG-076-merger-says-pushed-when-the-push-failed` with its tests; Open until the sweep runs on a real Farm run (2026-10-12).
**Screens:** 1Q, 1S.
**Raised by:** a review of the 2.0.1 stack on 2026-10-12, before FEAT-117 was merged.

## What happens

Before FEAT-117 the Farm merged a task itself, and refused unless:

- the task was in review;
- none of its runs was still going;
- where a supplemental review is *Required*, a current one had passed.

FEAT-117 moved the merge to Merger. The Farm is told afterwards (`mark_merged`), and that only asked whether the task's commit was in the target branch. So:

- a task whose supplemental review was required and missing could be merged from the Farm's own *Merge it in Merger* button, since the screen's *ready* does not look at supplemental review;
- any task whose commit was already in its target, such as one with no work yet, was finished and its worktree removed whatever state it was in, a run in progress included.

## Acceptance criteria

- *Merge it in Merger* asks the Farm first. A task the Farm would not merge is refused with the Farm's own reason, and Merger is not opened.
- Telling the Farm a task was merged is refused for a task that is not in review, has a run going, or lacks a required supplemental review. The task and its worktree are left as they were.
- A task that passes is finished as before, and telling the Farm twice changes nothing.
- Each of these has a test.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `crates/spagitty-farm/src/service.rs` (`awaiting_merge`, `check_mergeable`, `mark_merged`), `src-tauri/src/farm.rs` (`farm_merge_check`), `src/lib/farm/merge.ts` (`openInMerger`).

## Fix

The in-review and nothing-running check that the Farm's own merge made is now one function, `awaiting_merge`, used by that merge and by `mark_merged`; `mark_merged` also asks for the required supplemental review. `check_mergeable` asks the same before any merge is made, and `openInMerger` calls it through `farm_merge_check` before it opens Merger.

## Left

Merger can still be opened on a Farm task's branch by hand, from the Graph or by choosing the two branches. That merge is the person's own and the Farm is not told of it; the task stays in review until it is merged from the Farm.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-146 — Plan

**Item:** [FEAT-146](../items/FEAT-146-a-conflict-resolver-chosen-in-rules.md)

Written on 2026-10-19, as the work began.

## The pieces

1. **The choice** — `ConflictResolver` (`ask`, `agent`, `tool` with a command) on `Farm` and `Setup`, default `ask`; `farm_configure` takes it and turns an empty command into `ask`.
2. **Opening the conflict** — `workspace/resolve.rs`: `git merge --no-ff --no-commit <destination tip>` in the task's workspace, after fetching the destination into a copy by path. A merge that is clean by now is committed at once.
3. **The command** — run through the verifier's runner with the conflicted paths quoted after it; `finish` refuses a conflict marker, stages the paths and commits; anything wrong is `merge --abort` and a blocked task.
4. **The agent** — the change request names the paths and how to finish the merge; the task goes to Waiting and the scheduler gives it back to its agent. A slice is not narrowed while a merge waits in it.
5. **Again from the top** — the committed resolution goes through `verify`, which for a copy takes it at the door (`allow_merge` lets that one merge in and measures paths from the destination), then review, then the lane.
6. **The screen** — *When a merge conflicts* under *More rules*.

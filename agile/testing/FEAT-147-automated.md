<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-147 — Automated test record

**Item:** [FEAT-147](../items/FEAT-147-own-copies-and-intake.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The copy | `crates/spagitty-farm/src/workspace/copy.rs` | A clone on the farm branch with no remote and no other branch, not a linked worktree; push, fetch and `branch -f` from it move nothing of the person's; a second copy starts from work already taken; uncommitted work survives removal unless forced. |
| Intake | same | Work is taken into the inbox and the farm branch moved; a path outside *May touch* is refused and named; a protected path is refused unless named; a file over the limit is refused; work not from the start is refused; a copy that committed nothing brings nothing in. |
| Cleanup | `crates/spagitty-farm/src/workspace/cleanup.rs` | An unclaimed copy with nothing unique is swept; a claimed one is kept. |
| The prompt | `crates/spagitty-farm/src/context.rs` | A copy farm's prompt says *YOUR COPY* and not to push. |
| End to end | `crates/spagitty-farm/tests/pipeline.rs` | On a copy farm the agent's copy has no remote and cannot push, its commit becomes the farm branch, the checks pass and the task is in review; a stray file outside `docs/**` blocks the task and is never checked. |

## Not covered

- Not looked at in the running application; the author is checking it.
- How long a copy of a large repository takes against a worktree, on each platform (the proposal's measure).
- The pipeline tests use `/bin/sh` agents and run in WSL.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-073 — Two quick Assign clicks start two agents on one pull request

**Status:** Open — found by the TASK-063 audit of pull request #52 (2026-10-12).
**Screens:** 1R, 1S.
**Raised by:** CodeRabbit on #52, still unresolved there; confirmed in the code on 2026-10-12.

## What happens

`assignment_start` (`src-tauri/src/agents.rs`) checks that no live assignment is on the same job, lets go of the `live` lock, then reads consent, opens the store, builds the driver and the scratch worktree and starts the engine. It inserts the new handle into `live` only at the very end. Two Assign clicks, or a click and a resumed assignment, that arrive while the first is still being set up both pass the check, and two agents work one pull request, spending tokens twice and proposing twice.

## Acceptance criteria

- A second start on the same job, begun while the first is still being set up, is refused with *An agent is already assigned to this.*
- A refused or failed start leaves no claim behind: the job can be assigned again at once.
- The two cases have a unit test that does not need a network or an agent.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src-tauri/src/agents.rs`, `assignment_start` and `same_job`.
- **Shape:** claim the job under the same lock as the check (a reserved entry, or a set of jobs being started), and release the claim on every early return.

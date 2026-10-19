<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-073 — Plan

**Item:** [BUG-073](../items/BUG-073-two-quick-assign-clicks-start-two-agents.md)

1. Give `AgentsState` a list of jobs being started beside `live`.
2. Check both under one lock and claim the job in the same step, so a second start that arrives while the first is set up is refused with *An agent is already assigned to this.*
3. Make the claim a guard that lets the job go when dropped, so every early return (consent, key, a missing agent, a repository that will not open) releases it, and drop it only after the handle is in `live`.
4. Unit-test the claim: a second start refused, other jobs free, nothing left behind by a start that fails.

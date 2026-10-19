<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-073 — Sweep

**Item:** [BUG-073](../items/BUG-073-two-quick-assign-clicks-start-two-agents.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG073-01 | Review open on a pull request; an agent set up for reviews | Click **Assign an agent…**, choose the agent, then click its start button twice quickly | One agent starts; the second click says *An agent is already assigned to this.* | Unit tests; desktop check not run |
| SWEEP-BUG073-02 | A remote agent with no key saved | Assign it, read the refusal, save the key, assign again | The second assignment starts at once; nothing is left claiming the pull request | Unit tests; desktop check not run |
| SWEEP-BUG073-03 | Merger open on two branches; an agent set up for merges | Assign an agent to the merge and click start twice quickly | One agent works the merge | Unit tests; desktop check not run |

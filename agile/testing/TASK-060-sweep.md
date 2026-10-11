<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-060 — Sweep

**Item:** [TASK-060](../items/TASK-060-agent-permissions-with-standard-names.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK060-01 | An agent set up | Open the Assign popover in Review and in Merger | The four permissions are Read-only, Ask every step, Ask at the end and Full access, each with a sentence that says what the agent may do | P1 | Not run. |
| SWEEP-TASK060-02 | Settings › Agents with a repository open | Look at the repository's card | *Maximum permission* and *Branches without full access*; no *Highest level* or *Sign off* | P1 | Not run. |
| SWEEP-TASK060-03 | A repository whose maximum is Ask before sending or merging | Open the Assign popover | Full access is disabled and says *The most allowed here is Ask before sending or merging* | P2 | Not run. |

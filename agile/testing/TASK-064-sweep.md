<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-064 — Sweep

**Item:** [TASK-064](../items/TASK-064-refuse-agent-attribution-in-ci.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK064-01 | A pull request with this change merged into its base | Add a commit with a `Co-Authored-By: Claude …` trailer and push | Gate 1 fails on the attribution step and names the line | P1 | Not run. Needs the workflow on GitHub. |
| SWEEP-TASK064-02 | As above | Put a *Generated with Claude Code* line in the description | Gate 1 fails; editing it out and re-running passes | P1 | Not run. |
| SWEEP-TASK064-03 | As above | A clean pull request | The step passes and gate 1 goes on to the licence checks | P1 | Passed, 2026-10-12, on pull requests #63 and #64: gate 1 ran the step and finished green. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-074 — Sweep

**Item:** [BUG-074](../items/BUG-074-a-remote-agents-token-limit-is-read-only-between-steps.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG074-01 | A remote agent with **Tokens per run** set low (such as 5,000); a pull request with a large changed file | Assign the agent to review it | The step stops part-way with *Reached its limit of 5,000 tokens. Everything it made is kept.*; the tokens shown are not several times the limit | Unit tests; desktop check not run |
| SWEEP-BUG074-02 | Same agent, **Minutes** set to 1; a slow model | Assign it and wait | Within about a minute of starting, the assignment stops with the minutes sentence, even while waiting for an answer | Unit tests; desktop check not run |
| SWEEP-BUG074-03 | A remote agent with no limits | Review a pull request to the end | It runs to the verdict as before | Unit tests; desktop check not run |
| SWEEP-BUG074-04 | Any remote agent working | Press **Stop** while it waits for the provider | It stops within a second rather than when the answer comes | Unit tests; desktop check not run |

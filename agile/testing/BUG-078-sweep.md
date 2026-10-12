<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-078 — Sweep

**Item:** [BUG-078](../items/BUG-078-the-attribution-check-refuses-a-person-named-like-an-agent.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG078-01 | A pull request with this change in its base | Add a commit with `Co-Authored-By: Claude Dupont <claude.dupont@example.org>` and push | Gate 1's attribution step passes | P2 | Not run. |

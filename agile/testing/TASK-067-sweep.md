<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-067 — Sweep

**Item:** [TASK-067](../items/TASK-067-the-attribution-check-does-not-read-who-wrote-a-commit.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK067-01 | A pull request with this change in its base | Push a commit made with `git commit --author="Claude <noreply@anthropic.com>"` | Gate 1 fails on the attribution step and names the commit as *author* | Not run. Needs the workflow on GitHub. |
| SWEEP-TASK067-02 | As above | A clean pull request, including GitHub's own merge commits | The step passes | Not run. |

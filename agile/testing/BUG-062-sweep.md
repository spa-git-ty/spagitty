<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-062 — Sweep

**Item:** [BUG-062](../items/BUG-062-after-a-merge-another-pull-request-opens-empty.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG062-01 | Two open pull requests on `maxmya/trattoria-demo`, workspace open on one | *Merge*, *Create a merge commit*, *Confirm Merge* | The list returns without the merged one and a notice says *#N merged* | P1 | Not run. |
| SWEEP-BUG062-02 | As above | *Close*, *Close Pull Request* | The list returns and a notice says *#N closed* | P1 | Not run. |
| SWEEP-BUG062-03 | After either | Open the other pull request from the list | Its files and commits show | P2 | Not run. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-081 — Sweep

**Item:** [BUG-081](../items/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG081-01 | New local branch; remote `origin` | Press Push twice, adding a commit between pushes | First push creates the branch and tracking; second pushes normally | Covered with temporary repositories; desktop check not run |
| SWEEP-BUG081-02 | Branch tracks a differently named branch on another remote | Press Push | Correct remote and upstream branch receive it | Destination covered in frontend tests; desktop check not run |
| SWEEP-BUG081-03 | Remote has commits missing locally | Press Push | Git's rejection is shown; remote commits remain | Real rejection covered by worker test; desktop check not run |
| SWEEP-BUG081-04 | Missing credentials or remote access denied | Fetch and Push | Git's error is visible and another operation can be started | stderr and completion covered automatically; real-host check not run |
| SWEEP-BUG081-05 | Large remote transfer | Fetch and Push | Progress changes while the transfer runs | Progress event parser and local fetch covered automatically; desktop check not run |

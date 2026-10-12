<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-067 — Sweep

**Item:** [BUG-067](../items/BUG-067-review-says-the-token-is-invalid.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG067-01 | Two accounts connected (GitHub and the work GitLab); a GitHub repository open; the app just started | Go straight to Review | The inbox lists pull requests; no refusal is shown; it says *signed in as* the GitHub login | P1 | Not run. |
| SWEEP-BUG067-02 | A token that has been revoked | Open Review | The refusal names the host and the account | P2 | Not run. |

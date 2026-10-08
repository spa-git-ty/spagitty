<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-064 — Sweep

**Item:** [BUG-064](../items/BUG-064-review-says-nobody-asked-you-after-you-answered.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG064-01 | Requested reviewer on two pull requests on `maxmya/trattoria-demo` | *Request changes* on one, *Approve* the other, then refresh Review | Both under *Reviewed by you · you left a review*, saying *you asked for changes* and *you approved*; neither under *nobody asked you yet* | P1 | Not run. |
| SWEEP-BUG064-02 | As above | Push a commit to the one with changes requested, refresh | Its card says *you asked for changes · changed since* and it comes first in the group | P1 | Not run. |
| SWEEP-BUG064-03 | A pull request you have never reviewed | Open Review | It stays under *Open on this repo · nobody asked you yet* | P2 | Not run. |

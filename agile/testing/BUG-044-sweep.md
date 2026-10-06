<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-044 — Manual sweep

**Item:** [`agile/items/BUG-044-start-review-does-nothing-on-gitlab.md`](../items/BUG-044-start-review-does-nothing-on-gitlab.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG044-01 | Release build; a GitLab repository open; *All my repos* not visited since launch | 1. Review, *This repo* 2. Choose a merge request 3. Start review | The review room opens on that merge request | P1 | |
| SWEEP-BUG044-02 | As 01, after viewing a file in the room | 1. Back to Review | The card shows how far the review got, not "not started" | P1 | |
| SWEEP-BUG044-03 | Release build; a github.com repository open | 1. Review 2. Start review | The review room opens, as before | P1 | |

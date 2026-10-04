<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-087 — Manual sweep

**Item:** [`agile/items/FEAT-087-the-review-screen-and-inbox.md`](../items/FEAT-087-the-review-screen-and-inbox.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT087-01 | A GitHub repository where you are a requested reviewer | 1. Open the repository | The rail shows Review after Pull requests, with a dot | P1 | |
| SWEEP-FEAT087-02 | As 01 | 1. Review | Needs you lists it; the preview shows its description, threads and checks | P1 | |
| SWEEP-FEAT087-03 | A pull request where the author answered your comment | 1. Review | It is under Back with you, with "1 reply to you" | P2 | |
| SWEEP-FEAT087-04 | As 02 | 1. Start review 2. ‹ Review | The room's header opens; back returns to the inbox | P1 | |
| SWEEP-FEAT087-05 | Pull requests on two repositories, one cloned in Spagitty | 1. All my repos 2. Start review on each | The cloned one opens its repository and room; the other says there is no clone | P2 | |
| SWEEP-FEAT087-06 | A GitLab project (the author's work server) with a merge request assigned to you as reviewer | 1. Review | It is under Needs you | P1 | |
| SWEEP-FEAT087-07 | No account connected | 1. Review | "No account is connected." with the way to Settings | P2 | |
| SWEEP-FEAT087-08 | Light and dark | 1. Review in each | Cards, chips and preview read in both | P2 | |

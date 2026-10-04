<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-088 — Manual sweep

**Item:** [`agile/items/FEAT-088-gitlab-as-its-api-says.md`](../items/FEAT-088-gitlab-as-its-api-says.md)

On the author's work GitLab unless a row says otherwise.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT088-01 | A project in a nested group, cloned | 1. Open it 2. Pull requests | Its merge requests are listed | P1 | |
| SWEEP-FEAT088-02 | A merge request with you as reviewer and one without | 1. Pull requests | Only the first is under Needs you; neither claims passing checks | P1 | |
| SWEEP-FEAT088-03 | As 02 | 1. Review | Needs you shows the first with its real pipeline state and thread count | P1 | |
| SWEEP-FEAT088-04 | A merge request with a failed pipeline | 1. Review → preview | "Checks failing" | P1 | |
| SWEEP-FEAT088-05 | A merge request | 1. Pull requests → open it | Files with diffs, commits, and its line comments | P1 | |
| SWEEP-FEAT088-06 | A thread on it | 1. Reply from Pull requests | The reply is in the thread on GitLab | P2 | |
| SWEEP-FEAT088-07 | A merge request you may approve | 1. Leave a review with one comment and Approve | One published comment, and your approval, on GitLab | P1 | |
| SWEEP-FEAT088-08 | A GitLab instance whose host name does not start with `gitlab.` | 1. Settings → Accounts → connect it | It is connected as GitLab, and its projects are read | P2 | |
| SWEEP-FEAT088-09 | A GitLab older than 15.7, if one is reachable | 1. Open a merge request's files | They come from `/changes` | P3 | |

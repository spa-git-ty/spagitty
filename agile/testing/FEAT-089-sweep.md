<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-089 — Manual sweep

**Item:** [`agile/items/FEAT-089-a-pull-request-read-from-disk.md`](../items/FEAT-089-a-pull-request-read-from-disk.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT089-01 | A GitHub repository with an open pull request, its head not fetched | 1. Review → Open in worktree | A toast names `<repo>-pr-<N>`; that folder holds the head; no new branch in Branches or on the graph | P1 | |
| SWEEP-FEAT089-02 | The author's work GitLab, an open merge request | 1. As 01 | The same, fetched from `refs/merge-requests/N/head` | P1 | |
| SWEEP-FEAT089-03 | As 01, then the author pushes | 1. Open in worktree again | The same folder, now at the new head | P2 | |
| SWEEP-FEAT089-04 | A remote over SSH with no key loaded | 1. Open in worktree | A toast with git's refusal; nothing waits | P2 | |

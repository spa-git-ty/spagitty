<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-123 — Sweep

**Item:** [FEAT-123](../items/FEAT-123-merger-checks-out-what-it-merged-into-and-offers-the-push.md)

Row 01 was looked at on 2026-10-12 in the built frontend against fixtures, drawn by headless Chrome. git did not run: every row below is owed in the application, against a real repository and a real remote.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT123-01 | `main` checked out, a branch `release` with an upstream, a clean working copy | In Merger, merge `main` into `release` | The dialog says the merge is made, then *release is checked out*. The toolbar's branch reads `release`. *Push now*, *Not now* and *Open in Graph* are offered from the start. | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT123-02 | As above | Press *Push now* | The capsule shows git's progress; the dialog says *Pushed to origin/release*; the remote has the merge commit | P1 | Seen against fixtures. Not run against a remote. |
| SWEEP-FEAT123-03 | As 01 | Press *Not now* | The dialog closes. `release` is checked out and one commit ahead of its upstream. | P1 | Not run. |
| SWEEP-FEAT123-04 | As 01, with an uncommitted change to a file the merge also changes | Merge | *Merged, but release could not be checked out:* and git's sentence. `main` is still checked out and the change is untouched. *Push now* is still offered. | P1 | Not run. |
| SWEEP-FEAT123-05 | After 04 | Stash, then press *Check out release* | `release` is checked out | P2 | Not run. |
| SWEEP-FEAT123-06 | `main` checked out | Merge a branch into `main` | Nothing is said about a checkout. The files are the merged ones. *Push now* is offered. | P1 | Not run. |
| SWEEP-FEAT123-07 | `release` checked out in a second worktree | From the first, merge `main` into `release` | The dialog says `release` is checked out in the other worktree's path, not here. The files there are the merged ones. | P2 | Not run. |
| SWEEP-FEAT123-08 | Any two branches | Merge into a new branch | The new branch is checked out. *Push now* sends it to `origin` and it tracks `origin` afterwards. | P1 | Not run. |
| SWEEP-FEAT123-09 | A repository with no remote | Merge | No *Push now*. *No remote to push to.* | P2 | Not run. |
| SWEEP-FEAT123-10 | A remote that has moved on | Merge, then *Push now* | git's rejection is shown in the dialog and *Push now* can be pressed again after a pull | P1 | Not run. |
| SWEEP-FEAT123-11 | A pull request that conflicts | Resolve it in Merger | As before this item: *Push pr-N* and *Back to the pull request* | P1 | Not run. |
| SWEEP-FEAT123-12 | A Farm task in review | *Merge it in Merger* | As before this item: the working copy is not moved | P2 | Not run. |

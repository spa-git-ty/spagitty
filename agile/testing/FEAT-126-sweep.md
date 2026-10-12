<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-126 — Sweep

**Item:** [FEAT-126](../items/FEAT-126-merger-says-a-branch-is-behind-its-remote-and-pulls-it.md)

None of these rows has been run. The core is tested against a real remote on disk and the screen by mounted tests; nothing was looked at in a browser or in the application, and nothing was run against a hosted remote.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT126-01 | A repository on `main`, whose `origin/main` has three commits `main` lacks, fetched | Open Merger with `main` as a side | Its card says *3 new commits on origin/main* with **Pull now** | P1 | Not run. |
| SWEEP-FEAT126-02 | As above | Press **Pull now** | The toolbar says *Pulling main…*; the card's line goes; the plan is worked out again; the working copy has the new commits | P1 | Not run. |
| SWEEP-FEAT126-03 | On `main`; `feat/x` behind `origin/feat/x`, not checked out anywhere | Pick `feat/x` as a side, press **Pull now** on its card | `feat/x` moves; `main` is still checked out and its files are untouched | P1 | Not run. |
| SWEEP-FEAT126-04 | As 03, with uncommitted changes on `main` | Press **Pull now** on `feat/x` | It moves; the uncommitted changes are as they were | P1 | Not run. |
| SWEEP-FEAT126-05 | `feat/x` checked out in a second worktree, behind its upstream | Press **Pull now** on its card from the first window | It moves, and the second worktree's files follow | P2 | Not run. |
| SWEEP-FEAT126-06 | `main` behind by three and ahead by two | Open Merger | The line shows; **Pull now** is disabled and its title says `main` has 2 commits of its own and to pick `origin/main` as the other branch | P1 | Not run. |
| SWEEP-FEAT126-07 | `main` behind, with an uncommitted change to a file the new commits also change | Press **Pull now** | A notice with git's sentence; nothing moved; nothing stashed | P2 | Not run. |
| SWEEP-FEAT126-08 | A branch behind its upstream; the last fetch was before the remote moved | Open Merger, then press **Fetch** in the toolbar | Nothing on the card at first; the line appears after the fetch | P2 | Not run. |
| SWEEP-FEAT126-09 | A fetch running | Look at **Pull now** | Disabled, with *Fetching is still running* | P2 | Not run. |
| SWEEP-FEAT126-10 | Any of the above on GitLab over SSH and on GitHub over HTTPS | Press **Pull now** | As on the local remote; a credential that is refused is git's sentence in a notice | P2 | Not run. |

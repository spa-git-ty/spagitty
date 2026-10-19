<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-058 — Manual sweep

> **Refreshed 2026-10-19 (TASK-073).** FEAT-116 moved reviewing to the Review screen. The reading tickets still apply; the review tickets are obsolete.

**Item:** [`agile/items/FEAT-058-pull-request-files-and-review.md`](../items/FEAT-058-pull-request-files-and-review.md)

Every ticket needs a repository with an `origin` on GitHub, an account
connected under Settings → Accounts, and at least one open pull request.

**SWEEP-005 and SWEEP-006 write to a real server.** Run them on a repository
you own, on a pull request nobody is waiting on. A submitted review is visible
to everyone watching it and Spagitty cannot take it back.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A pull request with several changed files | 1. Open Pull requests 2. Select the request 3. Read the Files list | Every changed path is listed, each with `+n −m`. The list scrolls when it is long rather than pushing the diff off the panel. The first file is already open — no click is needed to see something. | High | |
| SWEEP-002 | The same | 1. Click a second file 2. Compare the diff against the same file on the host's own website | The selected row is highlighted and the diff below it is that file's, with the same added and removed lines in the same places. Line numbers on both sides match the host's. | High | |
| SWEEP-003 | A pull request touching an image or another binary | Select that file | It says `binary` in place of the counts, and the pane does not claim the file changed nothing. | Medium | |
| SWEEP-004 | Two pull requests open in the list | 1. Select the first and let its files load 2. Select the second immediately, before its files arrive | The panel never shows the first request's files under the second's title. The files that settle are the second's. | High | |
| SWEEP-005 | **A pull request you own.** A repository you can afford to post to | 1. Press Review 2. Choose Comment 3. Leave the box empty and look at the Submit button 4. Type something 5. Submit and read the confirmation 6. Confirm | Submit is disabled while the box is empty, with a line saying a comment is needed. The confirmation names the verdict — "Comment #N?" — rather than asking whether you are sure. After confirming, a notice says it was sent, and the comment appears on the host. | High | Obsolete, 2026-10-19 — FEAT-116 moved reviewing from Pull requests to Review. |
| SWEEP-006 | The same, a pull request you own | 1. Press Review, choose Request changes 2. Submit, and **cancel** at the confirmation 3. Submit again and confirm | Cancelling sends nothing — check the host to be sure. Confirming posts the review and the list re-reads: the row's review state changes to "changes requested" without a manual refresh. | High | Obsolete, 2026-10-19 — FEAT-116 moved reviewing from Pull requests to Review. |
| SWEEP-007 | Any account connected | 1. Open a pull request **you opened yourself** 2. Try to approve it | GitHub refuses it, and the panel shows the host's own sentence about not reviewing your own pull request. It is shown as a review failure, not as the pull request list having failed — the list is still on screen. | Medium | Obsolete, 2026-10-19 — FEAT-116 moved reviewing from Pull requests to Review. |
| SWEEP-008 | Any pull request | 1. Turn off networking 2. Select a pull request that has not been read yet | The files area says what went wrong in the host's words and offers Try again. Pressing it once networking is back loads the files. | Medium | |
| SWEEP-009 | A pull request from a fork, or on a branch you have never fetched | Open it and read the files | The diff renders. Nothing is fetched into the repository — check `git branch -a` before and after and confirm nothing new appeared. | High | |
| SWEEP-010 | Any pull request | 1. Drag the detail panel's splitter wider and narrower 2. Use the hide-panel control in the header | The file list and the diff resize with the panel and stay readable. The panel still hides and returns as it did before. | Low | |
| SWEEP-011 | A pull request with more than 100 changed files, if one can be found | Open it | Every file is listed, not the first hundred. | Low | |

## Negative paths this sweep deliberately covers

- **SWEEP-004** is the staleness path. It is timing-dependent by nature, which
  is exactly why a person doing it quickly is worth more than the automated
  test that also covers it.
- **SWEEP-006's cancel step** proves the confirmation is a gate and not a
  formality. Checking the host afterwards is the only way to know nothing was
  sent.
- **SWEEP-007** is the failure a reviewer will actually hit first, and the
  thing being checked is that one failure does not look like another.
- **SWEEP-009** is the promise that opening a review does not touch the
  repository. It is the reason the diff comes from the host at all.

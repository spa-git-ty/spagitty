<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-098 — Pull requests says only "500 Internal Server Error" on the author's GitLab

**Status:** Fixed — on `fix/BUG-097-sign-in-and-gitlab-list`; unmerged. Covered by tests with the server's answers stood in; not reproduced against the author's server, whose token could not be read from here (the author is checking it).
**Branch:** `fix/BUG-097-sign-in-and-gitlab-list`.
**Screens:** 1H (Pull requests), 1R (Review).
**Raised by:** the author, 2026-10-20: "fix this PR screen alwys tells "gitlab.apps.ocp-nonprod-01.hodomain.local: 500 Internal Server Error"".

## Problem

The screen showed the host's own message for a failed list, and the list failed every time with a 500. A GitLab answers `GET …/merge_requests` with a 500 when it cannot serialise one merge request in the page, or when the query runs past its time limit; one bad merge request fails the whole list. The message read as Spagitty's fault, and nothing was shown.

## Fix

- **A server error on a merge request list is gone round.** The list's simple view (`view=simple`: ids and titles, without the fields that fail) is asked for, then each merge request on its own, six at a time, in the list's order. One that still fails is left out; the rest are shown. Only when the simple view fails too is the server's error the answer. Both lists do this: a project's (Pull requests) and the host's (Review's *All my repos*).
- **A server error says what it is:** "500 Internal Server Error — the server failed, not the request. Try again later; if it keeps happening, the server's administrators can find why in its logs." A refusal (4xx) is not retried.

## Acceptance criteria

- A list that answers 500 while its merge requests read one at a time shows those merge requests, without the ones that still fail.
- A list whose simple view fails too shows the server's error, saying it is the server's.
- A 403 is not retried.

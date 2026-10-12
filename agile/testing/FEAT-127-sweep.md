<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-127 — Sweep

**Item:** [FEAT-127](../items/FEAT-127-the-review-room-says-the-pull-request-moved-and-pulls-the-latest.md)

None of these rows has been run. The behaviour is covered by mounted tests with the host's answers fixed; nothing was looked at in a browser or in the application, and no real host was asked.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT127-01 | A pull request on GitHub open in the room | Push a commit to its branch from elsewhere; switch away from Spagitty and back | The notice appears across the top with **Pull latest**. The files shown have not changed | P1 | Not run. |
| SWEEP-FEAT127-02 | As above, without leaving the window | Wait three minutes | The notice appears | P2 | Not run. |
| SWEEP-FEAT127-03 | The notice showing; two files viewed, one of which the push changed | Press **Pull latest** | The room reads again. The changed file is unviewed and is where the room lands; the other keeps its tick; the notice is gone | P1 | Not run. |
| SWEEP-FEAT127-04 | The notice showing; a pending comment on a line the push did not touch | Press **Pull latest** | The comment is still pending, marked as written before the push | P1 | Not run. |
| SWEEP-FEAT127-05 | The notice showing | Start a comment and leave the box open | **Pull latest** is disabled and says to save or cancel the comment first | P2 | Not run. |
| SWEEP-FEAT127-06 | A merge request on GitLab open in the room | Repeat 01 and 03 | The same, with *GitLab* in the sentence | P1 | Not run. |
| SWEEP-FEAT127-07 | A pull request open in the room | Disconnect the network; switch away and back | Nothing is said; the room is as it was | P2 | Not run. |
| SWEEP-FEAT127-08 | An agent reviewing the pull request | Push, then press **Pull latest** | The agent stops and asks whether to carry on, as it does today when the head moves | P2 | Not run. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-076 — Sweep

**Item:** [BUG-076](../items/BUG-076-merger-says-pushed-when-the-push-failed.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG076-01 | A pull request from a branch of this repository that conflicts with its target | Open it in Merger, *Resolve the conflicts here*, resolve, merge, *Push <branch>* | The button says *Pushed* after git finishes; the host shows the new commit on the pull request | P1 | Not run. |
| SWEEP-BUG076-02 | As above, after the push | Press *Fetch* on the toolbar | The fetch runs; nothing says a fetch or push is already running | P1 | Not run. |
| SWEEP-BUG076-03 | As 01, with somebody else's commit pushed to the pull request's branch after it was fetched | *Push <branch>* | git's rejection is shown under the button; it does not say *Pushed* | P1 | Not run. |
| SWEEP-BUG076-04 | A pull request from a fork that conflicts | Resolve it here, then *Push <branch>* | It says where the branch is not known and to push it yourself; nothing is pushed | P2 | Not run. |

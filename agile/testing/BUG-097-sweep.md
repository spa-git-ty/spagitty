<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-097 — Sweep

**Item:** [BUG-097](../items/BUG-097-a-push-opens-the-credential-helpers-sign-in-window.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG097-01 | A GitLab account connected in Settings, with a token that can push; a new local branch | Push it from the toolbar | It is pushed and created on the remote; no sign-in window opens | Not run; the author is checking it |
| SWEEP-BUG097-02 | Same | Open the command log | The push is listed without the token | Not run |

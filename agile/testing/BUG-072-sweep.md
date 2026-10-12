<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-072 — Sweep

**Item:** [BUG-072](../items/BUG-072-a-remote-agent-with-no-key-is-told-bad-request.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG072-01 | A new remote agent form, no key typed | Press *Test* | It says to enter the agent's API key first; nothing is sent | P1 | Not run. |
| SWEEP-BUG072-02 | A saved remote agent whose keychain entry was deleted | Press *Test*, then start a Review assignment with it | Both say the key is not in the keychain and to enter it again | P1 | Not run. |
| SWEEP-BUG072-03 | An OpenAI-compatible endpoint with no key | Press *Test* | It is not refused for a missing key | P2 | Not run. |

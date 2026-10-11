<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-071 — Sweep

**Item:** [BUG-071](../items/BUG-071-a-gemini-agent-gets-bad-request.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG071-01 | A Google AI Studio key and a current Gemini thinking model | Settings › Agents, attach the key, press *Test* | It reports ok with a time | P1 | Not run. |
| SWEEP-BUG071-02 | As above, with a current non-thinking Gemini model | Press *Test* | It reports ok with a time | P1 | Not run. |
| SWEEP-BUG071-03 | The thinking model as the Review agent | Start a full Review assignment on a pull request | Every step finishes; no 400 | P1 | Not run. |
| SWEEP-BUG071-04 | A key that is wrong | Press *Test* | It says Google's own sentence, not *Bad request* | P2 | Not run. |

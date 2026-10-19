<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-075 — Sweep

**Item:** [BUG-075](../items/BUG-075-a-reply-that-cannot-be-read-is-taken-as-empty.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG075-01 | A remote agent on a local OpenAI-compatible server (Ollama or LM Studio) | Start a review, then stop the server while the agent is waiting for a reply | The assignment says the reply was cut off, not that the model said nothing | Unit tests; desktop check not run |
| SWEEP-BUG075-02 | A remote agent whose key is wrong | Press **Test** in Settings → Agents | The provider's status and sentence are shown, as before | Existing behaviour; desktop check not run |
| SWEEP-BUG075-03 | Any remote agent | Run one review to the end | The review works as before; nothing new is reported | Desktop check not run |

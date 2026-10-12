<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-071 — Plan

**Item:** [BUG-071](../items/BUG-071-a-gemini-agent-gets-bad-request.md)

Pull request #60 is the first half: it keeps Gemini's thought signature and
sends it back, and strips the `default_api:` prefix from tool names. Reviewing
it found two faults of its own: the signature was also written *inside*
`functionCall`, where Google has no such field, and one signature was copied
onto every call and thought in the turn when, of calls made in parallel, Google
signs only the first. Calls made one after another each carry their own.
Both are corrected so a signature goes back on exactly the part it came on.

Then the other ways Google refuses a request, all in `crates/spagitty-core/src/models.rs`:

- a tool with no properties is declared without `parameters`, and an empty
  `required` list is dropped (Google refuses an object with no properties);
- an assistant turn with no parts is not sent;
- a reply with no `parts` (cut off while thinking) is read as an empty turn
  that stopped on length, not as "not a reply";
- an error body wrapped in a one-item list still gives Google's own sentence;
- Settings *Test* gives a Google model 1,024 output tokens, not 16, so a
  thinking model can finish thinking before it answers.

Test the request shape of a multi-turn exchange with a signed call, the tool
declarations, the empty turn and the list-wrapped error without a network. The
run against the real API needs an AI Studio key and is in the sweep.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-071 — A Gemini agent from Google AI Studio gets "Bad request"

**Status:** Fixed — shipped in 2.1.0, through pull request #63. It has not been run against the real API; the manual sweep is still owed.
**Screens:** 1K, 1R, 1S.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

A remote agent set up with a Google AI Studio key and a Gemini model cannot be used: Google answers 400 Bad Request.

## What is known

Pull request #60 (*preserve Gemini thought signatures and canonicalize tool names*) fixes one 400: *Function call is missing a thought_signature in functionCall parts*. Gemini 3 and thinking models send a signature with each tool call and refuse the next turn unless it comes back. On `main`, `crates/spagitty-core/src/models.rs` (`google_message`, `read_turn`) drops it. The pull request also handles the `default_api:` prefix Gemini puts on tool names. It has no item or testing records, and it conflicts with `main` in `CHANGELOG.md`.

If the author's 400 says something else, check these too:

- an assistant turn with no text and no calls is sent back with empty `parts`;
- Settings *Test* allows 16 output tokens, which a thinking model can spend before it says anything.

## Acceptance criteria

- With an AI Studio key, Settings *Test* and a full Review assignment both succeed on a current Gemini thinking model and a current non-thinking one.
- A 400 from Google shows Google's own sentence, not only *Bad request*.
- The request shape for a multi-turn exchange with tool calls has a unit test.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Order:** rebase and review pull request #60 (TASK-062), add the records, then run the criteria above against the real API.
- **Where:** `crates/spagitty-core/src/models.rs`, `crates/spagitty-farm/src/assign/remote.rs`.
- **Branch:** pull request #60's branch `fix/gemini-thought-signatures`, renamed or recorded as this item's.

## Fix

Pull request #60's signature and `default_api:` handling, corrected so a signature goes back only on the part Google signed and never inside `functionCall`. Also: a tool with no properties is declared without `parameters`; an empty assistant turn is not sent; a reply with no parts is read as an empty turn; an error wrapped in a list gives Google's sentence; *Test* gives a Google model 1,024 output tokens. The first two acceptance criteria wait on a run with an AI Studio key (see the sweep). The third has its unit tests.

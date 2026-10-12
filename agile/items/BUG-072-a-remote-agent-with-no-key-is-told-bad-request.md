<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-072 — A remote agent with no key is told "Bad request", not that its key is missing

**Status:** Open — fixed on `task/TASK-063-audit-the-merged-pull-requests` with its unit test; Open until the sweep runs in the real app (2026-10-12).
**Screens:** 1K, 1R, 1S.
**Raised by:** found while working BUG-071 on 2026-10-12: an agent whose key was never saved, or was lost from the keychain, is sent without one.

## What happens

A remote agent (Anthropic, OpenAI or Google) whose key is not in the keychain is still sent to the provider. Google answers 400 and the others 401, and the person is told the request was bad, which sends them looking at the model or the prompt instead of the key. It looked like BUG-071 to the author.

## Acceptance criteria

- Settings *Test* with no key typed and none saved says *Enter <name>'s API key first.*
- *Test* on a saved agent whose key is no longer in the keychain, and a Review or Merger assignment that starts with such an agent, say *<name>'s API key is not in the keychain. Enter it again in Settings › Agents.* No request is sent.
- An OpenAI-compatible endpoint, which may be open, is not stopped.
- The check has a unit test.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src-tauri/src/agents.rs`, `require_key`, used by the Settings probe and by `assignment_start`.

## Fix

`require_key` stops a provider that wants a key and has none, with the reason above, before anything is sent. A compatible endpoint is let through with an empty key. The second and third criteria are covered by the unit test; the first two, in the real app, are in the sweep.

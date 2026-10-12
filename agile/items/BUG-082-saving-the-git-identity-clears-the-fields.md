<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-082 — Saving the git identity clears the fields

**Status:** Open — implemented and tested on `bugfix/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why` for 2.2.3; awaiting merge and the desktop sweep.
**Screens:** 1K.
**Raised by:** the author, 2026-10-12: global and repository name and email disappear after Save.

## Cause

`set_identity` writes through Git and then reads the session's cached configuration from when the repository was opened. The response contains the old value, which resets the saved field. Separately, saving either field resets both drafts, throwing away an unsaved edit in the other field.

## Acceptance criteria

- Saving name or email returns the value just written in the selected scope, and a later read sees it without reopening the repository.
- Global values and repository overrides remain separate.
- Saving one field preserves an unsaved edit in the other, so both can be saved in sequence.
- Edits made while a save is pending remain available for the next save.
- Clearing still unsets a key, and a failed save keeps the draft and reports the error.

## Implementation

The identity command discovers a fresh repository handle before reading its configuration. The Settings store refreshes only the submitted field, provided its scope and draft have not changed while the write was pending.

## Validation

The new command test reproduces a stale global response with a repository kept open, using an isolated configuration directory. Store tests reproduce the loss of the other draft in both scopes. Mounted component tests verify that both fields remain visible after saving in sequence.

See [automated checks](../testing/BUG-082-automated.md) and [sweep](../testing/BUG-082-sweep.md).

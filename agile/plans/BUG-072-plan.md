<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-072 — Plan

**Item:** [BUG-072](../items/BUG-072-a-remote-agent-with-no-key-is-told-bad-request.md)

Both paths that send a remote agent's key read it with an empty default: the
Settings probe (`Probe::endpoint`, through `key_for`) and a Review or Merger
assignment (`assignment_start`, from the keychain). An empty key goes out as an
empty header, which Google refuses as a 400 and the others as a 401.

One function, `require_key(provider, name, key, saved)`, is used by both. A
provider other than the OpenAI-compatible one that has no key is refused with
kind `refused`: *Enter <name>'s API key first.* when the person has not saved
one, *<name>'s API key is not in the keychain. Enter it again in Settings ›
Agents.* when one was saved and is gone. A compatible endpoint keeps working
without a key, since a self-hosted one may be open.

Unit-test the four cases (no key and unsaved, no key and saved, a key, a
compatible endpoint with none) without a network or keychain.

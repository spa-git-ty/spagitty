<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-072 — Automated test record

**Item:** [BUG-072](../items/BUG-072-a-remote-agent-with-no-key-is-told-bad-request.md)

| Suite | Cases |
| --- | --- |
| `src-tauri/src/agents.rs` | A Google provider with no key and a saved agent is refused as `refused`, naming the keychain; an Anthropic provider with no key and nothing saved is told to enter it first; a key passes through unchanged; a compatible endpoint with no key is let through. |

## Results — 2026-10-12

- `cargo test -p spagitty --lib a_provider_with_no_key`: 1 passed.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-082 — Plan

**Item:** [BUG-082](../items/BUG-082-saving-the-git-identity-clears-the-fields.md)

1. Reproduce stale save responses with the real Tauri commands while the repository stays open. Use the test application's temporary configuration directory for global writes.
2. Read the current configuration from disk, including subsequent reads and reads after applying an identity profile.
3. Refresh the saved draft only, preserving the other field and any edit made while the write was pending.
4. Test both scopes at the command, store and mounted component layers, including unset behavior and error handling.
5. Include this fix with BUG-081 in the 2.2.3 pull request and run all required checks and builds.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-097 — Plan

**Item:** [BUG-097](../items/BUG-097-a-push-opens-the-credential-helpers-sign-in-window.md)

1. `shell.rs`: `sign_in_with_accounts` keeps the connected hosts and logins; `command()` and `clone_start` call `sign_in` for a command that reaches a remote, which reads each token from the keychain (cached) and `sign_in_as` adds the cleared and replaced `credential.https://<host>.helper` with the token in the environment.
2. `accounts.rs`: `share_with_git` on every save, and once at start-up in `lib.rs`.

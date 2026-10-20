<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-097 — Automated test record

**Item:** [BUG-097](../items/BUG-097-a-push-opens-the-credential-helpers-sign-in-window.md)

- `crates/spagitty-core/src/shell.rs`: `git credential fill` with a helper standing in for the system's, and an account for `gitlab.example.test`: that host answers with the account's login and token and the system's helper is not asked; `github.com` still gets the system's answer. Fetch, push, clone and pull sign in; status, commit and `rm --cached` do not.

## Results — 2026-10-20

- `cargo test -p spagitty-core --lib shell::tests::`: 17 passed.

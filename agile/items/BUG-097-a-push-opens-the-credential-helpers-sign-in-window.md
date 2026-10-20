<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-097 — A push opens the credential helper's sign-in window instead of using the connected account

**Status:** Fixed — on `fix/BUG-097-sign-in-and-gitlab-list`; unmerged. Covered by a test that runs git's own credential machinery; the push to the author's GitLab has not been run (the author is checking it).
**Branch:** `fix/BUG-097-sign-in-and-gitlab-list`.
**Screens:** every push and fetch: the toolbar, Branches, Merger, Untrack.
**Raised by:** the author, 2026-10-20: "check push to upstream ( if remote branch isn't exists ) it didn't use my saved or local creds it opend a gitlab ui username and token to do it !"

## Problem

Spagitty pushes and fetches through `git`, and git asks its configured credential helper for a user name and password. On Windows that is Git Credential Manager, which, when it has nothing it can use for the host, opens its own GitLab sign-in window asking for a user name and token. The person had already given Spagitty a token for that host in Settings › You › Accounts, and Spagitty never offered it to git.

## Fix

The application tells the core which accounts are connected, when it starts and whenever one is connected or removed. Every git command that reaches a remote — fetch, push, pull, clone, `ls-remote`, `submodule` — is given, for each connected host:

- `credential.https://<host>.helper=` — empty, which clears the helpers git gathered for that host, the system's included;
- `credential.https://<host>.helper=!f() { … }; f` — a helper that answers `get` with the account's login and token.

The token reaches the helper through the environment, so it is in neither the command line nor Spagitty's record of what ran. A host with no connected account is left to the helpers git is configured with.

The token has to be allowed to push: on GitLab `write_repository` (or `api`), on GitHub `repo` or *Contents: write*. A token that is not says so in git's own refusal.

## Acceptance criteria

- With an account connected for the host, a push or fetch asks no credential helper and opens no window.
- A host with no account is asked of the configured helpers as before.
- The token is not in any command line or in the command log.

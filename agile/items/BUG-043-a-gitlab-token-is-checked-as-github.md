<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-043 — A GitLab token is checked as GitHub

**Status:** Fixed.
**Branch:** `bugfix/BUG-043-a-gitlab-token-is-checked-as-github`
**Screens:** Settings (Accounts).
**Raised by:** the author, adding a token for a self-hosted GitLab once BUG-042
let it connect: "Field 'viewer' doesn't exist on type 'Query'".

## Problem

The Accounts section always sent `kind: 'gitHub'` to `forge_connect`, whatever
host was typed. The token was proved with GitHub's GraphQL `viewer` query,
which GitLab does not have. `forge::kind_of` already read the kind from a
hostname for remotes, but connecting an account did not ask it.

## Scope

- `forge_connect` takes the kind from the hostname when the hostname names a
  forge, and uses the one it was sent only for a host that does not.

## Acceptance criteria

- A token for a `gitlab.` host is proved against GitLab's `/user` and connects.
- `github.com` still connects as GitHub.

## Tests

`forge::tests::a_self_hosted_gitlab_is_known_by_its_hostname`. On Windows 11:
`cargo test -p spagitty-core --lib forge` (96 passed), `bun run test --
src/lib/settings` (134 passed).

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-037 — Plan

**Item:** [`agile/items/BUG-037-core-tests-read-the-machines-git-config.md`](../items/BUG-037-core-tests-read-the-machines-git-config.md)

## Approach

One more line in `Fixture::empty`, beside the identity and signing it already
pins: `git config core.autocrlf false`. Every fixture starts there, so every
test repository has a line-ending policy of its own and the machine's system
config no longer reaches it. A test about line endings sets its own value after
`empty`, which wins.

Not `GIT_CONFIG_NOSYSTEM` on the fixture's commands: the code under test runs
`git` itself (stash, rebase, merge) and opens the repository with `gix`, and
neither would see an environment variable set only on the fixture's own calls.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/fixture.rs` | `core.autocrlf=false` in every fixture. |

## Risks and rollback

- None for the application: `fixture` is test support, compiled only for tests
  and behind the `fixture` feature. Rollback is a revert.

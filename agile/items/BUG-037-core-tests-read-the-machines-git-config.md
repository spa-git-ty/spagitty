<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-037 — Core tests read the machine's git config

**Status:** Fixed.
**Branch:** `bugfix/BUG-037-core-tests-read-the-machines-git-config`
**Screens:** none. It is the test suite, on Windows.
**Raised by:** the review of 2026-09-30: ten `spagitty-core` tests had failed on
native Windows since the first native run on 2026-09-29.

## Problem

`cargo test -p spagitty-core` on Windows 11 fails ten of 534 tests: three in
`conflicts`, two in `ops`, one in `stash`, four in `work`. Eight say the same
thing:

```
assertion `left == right` failed
  left: "alpha\r\nbeta\r\nentry 1\r\n"
 right: "alpha\nbeta\nentry 1\n"
```

`Fixture` builds its repositories with the `git` binary and a local config of
its own — an identity, no signing, no gc — but leaves everything else to the
machine. Git for Windows ships `core.autocrlf=true` in its system config, so
on Windows every fixture checks its files out with CRLF, and every test that
reads a working file back after a checkout, a discard, a stash or a conflict
side sees `\r\n` where it wrote `\n`. The stash test's status disagreed for the
same reason. The pipeline runs these tests on Linux only, where there is no such
system config.

The other two failures, both interactive rebase, are not this. They are a
defect in the application itself, recorded as a bug of its own.

## Scope

- `Fixture::empty` sets `core.autocrlf=false` in each repository's own config,
  which outranks the system's for `git` and `gix` alike.

## Non-scope

- The application's handling of `core.autocrlf`, which is the user's setting
  and is honoured.
- Running the Rust suite on Windows in the pipeline (TASK-033).

## Acceptance criteria

- The eight line-ending tests and the stash test pass on native Windows with
  Git for Windows' default system config, and still pass on Linux.

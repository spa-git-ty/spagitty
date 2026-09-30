<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-037 — Automated test record

**Item:** [`agile/items/BUG-037-core-tests-read-the-machines-git-config.md`](../items/BUG-037-core-tests-read-the-machines-git-config.md)

## What was tested

The existing suite is the test: nothing new is asserted, the fixtures stop
depending on the machine.

## Test command and output

On Windows 11, Git for Windows with `core.autocrlf=true` in
`C:/Program Files/Git/etc/gitconfig`:

```
$ cargo test -p spagitty-core --no-fail-fast      # before
test result: FAILED. 524 passed; 10 failed

$ cargo test -p spagitty-core --no-fail-fast      # after
test result: FAILED. 532 passed; 2 failed
    ops::tests::a_rebase_that_conflicts_stops_and_leaves_state_to_read
    ops::tests::aborting_puts_the_branch_back_and_clears_the_state
```

The two that remain are a defect in the application rather than the fixtures:
interactive rebase could not start on Windows. It has its own item, and with it
fixed the crate passes on Windows, 534 of 534.

In WSL (Arch Linux): `cargo test --workspace`, unchanged, all passing.

## What is not covered automatically

Nothing: the change is to test support.

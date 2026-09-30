<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-048 — Automated test record

**Item:** [`agile/items/TASK-048-commands-off-the-main-thread.md`](../items/TASK-048-commands-off-the-main-thread.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src-tauri/src/commands.rs` | Only the eleven ordered writers are plain `#[tauri::command]` — read from the source, so a new command on the main thread fails it. An open overtaken by a later open is refused with `Superseded` and the later repository stays open. An open overtaken by a close is refused and nothing is open. An open after a close opens. The eight session tests pass unchanged. |
| `src/lib/repo.test.ts` | The later of two opens is kept when the earlier answers last; the failure of an overtaken open is not reported; an open overtaken by a close leaves nothing open and `busy` clear. |

Each new test was checked against the unfixed code: with one command put back
on the main thread the source test fails naming it, and with the store's guard
removed the three store tests fail.

## Test command and output

In WSL (Arch Linux), on the item's tree:

```
$ cargo fmt --all -- --check
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile
$ cargo test --workspace --no-fail-fast
test result: ok. 101 passed; 0 failed   (spagitty)
test result: ok. 534 passed; 0 failed   (spagitty-core)
test result: ok. 331 passed; 0 failed
test result: ok. 53 passed; 0 failed
```

On Windows 11:

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  2983 passed (2983)
```

One earlier run of `bun run test` failed `tools/dev-styles.test.ts` with
`spawnSync … ETIMEDOUT`: the test starts a Node process with a 30 s deadline,
and a full parallel suite on this machine can starve it. It passes alone and
on the rerun, and it fails the same way on the branch before this change.

Native `cargo test -p spagitty` builds but its test binary does not start on
Windows (`STATUS_ENTRYPOINT_NOT_FOUND`), which is why the Rust suite is run in
WSL.

## What is not covered automatically

Whether the window keeps painting on a large repository. That is what the
sweep and the release build are for.

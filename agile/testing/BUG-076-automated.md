<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-076 — Automated test record

**Item:** [BUG-076](../items/BUG-076-merger-says-pushed-when-the-push-failed.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/origin.test.ts` | The resolved branch is pushed through the network store to the remote it is on, with the refspec, and the push command is not called directly; *Pushed* is not set until the wait ends; a rejected push shows git's words and is not marked pushed; a push that could not start says why; a push waits its turn while a fetch or a push is running; a branch the remote is not seen to have is not pushed and the reason names it; nothing is pushed when it was not resolved here. |
| `src/lib/network/store.test.ts` | `whenDone` answers only when git has finished a push, true when it went well; false for a rejected push, with git's words kept in `error`; at once when nothing is running. |

The network store is replaced in `origin.test.ts`, not the push command, so a push that bypasses the store fails the first case.

## Results — 2026-10-12

- `bunx vitest run`: 185 files, 3851 tests, all passed. `bun run check`: zero errors, zero warnings.

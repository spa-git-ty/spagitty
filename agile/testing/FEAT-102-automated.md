<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-102 — Automated test record

**Item:** [`agile/items/FEAT-102-resolve-every-conflict-every-way.md`](../items/FEAT-102-resolve-every-conflict-every-way.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/merger/detail.rs` | A block found after its context, an empty block placed after it, a missing one not found; every region's line on each side — the second moved down by a line one side added — and the commit on each side that wrote it; reading the conflicts leaves the index, the refs and `git status` as they were; a deleted file has one side, no merged text and no regions. |
| `crates/spagitty-core/src/conflicts.rs` (`settle_tests`) | Settling with text writes it and marks it resolved; text with markers is refused and nothing is staged; taking a side takes it and marks it resolved; taking the side that deleted it deletes it. |
| `src-tauri/src/merger_state.rs` | A key is a file under `merges`; anything but short lowercase hex is refused; choices read back, a non-object is refused, forgetting works. |
| `src/lib/resolver/model.test.ts` | Lines split on `\n` alone; regions read with their base, strictly; each region found in each side's file; what the backend found wins; a deleted file is one region chosen whole and lands as the side; every choice's lines and origins; which lines will not land; Pick lines starts all ticked and turns one over; a hand edit starts from the result or both sides; labels, badges and the placeholder; the result text only once all are chosen, `\r\n` kept; later lines renumbered by earlier choices; each side's context by its own numbers; counts, all from a side; fingerprints; why each conflicts. |
| `src/lib/conflicts/store.test.ts` | Every conflicted file read whole, ours as A; a deleted file chosen whole; an error empties the list; a choice kept across a reload while the file is the same and dropped once it is not; Mark resolved waits for every region then writes and stages; a whole side for a whole-file conflict; a file with no markers left resolved as it is; a refusal reported and read again; continue and abort. |
| `src/routes/conflicts/page.test.ts` | The three columns and *Choose what lands here*; Continue blocked until every file is marked resolved; Take ours then Mark resolved writes and stages the result; Continue and Abort; an error, then Refresh to nothing conflicted. |
| `src/routes/merge/resolve.test.ts` | Resolve opens on the first file with the header, the column roles, the commits, and every way out; Complete merge disabled until all are resolved; result badges B, B, A for Both B first, and the next conflict renumbered by three; Pick lines' checkboxes and the faded line; Edit by hand from both sides; Base, the pill's Take and Next unresolved across files, the choices kept, the commit dialog's rows, the resolutions sent, the kept choices forgotten after landing; a kept choice applied only to the same conflict, and choices surviving a trip to the plan; Abort asks once something was chosen, returns to the plan and writes nothing. |
| `src/lib/conflicts/actions.test.ts` | Unchanged for abort; the draft questions went with the draft. |

## Test command and output

On Windows 11, git 2.55:

```
$ bun run check
COMPLETED 1254 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Test Files  162 passed (162)
     Tests  3383 passed (3383)
$ cargo test -p spagitty-core
test result: ok. 637 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
$ cargo check -p spagitty
(clean)
```

The frontend count is lower than FEAT-101's because the old Conflicts panes,
pager and draft were removed with their tests (`panes.test.ts` and most of
`store.test.ts`). `src-tauri`'s own test binary does not load on Windows; its
`merger_state` tests are run in WSL (TASK-056's record carries the run).

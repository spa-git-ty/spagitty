<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-092 — Automated test record

**Item:** [`agile/items/FEAT-092-conflict-fix-origin.md`](../items/FEAT-092-conflict-fix-origin.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/remerge.rs` | A remerge diff read file by file, numbered as in the merge; a conflict is the lines written for it and what each side had, a side kept as it was counting as much as a line added, and lines added away from any conflict a place with no sides; a three-way marker's base kept out of both sides; against real git, a branch that merged `main` through a conflict in two files and then added a line above the fix: the merge found, the fix followed to its line at the head, the target's and the branch's sides told apart, the merge's subject, and the file changed only by the resolution not counted as the author's; a pull request with no merges has none. |
| `src/lib/review/fixes.test.ts` | The changed part a merge wrote framed as a conflict fix and only that one, its exact lines marked; a side opened under the card's header; a part with no line a fix wrote left alone. |
| `src/routes/review/room.test.ts` | The fixes asked for from the merge base, head and target; the card's heading and the merge it names; the file marked; the fixed line marked; each side opened and closed; the files and merges kept in the record; the files filtered by Author and Conflict fixes, and the legend; the legend saying when they could not be looked for, with no card. |
| `src/lib/ui/flat.test.ts` | `--resolve` and `--resolve-soft` are published in `app.css`. |

## Test command and output

On Windows 11, git 2.55:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3138 passed (3139)
$ cargo test -p spagitty-core
test result: ok. 581 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut.

In WSL (Arch), git 2.55:

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
$ cargo test -p spagitty-core --lib remerge
test result: ok. 5 passed; 0 failed
```

## What is not covered automatically

A real pull request on GitHub and on the author's work GitLab, and git older
than 2.36. See the sweep.

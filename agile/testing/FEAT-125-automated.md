<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-125 — Automated test record

**Item:** [FEAT-125](../items/FEAT-125-a-choice-on-a-conflict-can-be-cancelled-and-pick-lines-starts-empty.md)

| Suite | Cases |
| --- | --- |
| `src/lib/resolver/model.test.ts` | Pick lines starts with no line ticked, and a tick turns one over. An empty pick does not settle its region: it has no result lines, is not counted as resolved, gives no result text and nothing to write, and a hand edit started from it has both sides, as from nothing. One ticked line settles it. A hand edit cleared to nothing still settles. |
| `src/routes/merge/resolve.test.ts` | Mounted, in Merger: Pick lines opens with no box ticked, the card still says *Unresolved* and *Tick the lines to keep*, no line is faded and the count has not moved; one tick makes it *Picked lines*, fades the other four and moves the count. *Cancel* is absent before a choice, undoes a side, and leaves Pick lines; *Cancel all* is absent with nothing chosen, undoes *All from main* for the file's two conflicts, and is absent again. |

## Not covered

- The question *Cancel all* asks when a hand edit would go.
- That an agent's proposal is kept when Pick lines is opened and nothing ticked (`personChose`).
- Conflicts (1D), which mounts the same resolver; its own page tests pass unchanged.

## Results — 2026-10-12

- `bunx vitest run`: 189 files, 4074 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
- No Rust changed, so `cargo test` was not run.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-107 — Automated test record

**Item:** [`agile/items/FEAT-107-hooks-you-can-see-and-skip.md`](../items/FEAT-107-hooks-you-can-see-and-skip.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/hooks.rs` | Samples are not hooks; plain hooks with their script and commit membership; Husky shows `.husky/<name>` and drops bare stubs; lefthook brings its config; the switch is kept in `.git/config` and leaves no trace when back on; a real failing pre-commit streams its line, and a skipped commit runs not even `post-commit`. (Native Windows.) |
| `src/lib/hooks/hooks.test.ts` | Commit hooks in git's order and none when off; ask → run → log fed by its own token only → passed; Skip; Cancel keeps the message; the per-commit skip asks nothing and resets; no question without hooks; a failing hook keeps the log and the message; the three-way dialog's buttons; the run window locked while running; the view's groups, scripts, switch and lefthook config. |
| `src/lib/changes/store.test.ts`, `src/routes/changes/page.test.ts` | The commit call's new arguments. |
| All | `bun run check` 0/0; `bun run test` 3467 passed. |

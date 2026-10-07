<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-055 — Automated test record

**Item:** [`agile/items/BUG-055-branch-where-you-point.md`](../items/BUG-055-branch-where-you-point.md)

| Suite | Cases |
| --- | --- |
| `src/lib/chrome/chrome.test.ts` | No Clone or Rebase; Branch at the selected commit on the graph, HEAD elsewhere; Stash stays on the screen. |
| `src/lib/graph/naming.test.ts` | Enter asks, the field survives the question taking focus, OK creates; Cancel keeps the field and the name. |
| `src/lib/graph/actions.test.ts` | Quick stash: `WIP on <branch>`, untracked too; refused; nothing to stash. |
| `src/routes/review/page.test.ts` | All my repos with no account: the no-account state, nothing asked of the host. |
| `crates/spagitty-core/src/ignore.rs` | Ignored output does not count; a file git would notice does. |
| `src-tauri/src/watch.rs` | A working-tree edit emits a worktree change and ignored output emits nothing; a ref path with Windows separators is a ref change. (Native Windows.) |
| All | `bun run check` 0/0; `bun run test` 3487 passed. |

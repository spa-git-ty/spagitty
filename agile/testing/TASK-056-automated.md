<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-056 — Automated test record

**Item:** [`agile/items/TASK-056-the-rail-as-the-author-chose-it.md`](../items/TASK-056-the-rail-as-the-author-chose-it.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/nav.test.ts` | The rail's order with Stash, Tags and Reflog after Log; the tools shown on a quiet day; Stash, Tags and Reflog each the active row on its own screen, and Branches on none of them. |
| `src/lib/chrome/chrome.test.ts` | The rail as drawn; each refs screen marks its own row; Merger's dot while a merge started there has unresolved conflicts, gone once every one is resolved. |
| `src/routes/merge/page.test.ts` | The pair the graph's drag asks for is the one Merger opens on, the dragged branch coming in. |

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1253 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Test Files  163 passed (163)
```

`src-tauri`'s own tests (FEAT-102's `merger_state`) do not load on Windows;
in WSL (Arch) on 2026-10-06: `cargo test -p spagitty --lib merger_state` — 3 passed, 0 failed.

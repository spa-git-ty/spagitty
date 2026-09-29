<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-045 — Automated test record

**Item:** [`agile/items/TASK-045-a-rail-of-five.md`](../items/TASK-045-a-rail-of-five.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/nav.test.ts` | The new order; the divider before Settings; the group boundaries; `isShown` on a quiet day, with conflicts, on an off-rail screen, and for Badges with the layer off and on; `navRows` filters; `isItemActive` for the four refs routes and not elsewhere. |
| `src/lib/chrome/chrome.test.ts` | The rail draws six rows; Conflicts after Working copy when conflicted; Log, Rebase and All repositories shown and active while open; Branches active on all four refs routes; no Badges while the layer is off. The page store is now settable per test. |
| `src/lib/delight/store.test.ts` | Off: no queue, no notice, no pulse, the badge still earned; shame stays quiet. |
| `src/routes/settings/page.test.ts` | No God mode chip by default; every other section still reachable. |
| `src/routes/stash/page.test.ts` | Finds the Stash action rather than the Stash tab. |
| `src-tauri/src/settings.rs` | `Off` is the default; a file naming a personality keeps it; `off` is stored as `"off"`. |

### Tests changed, and why

`nav.test.ts` asserted the fourteen-row order, the divider before All
repositories, and All repositories in the `app` group. Those were the design;
this item changes the design, so they now assert the new one. The other
TASK-041 assertions (a group per run, the Farm alone, headings) pass unchanged.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1164 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Test Files  137 passed (137)
     Tests  2936 passed (2936)

$ cargo test -p spagitty --lib settings
(pending: the Rust toolchain was still installing)
```

## What is not covered automatically

How the rail looks with a row appearing and leaving. That is in the sweep.

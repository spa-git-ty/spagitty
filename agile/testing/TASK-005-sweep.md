<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-005 — Manual sweep

**Item:** [`agile/items/TASK-005-branch-coverage-floor.md`](../items/TASK-005-branch-coverage-floor.md)

TASK-005 changed no production code — it is a test-only change. The sweep is
therefore short and is about the **gate**, plus a spot-check that the components
the new tests describe still behave that way in a running application, since a
test can only ever assert what it was told to.

Run the application from this branch. Tester fills the **Result** column.

---

## TASK-005-T1 — The coverage gate passes

**Priority:** high
**Preconditions:** a clean checkout of `task/TASK-005-branch-coverage-floor`, `npm ci` run.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Run `npx vitest run --coverage` | Exits **0**. No `ERROR: Coverage for … does not meet global threshold` line. |
| 2 | Read the coverage summary | All four metrics ≥ 70%. Branches is **≥ 71%**. |
| 3 | Read the test count | 980 tests across 55 files, all passing, none skipped. |
| 4 | Run `npm run check` | 0 errors, 0 warnings. |

**Result:**

---

## TASK-005-T2 — The threshold was not moved

**Priority:** high — this is the thing the item forbids.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open `vite.config.ts` and find `thresholds` | Still `{ statements: 70, branches: 70, functions: 70, lines: 70 }`. |
| 2 | `git diff origin/main -- vite.config.ts` | **Empty.** The gate was met, not lowered. |
| 3 | `git diff --stat origin/main` | Only `*.test.ts` files and `agile/` documents. No file under `src/lib` other than tests. |

**Result:**

---

## TASK-005-T3 — Confirmation dialogs still behave as described

**Priority:** high
**Preconditions:** a repository open with at least one branch other than the checked-out one.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Right-click a commit → *Reset* → *Hard* | A confirmation appears naming the commit, worded as destructive. |
| 2 | Press `Escape` | Dialog closes. **Nothing is reset** — the graph is unchanged and no success notice appears. |
| 3 | Repeat step 1, click **Cancel** | Same: nothing is reset. |
| 4 | Turn *Confirm history rewrite* **off** in Settings, repeat step 1 | The confirmation **still appears** — a hard reset always asks. |
| 5 | Right-click a commit → *Create branch here*, leave the name empty | The affirmative button is **disabled**. |
| 6 | Type a name, press `Enter` | The branch is created and checked out; a success notice names it. |
| 7 | Repeat step 5, press `Escape` | No branch is created. |
| 8 | Open a confirmation and click the greyed area **outside** the panel | Dialog dismisses, nothing happens. |
| 9 | Open a confirmation and click **inside** the panel, not on a button | Dialog stays open. |

**Result:**

---

## TASK-005-T4 — Notices appear and leave correctly

**Priority:** medium

| # | Step | Expected |
| --- | --- | --- |
| 1 | Perform a successful operation (e.g. check out another branch) | A notice appears bottom-right and **disappears on its own** after about four seconds. |
| 2 | Force a failure (e.g. push with no remote configured) | A notice appears carrying **git's own message**, and **stays** until dismissed. |
| 3 | Click the `×` on the failure notice | It closes. |
| 4 | Trigger a success, then quickly trigger a second operation | Only the newest notice is shown; the second is **not** removed early by the first one's timer. |

**Result:**

---

## TASK-005-T5 — Menus behave under the keyboard

**Priority:** medium

| # | Step | Expected |
| --- | --- | --- |
| 1 | Right-click a commit row | The menu opens at the pointer. |
| 2 | Press `ArrowDown` repeatedly past the last entry | Selection wraps to the first. Headings and separators are **never** selected. |
| 3 | Find a disabled entry (e.g. *Delete* on the checked-out branch) | It is **shown**, greyed, with a short reason beside it — not hidden. |
| 4 | Arrow through the menu | The disabled entry is **skipped**, never landed on. |
| 5 | Press `Enter` on a highlighted entry | The menu closes **first**, then the action runs — a dialog it opens is not dismissed by the menu's own outside-click. |
| 6 | Right-click, then press `Escape` | The menu closes and nothing runs. |
| 7 | Right-click, then click elsewhere in the window | The menu closes. |
| 8 | Right-click **near the bottom-right corner** of the window | The menu stays fully on screen — it is not cut off by the window edge. |
| 9 | Right-click, then resize the window | The menu closes rather than floating in the wrong place. |

**Result:**

---

## TASK-005-T6 — The size dials

**Priority:** medium

| # | Step | Expected |
| --- | --- | --- |
| 1 | Zoom in repeatedly to the maximum | It stops at 200% and does not go past. Label reads a round number — never `109.99999%`. |
| 2 | Zoom out repeatedly to the minimum | Stops at 100%. |
| 3 | Set text size and zoom to non-default values, close and reopen the app | Both are restored. |
| 4 | *Reset* the zoom | **Both** dials return to 100%, not just the zoom. |
| 5 | Edit `spagitty.scale.zoom` in `localStorage` to `banana`, reload | The app opens at 100% rather than a broken size. |

**Result:**

---

## TASK-005-T7 — The command palette

**Priority:** medium

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open the palette, type `cbh` | *Create branch here* is found; the matched letters are shown in **bold**, and each contiguous run is one bold piece, not one per letter. |
| 2 | Type something matching nothing | A line says nothing matches, quoting **what was typed**. |
| 3 | Find a command that cannot run right now | It is listed, greyed, with a short reason — not hidden. |
| 4 | Press `Enter` on it | Nothing happens; it is not runnable. |
| 5 | Arrow up and down | The highlight moves and the list **scrolls to keep it visible**. |
| 6 | Press `Escape` | The palette closes. |
| 7 | Scan the list | Each group heading appears **once**, above its first command. |

**Result:**

---

## TASK-005-T8 — BUG-007 reproduction (known defect, expected to fail)

**Priority:** low — recorded so it is not rediscovered as new.
**This ticket documents a defect TASK-005 found and did not fix.**

| # | Step | Expected once BUG-007 is fixed | Today |
| --- | --- | --- | --- |
| 1 | Open a naming prompt (*Create branch here*) | — | — |
| 2 | Without answering it, trigger an action that opens a **confirmation** (e.g. from the command palette) | The prompt is cancelled and **no branch is created**. | The prompt resolves `false`, which its caller's `=== null` guard does not catch. |
| 3 | Observe | No branch named `false`, no error. | Verify whether a branch is wrongly created. |

**Result:** Obsolete, 2026-10-19 — BUG-007 is fixed; its own sweep checks the fix.

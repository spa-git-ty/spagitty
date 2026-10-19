<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-142 — Automated test record

**Item:** [FEAT-142](../items/FEAT-142-a-notification-center.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The record | `src-tauri/src/inbox.rs` | What was said is there after a restart, newest first, with what was read; reading twice keeps the first time; mark all read can stay inside one repository; clearing forgets only what is older; a file that is not a record reads as an empty one; the stored shape is the one the screen reads. |
| Slices | `src/lib/notifications/center.test.ts` | Unread and category chips; this repository only; search on every word; unread counts; Today, Yesterday, then the weekday and date; one group per day, newest first; minutes and hours today, the clock before; an unknown kind drawn quietly. |
| Sources | `src/lib/notifications/center.test.ts` | The watcher's headline kept verbatim with its link; a failed fetch, pull or push kept in git's words and nothing else; a published release; ready to merge only where a person merges; a stuck task with its tries; a stopped or finished farm only on a change of status; the first look at a branch's runs says nothing, then a failure and a recovery are each said once; a running or cancelled run gives no verdict. |
| The drawer | `src/lib/notifications/center.test.ts` | Lists by day, Mark read clears the count and the `n new` line, Escape closes it; an empty chip says so. |
| Existing | `src/lib/notifications/watcher.test.ts`, `src/lib/ui/NoticeToast.test.ts`, `src/lib/chrome/chrome.test.ts` | The watcher's clock and the toast are unchanged; the title bar's window controls are still the three controls. |

## Not covered

- Not looked at in the running application; the author is checking it.
- The operating system's notification and the toast are unchanged code, not re-tested with the drawer.
- The checks feed's forge reads (`checkRuns`) are not run against a host; the comparison they make is.

## Results — 2026-10-18

- `cargo test -p spagitty --lib inbox::`: 6 passed, 0 failed (Windows).
- `bunx vitest run src/lib/notifications`: 29 passed, 0 failed.
- `bun run check`: 0 errors, 0 warnings.

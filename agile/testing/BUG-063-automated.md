<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Automated test record

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

| Suite | Cases |
| --- | --- |
| `src/lib/review/inbox.test.ts` | `chipsOf` names *conflicts with main* only for `mergeable: false`, not for `null` or `true`; `factsOf` puts the danger fact first. |
| `src/routes/review/page.test.ts` | The conflicting pull request's card carries the red tag and its preview the fact; the others carry neither. |
| `src/routes/review/room.test.ts` | The room's header says *Conflicts with main* for a conflicting pull request and nothing for one the host has not decided. |
| `src/lib/requests/requests.test.ts` | `RequestRow` shows *conflicts* for `false` and not for `null`. |
| `src/lib/requests/workspace.test.ts` | `merge()` refuses without calling the host; the workspace header carries the chip; *Merge* explains, offers no *Confirm Merge*, and *Resolve in Merger* presents `{ a: 'main', b: <branch>, into: 'b' }` and goes to `/merge`; the chip clears when a re-read reports the pull request mergeable. |

All seven new cases fail without the fix.

## Results — 2026-10-08

- `bunx vitest run`: 3,558 passed across 170 files.
- `bun run check`: zero errors, zero warnings.

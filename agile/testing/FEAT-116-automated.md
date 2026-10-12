<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-116 — Automated test record

**Item:** [FEAT-116](../items/FEAT-116-pull-requests-does-not-review.md)

| Suite | Cases |
| --- | --- |
| `src/lib/requests/files.test.ts` | The store offers no `review`, `reviewing`, `reviewError`, draft, reply, resolve or `role`; the panel's own tests (files, selecting) are unchanged. |
| `src/lib/requests/workspace.test.ts` | The workspace has no *Review*, *Reply in Review*, *Approve*, *Request Changes*, *Submit Review* or *Publish Review*, and no *Reviewer View* / *Developer View*; the diff shows the comments already on a line, with no composer, reply box, resolve button or comment trigger. |

## Results — 2026-10-11

- `bunx vitest run`: 3,746 passed. `bun run check`: zero errors, zero warnings.

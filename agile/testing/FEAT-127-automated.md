<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-127 — Automated test record

**Item:** [FEAT-127](../items/FEAT-127-the-review-room-says-the-pull-request-moved-and-pulls-the-latest.md)

| Suite | Cases |
| --- | --- |
| `src/routes/review/room.test.ts` | Mounted, with the host's answers fixed. Nothing is said while the host has the head that is read. When the host has another, the notice says so and nothing has been fetched for it; *Pull latest* fetches the new head, lists its files, records it, and the notice goes. A short id and a whole one for one commit are the same commit. A host that cannot be asked changes nothing. The host is not asked twice within thirty seconds by the window gaining focus, and is when asked to. With a comment being written, *Pull latest* is disabled and says why. |

## Not covered

- The three-minute timer. The asking it calls is covered; the waiting is not.
- That ticks on changed files are dropped and pending comments kept after *Pull latest*. That is what opening a pull request already does, and it is covered where it was built (FEAT-091, FEAT-093).
- GitLab. The room asks through the same call for both hosts.

## Results — 2026-10-12

- `bunx vitest run`: 189 files, 4099 tests, all passed.
- `bun run check`: zero errors, zero warnings. `bun run build`: succeeded.

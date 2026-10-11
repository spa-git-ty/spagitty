<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-067 — Automated test record

**Item:** [BUG-067](../items/BUG-067-review-says-the-token-is-invalid.md)

| Suite | Cases |
| --- | --- |
| `src/lib/requests/store.test.ts` | A refused first read is asked again and shows nothing when the second works; a repeated refusal is shown with its account; a failure that is not a refusal is not asked twice; *you* is the account for the repository's host with two accounts connected. |
| `src/lib/review/store.test.ts` | Review primed before Pull requests with two accounts reads as the repository's account and shows no error. |

## Results — 2026-10-11

- `bunx vitest run src/lib/requests src/lib/review src/routes/review src/routes/requests`: 238 passed.
- `bun run check`: zero errors, zero warnings. `cargo clippy -p spagitty --all-targets -D warnings`: clean.

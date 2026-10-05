<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-044 — Automated test record

**Item:** [`agile/items/BUG-044-start-review-does-nothing-on-gitlab.md`](../items/BUG-044-start-review-does-nothing-on-gitlab.md)

## What was tested

In `src/routes/review/page.test.ts`:

- *opens a GitLab merge request from this repository*: a GitLab row naming
  `team/billing` under *This repo* opens the room, keyed on the
  open repository's host, without looking for a clone. It failed before the
  fix (`review.room` stayed null).
- *says why a review did not open*: a clone lookup that throws shows "The
  review could not be opened" with the cause, and Start review is enabled
  again. It failed with the catch removed.

## Test command and output

On Windows 11: `bun run test` — 3162 passed, 1 failed. The failure is
`tools/record.test.ts`, which reports BUG-043's missing plan, automated record
and sweep; it is on `main` and is not this change. `bun run check` — 0 errors,
0 warnings.

## What is not covered automatically

The review room against a real GitLab, which needs the host and a token. See
the sweep.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-032 — Plan

**Item:** [`agile/items/BUG-032-a-path-test-that-only-passes-on-unix.md`](../items/BUG-032-a-path-test-that-only-passes-on-unix.md)

## Approach

Take the path relative to the core with `path.relative`, split it on the
platform separator, and join it with `/`. That is the form the expected value
is written in, and the form `tools/case.test.ts` already normalises to.

## Files

| File | Change |
| --- | --- |
| `src/lib/requests/requests.test.ts` | Normalise the found paths before comparing. |

## Risks and rollback

- None beyond the test. Rollback is a revert.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-069 — Automated test record

**Item:** [BUG-069](../items/BUG-069-added-lines-look-red.md)

| Suite | Cases |
| --- | --- |
| `src/lib/diff/colours.test.ts` | The ruler and comment range are `background-image` layers, not a `background`; an added line takes `--diff-add-bg` (green) and a removed one `--diff-del-bg` (red); the comment range and the `+` button are not accent-coloured; no syntax colour is lane 1, the danger red or its hex; the added tint is at least 14%. |

## Results — 2026-10-11

- `bunx vitest run src/lib/diff/colours.test.ts`: 5 passed.

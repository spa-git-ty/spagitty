<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-049 — Automated test record

**Item:** [`agile/items/BUG-049-small-monospace-text-ignores-the-reading-font.md`](../items/BUG-049-small-monospace-text-ignores-the-reading-font.md)

## What was tested

`src/lib/reading.test.ts`, *the monospace outside the code*:

- `--font-mono` is published as the default code face, then as JetBrains Mono
  and as the system stack when those are chosen;
- with Lexend chosen, `--code-font` is Lexend and `--font-mono` is the default
  code face, and no choice gives `--font-mono` a proportional face;
- the stylesheet's `--font-mono` is the same stack as the one published for the
  defaults, so the first frame does not change face.

## Test command and output

On Windows 11: `bunx vitest run` — 3 256 passed, 1 failed. The failure is
`tools/record.test.ts`: BUG-043 is Fixed and lacks its plan and testing
documents. It fails the same way before this change and is not part of it.

`bun run check` — 0 errors, 0 warnings.

## What is not covered automatically

The faces as drawn, and the reflog row's width. See the sweep.

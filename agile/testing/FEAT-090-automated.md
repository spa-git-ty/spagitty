<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-090 — Automated test record

**Item:** [`agile/items/FEAT-090-settings-reading.md`](../items/FEAT-090-settings-reading.md)

Written after the branch was built, on the tip of the stack that contains it
(`feature/FEAT-093-threads-done-properly`); the suites below are this item's,
and the counts are the whole run at that tip.

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/diff/words.test.ts` | Words, spaces and punctuation split with nothing lost; only the words that changed marked; two changed words joined across the space between them; nothing marked when a line was rewritten rather than edited; a pair too long to compare cheaply left unmarked; each removed line paired with the added line in the same place, and with the new line it most resembles when that is another. |
| `src/lib/reading.test.ts` | Nothing read as the defaults; each good field kept and each bad one replaced on its own; the defaults published as tokens on the first read; every token changed at once and the choice kept; code scaled with the zoom; the interface given back to the desktop when System is chosen again; Settings › Reading offering every face, ruler and colour, each chip in its own face; its chips and sliders changing the preferences; its preview marking the changed words. |
| `src/lib/ui/flat.test.ts` | The section and the panes read only defined tokens, with no literal colours or pixel font sizes. |

## Test command and output

On Windows 11, at the stack's tip:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3153 passed (3154)
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut.

No Rust changed.

## What is not covered automatically

How each face renders on Omarchy's WebKitGTK, offline: the sweep.

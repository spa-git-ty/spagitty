<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-040 — Automated test record

**Item:** [`agile/items/BUG-040-the-shells-pane-class-leaks-into-five-components.md`](../items/BUG-040-the-shells-pane-class-leaks-into-five-components.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/ui/flat.test.ts` | `app.css` has no global `.pane` rule; `window-pane` is used by `+layout.svelte` alone. FEAT-082's test — ornaments have glass, the pane never blurs, the layout's `<main>` is the pane — reads the new name. |

The new test fails against the stylesheet before the change.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  3001 passed (3001)
```

## What is not covered automatically

What the five screens look like. Checked in the Windows release build.
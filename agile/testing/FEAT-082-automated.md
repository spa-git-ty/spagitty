<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-082 — Automated test record

**Item:** [`agile/items/FEAT-082-a-spatial-shell.md`](../items/FEAT-082-a-spatial-shell.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/chrome/chrome.test.ts` | The title bar carries the tabs as pills and drops the name while they are open; the name is still centred when there are none. The toolbar is an ornament with `role="toolbar"`, no repository name and no Settings gear. The strip keeps state on the toolbar's left and counts and the licence on its right, the licence last. |
| `src/lib/ui/flat.test.ts` | The ornaments take `--blur-thick` through `.ornament`; the pane and the environment never blur; the rail and the toolbar use the class; the layout's pane is `<main class="pane">`. `--r-floating` is 16px. |
| `src/lib/metrics.test.ts` | `RADII` and `app.css` agree, now including `r-pane` and `r-ornament`. |

### Tests changed, and why

Six chrome assertions described the bars this item replaces: the tabs absent
from the title bar (FEAT-044), the title bar's leading column empty, the
repository name on the toolbar, the counts inside `.repo`, a hairline divider
between the strip's groups, and the licence as the strip's last child. Each now
asserts the shell as designed here; what they protected — the name centred, the
state and inventory apart, the licence never pushed off — is still asserted.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1164 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Test Files  137 passed (137)
     Tests  2943 passed (2943)
```

## What is not covered automatically

What it looks like. Screenshots in both themes, closed and open rail, were
taken in a headless browser during the work; the sweep repeats them in the
application on each platform.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-034 — Automated test record

**Item:** [`agile/items/BUG-034-a-transparent-band-round-the-window-on-windows.md`](../items/BUG-034-a-transparent-band-round-the-window-on-windows.md)

## What was tested

`src/lib/chrome/window.test.ts`: Windows is `flush` restored and maximized, as
Linux and macOS are; only a host whose agent names none of the three gets the
drawn card, and a maximized one squares itself; the platform is still read from
whole words. `tools/window.test.ts` still holds: the Linux override differs from
the manifest in transparency alone.

## Test command and output

```
$ bunx vitest run tools/window.test.ts src/lib/chrome
Tests  90 passed (90)

$ bun run test
Tests  2945 passed (2945)
```

## What is not covered automatically

What Windows draws. The sweep checks it on Windows 11 and Windows 10.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-035 — Automated test record

**Item:** [`agile/items/BUG-035-native-scrollbars-and-a-smudged-tab.md`](../items/BUG-035-native-scrollbars-and-a-smudged-tab.md)

## What was tested

`src/lib/ui/flat.test.ts`: the standard scrollbar properties sit inside the
`@supports not selector(::-webkit-scrollbar)` guard, scrollbar buttons are not
displayed, and the open tab declares no `box-shadow`.

## Test command and output

```
$ bunx vitest run src/lib/ui/flat.test.ts src/lib/chrome
Tests  112 passed (112)
```

## What is not covered automatically

What Chromium draws. Checked in the Windows release build: thin rounded thumbs,
no arrows, no filled corner; the open tab without a shadow.

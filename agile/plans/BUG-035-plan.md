<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-035 — Plan

**Item:** [`agile/items/BUG-035-native-scrollbars-and-a-smudged-tab.md`](../items/BUG-035-native-scrollbars-and-a-smudged-tab.md)

## Approach

Wrap the `* { scrollbar-width; scrollbar-color }` rule in
`@supports not selector(::-webkit-scrollbar)`, so an engine with the
pseudo-elements uses them and one without keeps a thin themed bar. Add
`::-webkit-scrollbar-button { display: none }`. The two components that hide a
scrollbar with `scrollbar-width: none` keep doing so; Chromium honours it.

The tab drops `box-shadow` and takes `--surface` for its fill.

## Files

| File | Change |
| --- | --- |
| `src/app.css` | The guard; no scrollbar buttons. |
| `src/lib/chrome/RepoTabs.svelte` | No shadow on the open tab. |
| `src/lib/ui/flat.test.ts` | Both, asserted. |

## Risks and rollback

- Firefox, were it ever the engine, keeps the standard properties. Rollback is
  a revert.

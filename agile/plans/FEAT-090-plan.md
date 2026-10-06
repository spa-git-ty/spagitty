<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-090 — Plan

**Item:** [`agile/items/FEAT-090-settings-reading.md`](../items/FEAT-090-settings-reading.md)

## Approach

A store beside `scale.svelte.ts`, `reading.svelte.ts`, kept in `localStorage`
for the same first-frame reason, normalised field by field, publishing tokens
on `<html>`: the font stacks and spacing as custom properties, the diff
palette as `data-diff` and `data-words` that `app.css` keys off, and the code
size through a new `scale.setCodeSize`, so it still multiplies with zoom and
text size. `app.css` declares the faces and the calm defaults; classic and
words-off are attribute overrides.

The panes change only what they read: `--code-font` and the line and letter
spacing where they read `--font-mono`, the diff tokens where they had 14%
tints, and a `.word` span around the changed words that `diff/words.ts` finds.

`words.ts` is pure: words, spaces and single punctuation as tokens; a
bounded LCS per pair; pairing within each removed-then-added run by
likeness, penalising scattered marks; settling marks to bridge space and lone
punctuation between changes and keep space out of a mark's ends.

## Files

| File | Change |
| --- | --- |
| `assets/fonts/**` | The five faces and their licences. |
| `src/app.css` | `@font-face`; reading and diff tokens. |
| `src/lib/reading.svelte.ts`, `scale.svelte.ts` | The store; `setCodeSize`. |
| `src/lib/diff/words.ts` | Changed words. |
| `src/lib/settings/ReadingSection.svelte`, `store.svelte.ts`, `routes/settings/+page.svelte` | The section. |
| `src/lib/diff/DiffPane.svelte`, `changes/HunkPane.svelte`, `history/FileHistoryView.svelte` | Read the tokens. |
| `src/routes/+layout.svelte` | `reading.init()` after the scale. |
| `NOTICE` | The fonts. |

## Risks and rollback

- **Diffs look different on upgrade**: the reading set and calm colours are
  the defaults. System monospace and Classic restore the old look in two
  clicks.
- **A Lexend code font is proportional**: columns do not line up, which its
  note says.
- Rollback is a revert.

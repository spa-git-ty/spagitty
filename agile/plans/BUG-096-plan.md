<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-096 — Plan

**Item:** [BUG-096](../items/BUG-096-the-working-copy-s-hunks-are-not-coloured.md)

1. `HunkPane.svelte`: a `syntax` derived from `tokenizeDiff(hunk.lines, detectLanguage(file.path))` per hunk.
2. Each line's runs are `paint(syntax, marks)`, drawn as `tok-*` spans with `word` on the changed ones.

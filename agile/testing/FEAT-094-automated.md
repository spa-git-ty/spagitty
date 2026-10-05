<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-094 — Automated test record

**Item:** [`agile/items/FEAT-094-review-reads-like-code.md`](../items/FEAT-094-review-reads-like-code.md)

## What was tested

- `src/lib/diff/highlight.test.ts`: languages by file name; Kotlin, TOML keys
  and sections, JSON keys, Rust lifetimes/characters/macros/attributes, CSS;
  a block comment and a string carried over lines; a Svelte file's markup,
  expressions, comment, script and style; a tag whose attributes wrap;
  Markdown headings, quotes and fenced code; the two sides of a diff read
  apart; `paint` cutting tokens at changed words. The FEAT-064 tests stand,
  one changed: a `.svelte` file is now `svelte`.
- `src/lib/ui/markdown.test.ts`: a dependabot table; `<details>` and
  `<summary>`; nested and task lists and line breaks; fenced code coloured;
  links opening apart; `javascript:` (plain and in HTML), `<script>`,
  `onerror`, `onclick`, `style`, foreign `class` and `id`, `<iframe>`,
  `<form>` and relative links all gone; an image made a link; `safeUrl`
  refusing disguised schemes (`java\tscript:`, upper case, leading space,
  `data:`, `vbscript:`, `file:`); `markdownText`.
- `src/routes/review/room.test.ts`: an added Rust line's functions coloured
  with its changed words still marked; the files and Conversation card resized
  from the keyboard; a comment's fenced code drawn as code.
- `src/routes/review/page.test.ts`: the preview draws a description's table,
  and widens from its edge.
- `src/lib/panels.test.ts`: the reset record holds the three new panes.

## Test command and output

On Windows 11: `bun run test` — 3225 passed, 1 failed: `tools/record.test.ts`
on BUG-043's missing documents, which is on `main` and not this change.
`bun run check` — 0 errors, 0 warnings.

Seen in headless Chrome on a preview route (not committed): the room's Rust
file coloured by kind, changed words over the colours; the inbox preview with
a dependabot table, cells kept whole and the table scrolling sideways, and
release notes in a `<details>` that opens.

## What is not covered automatically

The release build on a real pull request, and what the webview does with a
link that opens a window. See the sweep.

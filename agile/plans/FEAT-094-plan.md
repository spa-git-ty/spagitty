<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-094 — Plan

**Item:** [`agile/items/FEAT-094-review-reads-like-code.md`](../items/FEAT-094-review-reads-like-code.md)

## Approach

1. **The highlighter** (`src/lib/diff/highlight.ts`). A `LineState` — what is
   open (comment, string, tag) and, in markup, the embedded block and its own
   state — passes from line to line. `tokenize(line, language)` keeps its
   meaning (a line read as a file's first), so the Diff and File history
   screens are unchanged in use; a `.svelte` line read alone is read as
   script, as before. `tokenizeLines` reads a file; `tokenizeDiff` reads the
   old side (unchanged and removed lines) and the new side (unchanged and added
   lines) apart. `paint(tokens, words)` cuts both at every boundary of either,
   so a run carries its colour and its change. Languages are a table of
   keywords and a `Syntax` (line and block comments, strings that may run on,
   quotes, annotations, keys, sections); CSS, markup and Markdown have their
   own readers.
2. **The room.** `Content.syntax` is computed with the words when a file is
   read; `RoomDiff` draws `paint(syntax, words)`.
3. **Colours.** `.tok-*` in `app.css`: keyword lane 4, string lane 2, number
   lane 3, function lane 5, type lane 1, operator and punctuation muted,
   comment muted italic, tag lane 1, attribute lane 3, meta lane 4 italic.
4. **Panes.** Three `PANELS` entries (`reviewPreview`, `roomFiles`,
   `roomConversation`) with defaults in `metrics.ts`, and a `Splitter` at each
   edge. The preview's description is `flex: 0 1 auto` with a floor, not 40%.
5. **Markdown.** `renderMarkdown` (`src/lib/ui/markdown.ts`): `marked` with
   `gfm` and `breaks`, a code renderer that colours fences with
   `tokenizeLines`, then the allow-list rebuild. `Markdown.svelte` draws it;
   `PRMarkdown` keeps its frame and empty state around it; the room's thread
   and draft bodies use it compact; `markdownText` gives the Conversation card
   plain words.

## Files

| File | Change |
| --- | --- |
| `src/lib/diff/highlight.ts` | Line state, languages, `tokenizeLines`, `tokenizeDiff`, `paint`. |
| `src/app.css` | One colour per token kind; `tag`, `attr`, `meta`. |
| `src/lib/review/room.svelte.ts`, `RoomDiff.svelte` | Colours per line; comments as Markdown. |
| `src/lib/metrics.ts`, `src/lib/panels.svelte.ts` | Three resizable panes. |
| `src/lib/review/ReviewInbox.svelte`, `InboxPreview.svelte`, `ReviewRoom.svelte`, `RoomFiles.svelte`, `RoomConversation.svelte` | Splitters, widths from variables, Markdown. |
| `src/lib/ui/markdown.ts`, `src/lib/ui/Markdown.svelte` | The renderer and its look. |
| `src/lib/requests/PRMarkdown.svelte` | Uses them. |
| `package.json`, `bun.lock` | `marked`. |

## Risks and rollback

- Colours on the Diff and File history screens change with the palette
  mapping, and `.svelte` files colour as before there (script).
- A link in a description opens with `target="_blank"`; whether the webview
  hands it to the browser is the webview's behaviour, as it was before.
- Images are links, not pictures, while the content policy loads nothing from
  the network.
- Rollback is a revert; `marked` goes with it.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-151 — Plan

**Item:** [FEAT-151](../items/FEAT-151-search-the-text-of-files.md)

1. `files::search` in `spagitty-core`: walk the revision's tree as *Find a file* does, read each blob, leave out binary, LFS and over 2 MB, and match each line with one `regex::Regex` built by `files::pattern` from the query and `SearchOptions` (escaped unless regex, `\b(?:…)\b` for a whole word, case-insensitive unless *Match case* or a capital outside an escape). Each hit carries its line, its column and length in UTF-16 units, and a long line cut around the match. A bad pattern is `Error::Pattern`.
2. `files_search` in `commands.rs`, a reader in TASK-052's list, at most 2,000 lines.
3. `keys.ts`: `patternOf` and `caseCounts` mirror the core's rules in the webview; `find` takes options, its default the old smart-case text search; `optionKey` reads Alt+R, W, C.
4. The store: one options object, `searchFiles` with a ticket, `openHit` setting the match and the query, a search re-run on an option and on a new revision.
5. `SearchToggles.svelte` beside both fields; the tree's *Names / Text* switch and its results, grouped by file; the status line says when a regex does not parse.

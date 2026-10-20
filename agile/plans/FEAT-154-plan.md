<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-154 — Plan

**Item:** [FEAT-154](../items/FEAT-154-usages-of-a-name.md)

1. `files.rs`: the blob walk and the text read move out of `search` into `blobs_at` and `searchable_text`; `hit_on` takes byte offsets. `search` returns what it did.
2. `usages.rs`: whole words by byte; each file holding one is parsed with its grammar and its tags query run (TypeScript and TSX read JavaScript's query with their own). A name the query captures is a definition or a use of its kind; otherwise a place inside a comment or string node, stopping at a substitution or interpolation, is a mention. Prose and data languages only mention. Sorted by role, stably. `tree-sitter` is named as a direct dependency: it was already in the tree, at the same version, through `tree-sitter-highlight`.
3. A `files_usages` command, a reader, 2,000 places at most.
4. Frontend: `filesUsages`; the store's `lookFor` takes `usages`, with `findUsages` (run again on a revision change) and `openUsage`; `UsageList.svelte` in the tree; `FileView` turns a Ctrl/⌘-click into a column with `caretPositionFromPoint` (or `caretRangeFromPoint`) and reads the name around it from the line.

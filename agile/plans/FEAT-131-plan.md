<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-131 — Plan

**Item:** [FEAT-131](../items/FEAT-131-files-any-revision-read-never-written.md)

Written on 2026-10-13, before the code.

## The core

Two new modules in `crates/spagitty-core`, and one extended.

1. **`files.rs`** reads git objects at a revision and nothing else.
   - `resolve(repo, rev)` peels a branch, tag or commit to a commit id.
   - `tree(repo, rev, dir)` lists one directory: folders first, then files, each with its kind (folder, file, link, submodule).
   - `read(repo, rev, path)` returns the blob id, the size, the language, and one of: text, an image (as the Diff screen's base64), binary, a Git LFS pointer (object id and size, parsed from the pointer text), or too large.
   - `find(repo, rev, query, limit)` walks the tree for the *Find a file* field, capped.
   - `before(repo, commit, path, line)` gives the line a commit's hunk replaced, for the peek. It reuses `diff::file_diff`: the line's place among the added lines of its change is the removed line's place among the removed ones.
   - The limits are named constants: `COLOUR_MAX_BYTES` (5 MB) and `OPEN_MAX_BYTES` (50 MB).
   - The module holds no path into the working tree. A test reads its source for `std::fs` and `workdir`, and another reads a file whose working copy was changed and then deleted.
2. **`highlight.rs`** is `tree-sitter-highlight` with the grammars compiled in.
   - `language_of(path)` names the language, and `has_grammar(language)` says whether tree-sitter has it.
   - `spans(repo, blob, language, start, end)` highlights the whole blob once and keeps the result per line in an LRU cache keyed by blob id and language. A blob never changes, so nothing is ever invalidated. A request is a slice of the cached lines.
   - A line's colour goes over the wire as pairs of token class and length in UTF-16 units, so the frontend cuts its own text and no text is sent twice.
   - Tree-sitter's capture names are mapped to the twelve token classes Diff already has (`tok-keyword` and the rest), so Files and Diff read the same.
   - A grammar that will not build against the current tree-sitter is left out, and its language falls back to `lib/diff/highlight.ts`. The item says the list is measured before it is final; the plan is to record which grammars are in and what each costs.
3. **`blame.rs`** gains `compact(repo, path, rev)`: the commits once, and for each line an index into them. `--line-porcelain` repeats every header on every line, and a 20,000-line file should not cross the wire that way. Each commit carries the author's address (for the portrait), the commit it replaced the line from (`previous`), and the agent named by a `Co-authored-by` trailer.
4. **`attribution.rs`** is the rule of `tools/attribution.mjs` (`isAgent`) in Rust, with the same table of cases, so "agent-written" means in Files what it means at gate 1.

## The screen

`/files`, a rail row after Conflicts until Terminal exists (then after Terminal), code 1U.

- **`src/lib/files/store.svelte.ts`** holds the revision, the open tabs, the tree's open folders and the view switches, kept per repository in `localStorage`.
- **The tree** is lazy: a folder is read when it is opened.
- **The file** is `VirtualRows`, which already measures rows of different heights. The peek is a row of its own under its line, so it is inline and never a popover.
- **Paint order**: the text is drawn as soon as it is read. Colour is asked for the rows in view and arrives after. Blame is asked for after the first paint and fills the gutter when it arrives.
- **Keys** are a small pure machine in `keys.ts` (`gg`, `]b`, counts are not taken), so they are tested without a component. No key reaches an input, because there is none in the file view.
- **The revision picker** is `BranchMenu` with its tags, and *A commit…*, which takes a revision typed into the menu's own field when it resolves.
- **Opened from another screen**: `/files?rev=…&path=…&line=…`.

## Dependencies

`tree-sitter`, `tree-sitter-highlight` and one crate per grammar, all MIT or Apache-2.0. `cargo deny` decides, and each is listed in the handoff with the reason.

## What decides it is right

- A file reads the same after its working copy is changed or deleted, and in a bare repository.
- A 20,000-line file paints its text before colour and blame, and scrolls without drawing rows that are not in view.
- An LFS pointer, a binary file, an image and a file over each limit each say what they are.
- A line written by a commit with an agent's trailer is drawn in the agent's colour, and a person who shares a name with an agent is not.
- The peek's "was" line is the line the commit removed in that place, and absent when the commit only added.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-131 — Automated test record

**Item:** [FEAT-131](../items/FEAT-131-files-any-revision-read-never-written.md)

## Rust

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/files.rs` | **The module has no way to the working tree**: its source is read and holds no `std::fs`, `workdir`, `shell::` or `Command`. A file is read from the commit after its working copy is changed, after it is deleted, and a file only on disk is not in the tree. A bare repository reads the same. A folder lists folders first, then files by name without regard to case. A folder that is not there is an error, and a file is not a folder. A branch, a commit and an annotated tag each read the file as it was then. A text file names its blob, size and grammar; a language with no grammar names none. A folder or nothing at the path is *missing*. A binary file, an image and an SVG (read as text) each say what they are. A Git LFS pointer gives its object id and size, and text that only looks like one is text. The limits are 5 MB and 50 MB. A file over 5 MB is read without colour. *Find a file* ranks a name that starts with the word first, wants every word, and says when more matched than it returned. `before` gives the line a commit replaced, nothing for a line that was only added or not touched, and follows the file to the name it had. |
| `crates/spagitty-core/src/highlight.rs` | Every grammar listed builds with its query. A language with no grammar says so. A path finds its grammar, and every grammar a path can name builds. Rust is coloured by what each word is (keyword, type, comment, function, macro, string, number). Every line is covered exactly, with two-unit characters, a CRLF and no final newline. A trailing newline ends the last line and starts none. A markup file's script is coloured as its own language, and its attributes as attributes. TOML and JSON colour their keys and values. **The cache**: a blob is coloured once and served in slices, past the end is the end, and the newest are kept while the oldest go. A blob that does not exist is refused. |
| `crates/spagitty-core/src/blame.rs` | The compact blame agrees line for line with the one that repeats itself. Each commit is said once, with the author's address. A commit that changed the file names the commit and path it changed it from; the commit that added it names none. A line from a commit with an agent's `Co-authored-by` trailer is marked, and a person called Claude is not. A renamed file's lines keep the name they were written under. The same refusals as the long form. An older revision is blamed as it was. A bare repository is blamed from its objects. A path with spaces is kept whole. |
| `crates/spagitty-core/src/attribution.rs` | A trailer naming a product is an agent (23 forms). A person who shares a name with an agent is a person (BUG-078's cases). An address only an agent writes from is enough, and a lookalike domain is not. The agent is read from the trailer, in any case, and named as it names itself. A commit that only mentions an agent is not credited to it. The first agent named wins. **The names are the ones gate 1 checks**: `tools/attribution.mjs` is read and compared with the tables. |

## Frontend

| Suite | Cases |
| --- | --- |
| `src/lib/files/colour.test.ts` | The token classes are the ones the Rust highlighter numbers, in the same order (the Rust file is read), and each is a class the stylesheet colours. A line is cut by its runs, in UTF-16 units; it is plain until its colour arrives; whatever the runs leave is drawn plain. A search match cuts the tokens and keeps every colour. Lines are counted as the highlighter and `git blame` count them. |
| `src/lib/files/keys.test.ts` | The table of keys the item gives, one row each. The first key of a pair waits. A pair the second key does not finish is dropped. **No key types into the file**: twenty-two keys that enter a typing mode elsewhere do nothing. The application's own shortcuts are left alone. `find` goes forward and backward, wraps, ignores case until the word has a capital, and finds a second match on the cursor's line. `blockFrom` steps by runs, not commits, and stays put at either end. |
| `src/lib/files/age.test.ts` | An age is a number and a unit. The oldest commit's bar is the faintest and the newest the strongest, and none is drawn at nothing. |
| `src/lib/files/store.test.ts` | Entering reads the tree at the checked-out branch; the same repository again only looks for a moved branch; a moved branch reads the tree and the file again. A folder is read once, the first time it is opened. A file's folders are opened however it was opened. A file opens in a tab, goes back to the tab it has, opens at a line and at another screen's revision, and remembers where each tab was read to. An answer for a file no longer asked for is dropped. **The order**: the text is there before the blame is asked for; colour is asked for a page at a time and only for rows about to be drawn; a failed asking is tried again; nothing is asked of tree-sitter for a file too large; a language with no grammar is coloured by the other highlighter. Blame is not read while the gutter is off. The peek reads what the line was from the commit it was changed from, and asks for the blame when the gutter is off. *Blame before this* reads at the commit before, under the name the file had. Moving, a shift range, `path:line@sha`, searching both ways, and Escape's order. What is remembered per repository, and a remembered branch that has gone. |
| `src/lib/files/files.test.ts` | Mounted. **A file**: every line with its number; the status line; no claim of tree-sitter without a grammar; nothing to type into and nothing that edits; tabs, and the revision a tab was opened at; breadcrumb segments; the empty state. **Not text**: binary, LFS pointer, too large, missing, and an image drawn alone by the image viewer. **Blame**: one head per block; a bar on every line; the agent's name and hexagon against a person's portrait; a block lights its commit's lines; `b`; the lines are drawn before the blame arrives. **The peek**: under its line and in the flow, never a popover; who, when, pull request, why and what the line was; *Commit*, *Blame before this* (and why not, for the commit that added the file), *Investigate line*; closing. **The mouse**: a line number, and a range with shift. **The keys**: `j` `k` `G` `gg`, Ctrl+d and Ctrl+u, `]b` `[b`, `/` `?` `n` `N`, `y`. **The tree**: lazy folders, opening a file, a submodule not followed, *Find a file*. |
| `src/lib/branches/BranchMenu.test.ts` | *A commit…* is not offered unless asked for; it sits under the names and apart from them; it hands over what was typed; with nothing typed it asks; Enter takes a commit only when no name matches. |
| `src/lib/ui/VirtualRows.test.ts` | The list says which rows are drawn, and says nothing new for a scroll inside them. |
| `src/lib/nav.test.ts`, `src/lib/chrome/chrome.test.ts`, `src/lib/palette/commands.test.ts` | Files is on the rail after Working copy and Conflicts, and the palette reaches it. |

## Not covered

- **The Rust and the screen together.** Each side is tested against the other's shape, never against the other. Nothing was run in the built application.
- **Sixty frames a second.** Measured in headless Chrome, not asserted; see the sweep.
- **The revision picker and the URL parameters on the page.** The store's `setRevision` and `openFile` are tested; `routes/files/+page.svelte` is not mounted.
- **Dragging to select text** in a line, which must not open the peek. The check is one line and was not driven.

## Results — 2026-10-13

- `bun run test`: 196 files, 4324 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-core` (Windows): 735 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `bun run build`: succeeded.

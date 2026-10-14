<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-131 — Files: any revision, read and never written

**Status:** Done — on `release/3.0.0`, after FEAT-134; unmerged. Covered by Rust tests against real repositories and by mounted tests, and looked at in the built frontend in headless Chrome with a stand-in for the backend. The Rust and the screen have not been run together: not looked at in the running application.
**Branch:** `feature/FEAT-131-files-any-revision-read-never-written`.
**Screens:** 1U (Files, new, at `/files`), chrome (a rail row).
**Raised by:** the author, 2026-10-12: a file viewer like Neovim with AstroNvim — mouse-clickable, tree-sitter coloured — where the person can browse any branch and blame any line. No edits: Spagitty is not an editor.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §6. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `Viewer.dc.html`.

## Change

Files shows the repository's tree and any file in it at any branch, tag or commit. It has:

- tree-sitter colour, computed in Rust for the rows in view;
- FEAT-063's blame beside every line;
- a peek under a clicked line that says who, when, why and what the line was before.

It reads git objects only.

## Acceptance criteria

- **A rail row** in Repository, after Terminal, with code 1U.
- **Reading**:
  - `files::tree(repo, rev, dir)` and `files::read(repo, rev, path)` read from gix objects. No code path from this screen opens a file in the working tree, and a test asserts it.
  - The revision picker is the branch menu (FEAT-122) with Tags and *A commit…*.
- **Colour**:
  - `highlight::spans(blob_id, language, range)` uses `tree-sitter-highlight` with the grammar set in the spec (§6.2), cached by blob id.
  - Unsupported languages fall back to `lib/diff/highlight.ts`.
  - Token colours come from the theme's syntax tokens, the same ones Diff uses.
- **Size limits**: over 5 MB, no colour. Over 50 MB, the file is not opened and the size is said. Binary files show *binary* and their size, images use the image viewer, and an LFS pointer shows as one.
- **Blame**:
  - The blame comes from FEAT-063 at the revision and is computed after first paint.
  - Each block gets one header row; every row has an age bar.
  - Agent-written commits (a `Co-authored-by` trailer naming an agent) use `--agent` and the hexagon.
- **Clicking**:
  - A blame block lights the lines from its commit.
  - A line opens the peek inline under it, with: author, age, pull request (when the forge knows it), summary, sha, the line as it was before (struck through), and *Commit*, *Blame before this*, *Investigate line*.
- **Mouse and keys**:
  - Everything is mouse-clickable.
  - Keys: `j` `k`, `Ctrl+d` `Ctrl+u`, `gg` `G`, `/` `?` `n` `N`, `]b` `[b`, `b`, `Enter`, `Esc`, and `y` (copies `path:line@sha`). No key types into the file.
- **Chrome of the screen**: tabs of open files, kept per repository; a breadcrumb whose segments open the tree there; and a status line reading `VIEW · revision · file · language · line:col · %`.
- **Performance**: virtual rows, colour only in view. A 20,000-line file scrolls at 60 fps, and opening it paints text before colour and blame.
- **Tests**: Rust tests for tree and read at a revision, LFS pointers and the highlight cache. Component tests for the screen, the peek and the keys.

## Not in this item

- **Editing, saving, or an *Open in editor* button.**
- **Go-to-definition and any language-server feature.**
- **Using the Rust highlighter in Diff and Review.** That may follow as a task once this is measured.

## What was built

- **`crates/spagitty-core/src/files.rs`** reads trees and blobs at a revision: `resolve`, `tree`, `read`, `find` and `before`. It holds no path to the working tree, and a test reads its source to keep it so.
- **`crates/spagitty-core/src/highlight.rs`** is `tree-sitter-highlight` with 19 languages compiled in. A blob is coloured whole, once, and kept by its id (the newest 24). A request is a slice. Colour crosses to the screen as token class and length for each line, never as text.
- **`blame::compact`** gives each commit once and a commit index for each line, from `git blame --porcelain`. A commit carries the author's address, the commit and path it changed the file from, and the agent its trailer names.
- **`crates/spagitty-core/src/attribution.rs`** is gate 1's rule for an agent's trailer, in Rust. A test reads `tools/attribution.mjs` and compares the names.
- **Seven commands**, `files_*`, none of which holds the session lock while it works.
- **The screen** is `src/routes/files/+page.svelte` with `src/lib/files/`: the store, the tree, the file view and the peek. The file is `VirtualRows`, and the peek is a row of it.
- **`BranchMenu`** takes `oncommit` and then offers *A commit…*. **`VirtualRows`** takes `onrange`. **`ImageDiff`** takes `single`.

## Measured

- **Languages**: 19 of the proposal's 21. Kotlin and Dockerfile fall back to the frontend's highlighter. The Kotlin crate that builds against tree-sitter 0.27 (`tree-sitter-kotlin-ng`) ships no highlight query, and `tree-sitter-dockerfile` is built for tree-sitter 0.20.
- **Size**: the release binary on Windows goes from 12.4 MB (the 2.2.2 build of 2026-10-11) to 31.6 MB (`cargo build --release -p spagitty`, 2026-10-13). Four grammars are most of it. As compiled libraries: C# 5.5 MB, C++ 3.7 MB, TypeScript with TSX 3.4 MB and SQL 3.1 MB, against 7.9 MB for the other fourteen together and 2.5 MB for tree-sitter and its highlighter. The spec's list is kept as it is. Whether that cost is accepted, or those four are left to the frontend's highlighter, is the author's to say.
- **Crates**: `Cargo.lock` goes from 622 packages to 644.
- **Scrolling**: see the sweep, row 18.

## Dependencies added

All MIT. Each is needed for Files' colour.

| Crate | For |
| --- | --- |
| `tree-sitter-highlight` 0.27 (with `tree-sitter`, `tree-sitter-language`) | Reads a grammar's highlight query and gives the spans |
| `tree-sitter-bash`, `-c`, `-c-sharp`, `-cpp`, `-css`, `-go`, `-html`, `-java`, `-javascript`, `-json`, `-md`, `-python`, `-rust`, `-sequel`, `-svelte-ng`, `-toml-ng`, `-typescript`, `-yaml` | One grammar and its query for each language |

`tree-sitter` 0.27 declares a minimum Rust of 1.90 and the workspace declares 1.77. The gates build with `stable`, so nothing fails. TASK-072 records it.

## Differences from the spec and the artboard

- **The pull request number** is read from the commit's summary, as a host writes it (`(#214)`, or `Merge pull request #214`). The forge is not asked: Spagitty has no call that finds a pull request from a commit.
- **Wrap** is built. **Minimap** is drawn on the artboard and is in neither the spec nor this item. It was not built.
- **The breadcrumb** ends at the file. The artboard's last segment is the function the cursor is in, which needs the syntax tree on the screen's side.
- **Tabs can be closed**, with a button on the tab or the middle mouse button. The artboard draws no way to close one.
- ***Find a file*** is on the artboard and in neither the spec nor this item. It is built.
- **The column** in the status line is 1 except where a search has landed. No key moves along a line.
- **Blame** is refused over 4 MB, the limit FEAT-063 set. Colour stops at 5 MB.
- **An SVG** is read as text, so its lines can be blamed.
- ***Investigate line*** goes to `/investigate`, which FEAT-132 builds.

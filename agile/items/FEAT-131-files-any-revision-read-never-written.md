<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-131 — Files: any revision, read and never written

**Status:** Backlog — recorded 2026-10-12 for 3.0.0; not started.
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

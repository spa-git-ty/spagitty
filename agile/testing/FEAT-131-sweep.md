<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-131 — Sweep

**Item:** [FEAT-131](../items/FEAT-131-files-any-revision-read-never-written.md)

The rows marked *headless* were run on 2026-10-13 against the built frontend in headless Chrome at 1440×900, dark, with a stand-in for the backend that answered with a tree, five files, blame and colour. That shows the screen. It does not show tree-sitter, `git blame` or gix, which were run by the Rust tests only. None of these rows has been run in the running application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT131-01 | A repository open | Press **Files** on the rail | The tree of the checked-out branch, *Choose a file from the tree*, and `VIEW` with the branch in the status line | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT131-02 | As above | Open folders, then a source file | Folders open one level at a time. The file's text is drawn at once, then its colour, then the blame gutter | P1 | Passed headless: text at 0.2 s, colour and blame after, in that order with the stand-in's delays. Not run in the application. |
| SWEEP-FEAT131-03 | A Rust, a TypeScript, a Svelte and a Markdown file | Open each | Each is coloured, the status line names the language and *tree-sitter*, and the colours are the ones the Diff screen uses for the same file | P1 | Not run. Tree-sitter's output was checked by Rust tests only. |
| SWEEP-FEAT131-04 | A Kotlin file and a Dockerfile | Open each | Each is coloured, and the status line names the language without *tree-sitter* | P2 | Passed headless for Kotlin. Not run in the application. |
| SWEEP-FEAT131-05 | A file several people and an agent have written | Look at the gutter | Each block has one head: portrait or hexagon, name, summary, age. Every line has a bar, stronger where the line is newer. The agent's lines are in the agent's colour | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT131-06 | As above | Press a blame block; press it again | Every line from that commit is lit, then put out | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT131-07 | As above | Press a line of code | The peek opens under the line, pushing the lines below it down: who, when, the pull request, the summary, the sha, and what the line was before, struck through | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT131-08 | The peek open | Press **Commit**; come back and press **Blame before this**; come back and press **Investigate line** | The Graph with that commit selected; the file at the commit before, in a new tab; Investigate for that line | P1 | Not run. *Investigate line* waits on FEAT-132. |
| SWEEP-FEAT131-09 | A file open, the pointer in it | Press `j` `k`, Ctrl+d Ctrl+u, `gg` `G` | The cursor moves and the file follows. The status line's `line:col` and `%` follow | P1 | Passed headless for `gg`, `j`, `G`. Not run in the application. |
| SWEEP-FEAT131-10 | As above | Press `/`, type a word, Enter; `n`, `N`; then `?` | The search field is in the status line. The match is marked, `n` goes on and `N` back, and `?` searches the other way | P1 | Passed headless for `/` and `n`. Not run in the application. |
| SWEEP-FEAT131-11 | As above | Press `]b` `[b`, `b`, Enter, Esc, `y` | Next and previous blame block; blame off and on; the peek open and closed; `path:line@sha` on the clipboard | P1 | Passed headless for `]b` and `b`. The clipboard was not read. |
| SWEEP-FEAT131-12 | As above | Press `i`, `a`, `o`, `x`, `dd`, `:` | Nothing happens. The file cannot be typed into | P1 | Not run. Covered by tests. |
| SWEEP-FEAT131-13 | A repository with branches and tags | Press the revision in the header; pick a branch, then a tag; then type a commit's sha and press **A commit…** | The tree and the open file are read at each. `@` shows the commit. A word that is no revision says so and nothing changes | P1 | Passed headless for a typed commit. Not run in the application. |
| SWEEP-FEAT131-14 | Two files open, one opened at another revision | Look at the tabs; press each | The tab opened elsewhere names its revision. Pressing it takes the tree to that revision | P2 | Not run. Covered by tests. |
| SWEEP-FEAT131-15 | A file open deep in the tree | Press a segment of the breadcrumb | The tree opens at that folder and shows it | P2 | Not run. |
| SWEEP-FEAT131-16 | As 01 | Type in *Find a file* | Every path at the revision that holds the words, best first. Pressing one opens it. Emptying the field brings the tree back | P2 | Passed headless. Not run in the application. |
| SWEEP-FEAT131-17 | A PNG, a binary file, a Git LFS pointer, a file over 50 MB, a file over 5 MB | Open each | The picture; *binary* and its size; the pointer with its object and size; the size and that it is too large to open; the text without colour | P1 | Passed headless for the picture, binary and the pointer. Not run in the application. |
| SWEEP-FEAT131-18 | A file of 20,000 lines | Open it and scroll with the wheel, then drag the scrollbar | The text is there before colour and blame. Scrolling is smooth | P1 | Headless Chrome: 60 frames a second at 60 and 150 pixels a frame, 53 at 400, 37 at 900 (a flick). Not measured in the application's webview. |
| SWEEP-FEAT131-19 | A file open | Edit it in an editor and save, without committing; then delete it on disk | Files shows the committed text both times | P1 | Not run. Covered by Rust tests. |
| SWEEP-FEAT131-20 | Files open on a branch | Commit to that branch from Working copy and come back | The tree and the open file are of the new commit | P2 | Not run. |
| SWEEP-FEAT131-21 | Tabs open | Close Spagitty and open it again | The revision, the tabs and the open folders are as they were left | P2 | Not run. Covered by tests against `localStorage`. |
| SWEEP-FEAT131-22 | Light theme | Open a file with blame and a peek | It reads as clearly as in dark | P2 | Not run. |
| SWEEP-FEAT131-23 | A bare repository | Open it and open a file | The tree, the text and the blame | P2 | Not run. Covered by Rust tests. |

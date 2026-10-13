<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-132 — Automated test record

**Item:** [FEAT-132](../items/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line.md)

## Rust

`crates/spagitty-core/src/investigate.rs`, against repositories built with `git`.

| What | Cases |
| --- | --- |
| **Rename and move detection** | A file's life goes back past its renames to the commit that added it. A new name in the same folder is *renamed* and another folder is *moved*, each with the path before. Without following renames the life begins where the name does. |
| **The rewrite threshold** | `REWRITE_SHARE` is 0.5. Ten of twenty lines removed is a rewrite, and nine is an edit. The commit that added the file has no share. |
| **Size** | The lines after each commit are counted from the file. A deleted file ends at nothing and says so. |
| **Survival counting** | Today's blame gives each commit the lines of it that are left: of four an agent's commit wrote, three after one was replaced. Owners are by author, an agent's commits the agent's, most lines first, with shares. A file that cannot be blamed has no owners and is not an error. |
| **Branch birth** | With a reflog: a local branch is born where it left the default branch, with ahead and behind. Without one: the same birth for a branch that is only a remote's. Born where its work began whatever was merged since: the fork stays put when the default branch is merged in and when the branch lands, though the merge base moves both times. A branch with no commit of its own is where it was cut. The default branch is born at its root and measured against nothing. The remote's head names the default branch when it says. |
| **A branch's events** | Its reflog gives its resets and rebases. A merged branch says where it landed, and a merge into it is one. |
| **A folder** | It counts the files added, removed and renamed under it, leaves out a commit that only touched another folder, and is not born until its oldest commit has been read. |
| **A line** | Every commit that changed it, with its text then, followed through the lines above it moving. |
| **Paging and range** | A life is read a page at a time, and a range leaves out what is older. |
| **Agents** | A commit an agent co-authored is the agent's, and a person called Claude is a person. |
| **The hunk** | The largest hunk is the one with the most lines changed. A rename that changed no line has none. *Compare with today* is the file then against the file now under its new name, and nothing for the newest entry. |
| **Parsing** | A log page is read field by field, with a path that has spaces and a binary file's dashes. |
| **Errors** | A revision or a branch that does not exist is an error. |
| **Objects only** | A life does not read the working copy. |

## Frontend

| Suite | Cases |
| --- | --- |
| `src/lib/investigate/chart.test.ts` | **Size**: a file's lines as counted, kept across a commit whose size could not be read; a folder's lines and files summed from the oldest up, never below nothing; a branch's own commits; nothing for a line. **Own commits**: not where a branch landed, its reflog, or its fork. **A folder's list**: structural events, its birth and its twenty biggest edits, newest first. **Facts**: born, commits, authors, renames, size today and peak; no birth until the oldest entry is read; a branch's ahead and behind. **Layout**: time left to right; the area closed along the bottom; the peak off the frame; markers on born, renamed, moved and rewrite, with the rewrite's share; a row for each author in order of first appearance; the agent in `--agent`; authors past six counted; the axis by month, day or year; one commit drawn in the middle; no line for a line; a sentence for a screen reader. |
| `src/lib/investigate/store.test.ts` | The first page is asked for with the target, following renames, over all time, and from another screen's revision. **Streamed**: the first page is on screen, with an event chosen and today's blame asked for once, before the second is back. Page after page until done. **The limit**: reading stops at 5,000 and *Load more* reads on. A page for a target no longer shown is dropped. Follow renames and the range read again. A branch's pages are sorted into one order. A folder lists fewer than it holds. **Kept**: a life costs nothing to come back to, with today's blame; it is read again once HEAD moves; each range and way of following is kept apart. **The selected event**: its hunk, from the name the file had; the lines alive; comparing and going back; no hunk for a deleted file; a late hunk dropped. |
| `src/routes/investigate/page.test.ts` | Mounted, **one life for each target**. *A file*: the address, the facts, the chart's sentence and markers, the authors' rows with the hexagon, the events, the selected event's share, survival and ownership bar, its largest hunk, *Open file at* (to Files, under the name it had), *Compare with today*, *Show in Graph*, the arrows, Follow renames, the range. *A folder*: files and lines, its list, the files an event added, no comparison and no rename switch. *A branch*: the one checked out by default, where it left the other branch and how it stands, its events and markers, only the Graph, the branch menu. *A line*: its file, number and revision; no chart; the line as each commit left it. **Choosing**: from a file to its line and its folder; waiting for a path; a typed path; suggestions, and a folder's. **Nothing to show**: no repository, nothing checked out, an error, an empty range. **`/history`** goes to Investigate with its path, and with none. |
| `src/lib/nav.test.ts`, `src/lib/chrome/chrome.test.ts`, `src/lib/palette/commands.test.ts` | Investigate is on the rail in Tools after Rebase with code 1O, `OFF_RAIL` has no File history, and the palette reaches it. |

## Not covered

- **The Rust and the screen together.** Nothing was run in the built application.
- **A history of thousands of commits.** Paging is tested with five commits and pages of two in Rust, and with generated entries on the screen. How long a page of `git log --follow` takes on a large repository was not measured.
- **`git log -L` on a line that was moved between files**, which the item leaves out.

## Results — 2026-10-13

- `bun run test`: 198 files, 4422 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-core` (Windows): 763 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `bun run build`: succeeded.

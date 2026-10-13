<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-132 — Sweep

**Item:** [FEAT-132](../items/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line.md)

The rows marked *headless* were run on 2026-10-13 against the built frontend in headless Chrome at 1440×900, dark, with a stand-in for the backend that answered with made-up lives. That shows the screen. It does not show `git log --follow`, `git log -L` or the blame, which were run by the Rust tests only. None of these rows has been run in the running application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT132-01 | A repository open | Press **Investigate** on the rail | It is in Tools, after Rebase. With nothing asked for, it shows the life of the branch that is checked out | P1 | The rail row passed headless. The first target is covered by a mounted test. Not run in the application. |
| SWEEP-FEAT132-02 | A file that has been renamed, moved and rewritten | Choose **File** and type its path | The facts say when it was born and by whom, and count its commits, authors and renames, its lines today and at its peak. The chart has a marker on each born, renamed, moved and rewrite. Each author has a row of ticks | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT132-03 | As above | Compare the events with `git log --follow --stat -- <path>` | The same commits, in the same order, with the same lines added and removed | P1 | Not run. Needs the application. |
| SWEEP-FEAT132-04 | As above | Press the rewrite | Its share of the file, how many of its lines are still alive, who owns the file today, and its largest hunk | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT132-05 | As above | Check *still alive* against `git blame` at HEAD | The number is the lines blame gives that commit | P1 | Not run. Covered by a Rust test. |
| SWEEP-FEAT132-06 | An event selected | Press **Open file at**, come back; press **Compare with today**, then **Back to this commit**; press **Show in Graph** | Files at that commit, under the name the file had then; the file then against the file now, in place; the Graph with the commit selected | P1 | Passed headless for the comparison. Not run in the application. |
| SWEEP-FEAT132-07 | A renamed file | Switch **Follow renames** off | The life begins at the rename, which reads as born | P2 | Not run. Covered by a Rust test. |
| SWEEP-FEAT132-08 | Any file | Choose **The last 90 days** | Only events in the range. *Born before this range* when its birth is older | P2 | Passed headless. Not run in the application. |
| SWEEP-FEAT132-09 | A folder with a long history | Choose **Folder** and type its path | Its files and lines today, where it was born, and a list of the commits that added, removed or renamed a file, with its biggest edits | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT132-10 | A local branch cut from main, with main merged into it once | Choose **Branch** and pick it | Born where its work began, not at the merge. Its commits, the merge marked as one, and ahead and behind of main | P1 | Not run. Covered by Rust tests. |
| SWEEP-FEAT132-11 | A branch that was rebased and reset, then merged | As above | The rebase and the reset from its reflog, and *merged · into main* where it landed | P1 | Passed headless with made-up entries. Not run in the application. |
| SWEEP-FEAT132-12 | A branch that exists only on a remote | As above | The same life, with no reflog events | P2 | Not run. Covered by a Rust test. |
| SWEEP-FEAT132-13 | Files open on a line | Press the line, then **Investigate line** | Investigate opens on that line at the revision Files was showing: every commit that changed it, with the line as each left it | P1 | Passed headless from the address. The button in Files was not pressed. |
| SWEEP-FEAT132-14 | Any target | Type part of a path in the field | Paths at the revision that hold it. The arrows and Enter take one | P2 | Passed headless for the list. Not run in the application. |
| SWEEP-FEAT132-15 | A file with more than 200 commits | Open its life | The newest 200 are on screen at once, and the count rises as the rest arrive | P1 | Passed headless with 650 made-up entries: 200, 400, 600, 650. Not run in the application. |
| SWEEP-FEAT132-16 | A file with more than 5,000 commits | Open its life | Reading stops at 5,000 and **Load more** reads on | P2 | Not run. Covered by a store test. |
| SWEEP-FEAT132-17 | A life open | Go to another screen and come back; then commit and come back | The first time it is there at once. After the commit it is read again | P2 | Not run. Covered by store tests. |
| SWEEP-FEAT132-18 | An old link | Go to `/history?path=<a file>` | Investigate, on that file | P2 | Passed headless. |
| SWEEP-FEAT132-19 | Light theme | Open a file's life | The chart, the markers and the hunk read as clearly as in dark | P2 | Not run. |
| SWEEP-FEAT132-20 | A large repository | Open the life of a file with a long history and use the toolbar while it reads | The first page appears promptly, and Fetch and Pull are not held up for long | P1 | Not run. Each page is one `git log`, which holds Spagitty's own git lock while it runs. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-051 — What the author saw on first run

**Status:** Fixed — merged into `main`; the manual sweep is not yet run.
**Branch:** `bugfix/BUG-051-what-the-author-saw-on-first-run`
**Screens:** 1S, 1D, 1F, 1G, 1M, 1N, 1A.
**Raised by:** the author, 2026-10-06, from screenshots of the first release build with Merger (FEAT-100–FEAT-103, TASK-056), and the graph beside GitKraken.

## Problem

- Merger's plan squeezed its rows under one another: the Result card's buttons and notes were drawn over *The result lands*.
- A pair where nothing comes in still counted one merge commit and every file as changing.
- Resolving a merge with several files of one name (`package.json`) listed them identically.
- Branches, Tags, Stash and Reflog still named all four as tabs, though TASK-056 put each on the rail.
- The graph's lanes were squeezed together on a busy history and stayed so however wide the column was dragged; GitKraken's read cleaner.

## Change

- The plan's rows keep their content's height (`flex: none`); the plan scrolls.
- Nothing coming in reads 0 commits, 0 new commits, 0 files.
- The resolver's file list shows each file's folder under its name.
- Each refs screen shows only its own title and count.
- The graph column's resting room is 16 lanes, not 12, and dragging it wider spreads a squeezed history back out up to the design pitch — reversing the half of BUG-048 that kept lanes still on widening, at the author's request. Narrowing still moves no lane that was apart.

## Acceptance criteria

- None of the above is visible in the release build.

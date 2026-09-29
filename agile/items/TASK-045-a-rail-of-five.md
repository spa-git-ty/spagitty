<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-045 — A rail of five

**Status:** Open.
**Branch:** `task/TASK-045-a-rail-of-five`
**Screens:** chrome (the rail); Branches (1F), Tags (1N), Stash (1G), Reflog
(1M); Settings (1K), Personality; Badges (1P).
**Raised by:** the author: "I feel the software is … bloated", and, asked what
bothers them most, "too much on screen" and "feels slow or heavy". The
delight layer is to be kept, off by default.

## Problem

**Fourteen destinations in the rail.** TASK-041 grouped them, and grouping made
the list readable without making it shorter. Half of the rows are screens a
person visits to look one thing up, or only while something specific is going
on:

- Tags, Stash and Reflog are three more views of the refs the Branches screen
  already lists.
- Conflicts has nothing to show unless an operation has stopped.
- Rebase is started from the toolbar's Rebase button.
- Log is `Ctrl+F`, and the palette's first answer to "log".
- All repositories is the tab strip's `+`.
- Badges belongs to a layer the author wants off until asked for.

Every one of them is also a palette command. The rail is the only place they
compete for attention.

**The delight layer is on by default.** `Balanced` is the default personality:
an unlock arrives as a reward moment over the work, and Badges and God mode are
in the rail and in Settings from the first launch. God mode is a debug control
for the layer itself.

## Change

- **The rail is Farm; Graph, Working copy, Branches, Pull requests; Settings.**
- **Conflicts joins the rail while there is something in it** — a non-zero
  conflict count — and any screen that is not on the rail is shown there while
  it is the one open, so "where am I" always has an answer.
- **Branches, Tags, Stash and Reflog are one place.** The rail's Branches row is
  active on all four, and each of the four screens opens with the same
  segmented control naming all four, with their counts, in place of its title.
- **A fourth personality, `Off`, and it is the default.** Badges are still
  recorded, silently: nothing is shown, played or queued. Turning the layer on
  later shows everything that was earned in the meantime. The Badges screen and
  God mode are offered only when the layer is on.
- An existing `settings.json` that already names a personality keeps it. Only a
  file that names none, which is every new install, starts at `Off`.

## Non-scope

- The rail's look. The spatial shell that follows this item redraws it; this
  item decides what is in it.
- Settings' own sections, and the per-screen Fetch and Refresh buttons.
- Removing any screen, route or palette command.

## Acceptance criteria

- The rail shows six rows with a repository open and no conflicts: Farm, Graph,
  Working copy, Branches, Pull requests, Settings.
- A conflicted repository adds Conflicts after Working copy; resolving it takes
  the row away.
- Opening Log, Rebase, All repositories or Badges from the palette shows that
  screen's row in the rail, marked active, until you leave it.
- The Branches row is active on `/branches`, `/tags`, `/stash` and `/reflog`,
  and each of those screens can reach the other three in one click.
- A fresh install has personality `Off`: no reward moment, notice or sound on an
  unlock, no Badges row, no God mode section, and badges still recorded.
- Every screen reachable before is reachable after, from the palette.

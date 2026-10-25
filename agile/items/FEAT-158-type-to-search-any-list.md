<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-158 — Type to search any list

**Status:** Done — on `feature/FEAT-158-type-to-search`; unmerged. Covered by matcher, key-handling and graph tests; not looked at in the running application (the author is checking it).
**Branch:** `feature/FEAT-158-type-to-search`.
**Screens:** every screen with a list: Graph (1A), Branches, Tags, Reflog, Stash, Working copy, Repositories, Pull requests, Review, Issues, Releases, Investigate, Search, Files, Untrack, Terminal's sessions, the Worktrees and Submodules dialogs, Notifications.
**Raised by:** the author, 2026-10-25: "anywhere in the app where we have lists i shall have search ! even like intelijj like a ninja search no bar , or we add search bar on type !"

## Problem

Most lists could only be scrolled. A few had a filter field, which had to be clicked before anything could be typed, and the rest had nothing: finding one stash, one pull request or one commit among a hundred meant reading down the list.

## Change

IntelliJ's speed search, everywhere there is a list.

- **No bar to click.** With no text field focused, typing a letter on a screen starts a search of its list, and a small pill over the list's top right says what has been typed and how many match. Backspace takes a letter back, Escape puts the whole list back, Enter opens the first match. Clicking outside the list ends the search; clicking a match keeps it.
- **Which list.** The one holding the focus, else the one last clicked in, else the screen's first. With a dialog open only a list inside it is reached; with a menu open nothing is, since the menu has its own keys.
- **Matching.** Every word typed must be in the row, or its letters must start the row's words in order: `fb` finds `feature/BUG-098`, `cr` finds `CommitRows`. Case is ignored. What matched is marked where a row's main text is plain text.
- **Lists that already had a field** (Branches, Tags, Reflog, Files, Untrack, Notifications) take the letters in that field, so there is one query and not two.
- **The graph jumps instead of narrowing.** A commit taken out of the graph would break the lanes through it, so typing selects the first match from the current commit, the others are dimmed, and the arrow keys step from match to match. Only the commits walked so far are searched.
- **Screens that already answer to letters keep them.** On Stash `j` and `k` still step through hunks until a search is open. On Terminal the letters are the shell's; the sessions panel is searched only once clicked in.
- Only the rows narrow: *Stage all*, *Unstage all* and the like still act on every row.

## Acceptance criteria

- Typing a letter on any screen named above, with no field focused, narrows (or, on the graph, jumps through) its list, and the pill shows the query and the count.
- Backspace, Escape and Enter behave as above; Ctrl, Cmd and Alt shortcuts, a space, and keys typed into a field are untouched.
- A screen's own single-letter keys still work while no search is open.
- **Tests**: the matcher, the key handling (field, modifiers, space, reserved letters, menus, modals, field-backed lists, clicking away and in) and the graph's jump.

## Not in this item

- Searching rows the host or the walk has not delivered yet: Issues and Releases search the pages read so far, and the graph the commits walked so far.
- Lists of a few fixed entries (Settings sections, filter chips, the recipe catalogue), and the Farm's screens, whose lists are short and change as the farm runs.

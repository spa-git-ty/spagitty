<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-022 — Manual sweep

Test tickets for the graph's operations, its noise control, and its table.

**What this is.** The Graph screen (`/`) gained every verb it should have had:
right-click menus on commits and labels, drag-and-drop integration, multi-select
operations, hide/solo/smart visibility/pin-to-left, a configurable column table,
and a command palette.

**What these tickets are for.** The pure logic — hover ancestry, ghost paths,
the author filter, avatar colours, command ranking, lane geometry — is asserted
in `FEAT-022-automated.md`. What a test cannot answer is whether the graph is
*readable*, whether a confirmation actually appears before something
irreversible, and whether state survives a real repository being closed and
reopened. Several tickets are destructive by design; run them on a scratch
clone, never on work you need.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

**Preconditions shared by every ticket.** A scratch clone of a repository with
at least: one merge commit, two local branches, one remote, one tag, one stash,
and commits from more than one author. Spagitty open on the Graph screen.

---

## Operations on a commit

### SWEEP-22-01 — The commit menu offers everything, and explains what it will not do

- **Priority:** P1
- **Steps:** Right-click a commit that is not HEAD.
- **Expected:** Create branch here, Create tag here, Reset to this commit
  (soft/mixed/hard named by effect, not by flag), Revert, Cherry pick, Rebase
  onto this commit, Checkout this commit, Copy SHA. Anything that cannot run
  right now is **shown, disabled, with a short reason** — not hidden.
  Acceptance criterion 1.
- **Result:**

### SWEEP-22-02 — A destructive operation asks first

- **Priority:** P1
- **Steps:** Right-click a commit → Reset to this commit → hard.
- **Expected:** A dialog naming the commit and describing what hard reset does
  in words. Dismissing it (Escape, the backdrop, or Cancel) leaves the
  repository untouched — confirm with `git status` and `git log -1`.
  Acceptance criterion 2.
- **Result:**

### SWEEP-22-03 — Every operation reports its outcome

- **Priority:** P1
- **Steps:** Create a branch from a commit. Then attempt one that must fail —
  create a tag with a name that already exists.
- **Expected:** A notice bottom-right on success; on failure, a notice carrying
  **git's own message**, selectable so it can be copied. Neither silently does
  nothing. Acceptance criterion 2.
- **Result:**

### SWEEP-22-04 — Copy SHA puts the full id on the clipboard

- **Priority:** P2
- **Steps:** Right-click a commit → Copy SHA. Paste somewhere.
- **Expected:** The full 40-character id, not the abbreviation shown in the row.
- **Result:**

### SWEEP-22-05 — Checkout this commit enters detached HEAD, and says so

- **Priority:** P1
- **Steps:** Right-click a commit → Checkout this commit.
- **Expected:** It confirms first. Afterwards the header and the toolbar show
  a detached state rather than a branch name, and the graph reflects the new
  HEAD.
- **Result:**

## Operations on a branch label

### SWEEP-22-06 — Single click selects, double click checks out

- **Priority:** P1
- **Steps:** Single-click a branch label on a commit. Then double-click it.
- **Expected:** The single click does **not** check out. The double click does,
  and the current-branch marker moves.
- **Result:**

### SWEEP-22-07 — The label menu offers the integration verbs

- **Priority:** P1
- **Steps:** Right-click a branch label that is not checked out.
- **Expected:** Merge, Rebase, Fast-forward, Rename, Delete, Pin to the left,
  Show only this branch, Hide this branch. Delete is marked as destructive and
  is disabled with a reason when the branch is the one checked out.
- **Result:**

### SWEEP-22-08 — Deleting an unmerged branch warns differently

- **Priority:** P1
- **Steps:** Create a branch, commit on it, check out something else, then
  right-click it → Delete.
- **Expected:** The confirmation says the branch is not merged and that the
  commits will be unreachable — not the same wording as deleting a merged
  branch.
- **Result:**

### SWEEP-22-09 — Drag one branch onto another

- **Priority:** P1
- **Steps:** Drag branch `topic`'s label onto branch `main`'s label.
- **Expected:** A menu offering merge, rebase and fast-forward, whose wording
  makes clear which branch moves — the one you dragged, onto the one you
  dropped on. Fast-forward is disabled with a reason when it is not possible.
  Acceptance criterion 3.
- **Result:** Obsolete, 2026-10-19 — TASK-056: dropping a branch on another opens Merger; its sweep checks that.

### SWEEP-22-10 — Dropping a branch on itself does nothing

- **Priority:** P2
- **Steps:** Drag a branch label and drop it back on itself.
- **Expected:** No menu, no operation, no error notice.
- **Result:**

## Multi-select

### SWEEP-22-11 — Range and individual selection

- **Priority:** P1
- **Steps:** Click a row. Shift-click a row four below it. Then Ctrl/Cmd-click
  one row inside that range, and one outside it.
- **Expected:** Shift selects the whole range; Ctrl/Cmd removes the one inside
  and adds the one outside. The **detail panel keeps showing the row it was
  showing** — the multi-select is a second selection and must not hijack it.
- **Result:**

### SWEEP-22-12 — Cherry-pick a group

- **Priority:** P1
- **Steps:** Select three commits from a branch you are not on, right-click,
  Cherry pick.
- **Expected:** The menu names the count. It confirms first. Afterwards the
  three commits appear on the current branch **oldest first**, in their original
  order. Acceptance criterion 4.
- **Result:**

### SWEEP-22-13 — Rebase a range onto a branch

- **Priority:** P1
- **Steps:** Select a run of commits on a topic branch, then right-click the
  target branch's label.
- **Expected:** An entry naming both the count and the target. It confirms,
  then replays exactly that range. Acceptance criterion 4.
- **Result:**

### SWEEP-22-14 — A re-walk clears the multi-select

- **Priority:** P2
- **Steps:** Select several commits, then do something that rewrites history
  (or fetch, so refs move).
- **Expected:** The selection empties rather than pointing at rows that are no
  longer the same commits.
- **Result:**

## Noise control

### SWEEP-22-15 — Hide a branch

- **Priority:** P1
- **Steps:** Right-click a branch label → Hide this branch.
- **Expected:** Its commits leave the graph. The header chip stops saying "all
  branches" and names the filtered state. Acceptance criteria 5 and 6.
- **Result:**

### SWEEP-22-16 — There is always a way back

- **Priority:** P1
- **Steps:** With a branch hidden, open the gear at the top-right of the graph
  header.
- **Expected:** The hidden branch is listed by name under a Hidden heading with
  a way to unhide it, without the tester needing to remember what they hid.
  Same for soloed and pinned branches. Acceptance criterion 6.
- **Result:**

### SWEEP-22-17 — Solo

- **Priority:** P1
- **Steps:** Right-click a branch label → Show only this branch.
- **Expected:** Only that branch's history is drawn. Show all branches from the
  gear restores everything.
- **Result:**

### SWEEP-22-18 — Smart branch visibility

- **Priority:** P1
- **Steps:** Check out a topic branch. Gear → Smart branch visibility. Then
  check out a different branch **without touching the gear again**.
- **Expected:** Only the checked-out branch, what it is based on, and their
  upstreams. It re-computes against whatever is now checked out rather than
  staying pinned to the branch that was current when it was switched on.
- **Result:**

### SWEEP-22-19 — Pin to left

- **Priority:** P2
- **Steps:** Right-click `main`'s label → Pin to the left. Scroll through a
  region with several branches.
- **Expected:** `main` holds the leftmost lane throughout instead of its column
  moving as other branches come and go. The menu entry now reads Unpin.
- **Result:**

### SWEEP-22-20 — Visibility survives reopening the repository

- **Priority:** P1
- **Steps:** Hide a branch and pin another. Close the repository, open a
  *different* one, then reopen the first.
- **Expected:** The first repository's hidden and pinned branches are as they
  were, and the second repository was not affected by them.
  Acceptance criterion 5.
- **Result:**

### SWEEP-22-21 — Hovering a branch highlights its history

- **Priority:** P2
- **Steps:** Hover a branch label.
- **Expected:** Every commit in that branch stays bright and everything else
  dims — including **both** sides of any merge on it. Moving away restores
  everything.
- **Result:**

### SWEEP-22-22 — Ghost branch

- **Priority:** P2
- **Steps:** Hover a commit that carries no label of its own.
- **Expected:** A faded connector from it up to the nearest commit that does
  carry one. A commit that already has a label shows nothing.
- **Result:**

### SWEEP-22-23 — The author filter dims, never removes

- **Priority:** P1
- **Steps:** Show the Author column, type another author's name into its filter.
- **Expected:** Non-matching rows **dim**; the lanes are unchanged and no row
  disappears. The row count does not change. Acceptance criterion 8.
- **Result:**

## The table

### SWEEP-22-24 — Toggle columns

- **Priority:** P1
- **Steps:** Right-click the column header.
- **Expected:** Every column with a tick beside the shown ones. Graph and Commit
  Message are disabled with the reason "always shown". Toggling Author,
  Date/Time and SHA adds and removes them immediately.
- **Result:**

### SWEEP-22-25 — Reorder and resize

- **Priority:** P2
- **Steps:** Drag a column header sideways. Drag a divider.
- **Expected:** The column lands where it was dropped. Dividers resize, except
  the Graph column's, which is computed and says so in its tooltip. A column
  cannot be dragged narrower than its minimum.
- **Result:**

### SWEEP-22-26 — Layout is remembered per repository

- **Priority:** P1
- **Steps:** In repository A show SHA and widen Author. Open repository B.
  Return to A.
- **Expected:** B has its own layout; A is exactly as it was left.
  Acceptance criterion 7.
- **Result:**

### SWEEP-22-27 — Author avatars

- **Priority:** P3
- **Steps:** Show the Author column on a repository with several authors.
- **Expected:** A coloured disc with one or two initials beside each name. The
  same author always gets the same colour and the same initials, in both light
  and dark themes, with the letters legible on the disc. **No network request
  is made** — check with the network disconnected.
- **Result:**

### SWEEP-22-28 — The lane column grows and settles

- **Priority:** P2
- **Steps:** Scroll through a region where the branch count changes a lot.
- **Expected:** The lane column widens immediately when more lanes appear and
  narrows only after the narrower view has held for a moment — the message
  column must not jump left and right while reading.
- **Result:**

### SWEEP-22-29 — The graph is no wider than the history needs

- **Priority:** P2
- **Steps:** Open a repository with several parallel branches and compare the
  graph column to the width of the same history in another client, or to
  `git log --graph`.
- **Expected:** Lanes sit tight and adjacent with short elbows; a branch that
  changes lane does so within one row rather than drifting sideways over
  several. Two adjacent lanes are clearly two lines, not one thick one.
- **Result:**

## Palette, zoom, and the shell

### SWEEP-22-30 — The palette opens and finds things by initials

- **Priority:** P1
- **Steps:** Press `Ctrl/Cmd+P`. Type `gts`. Then type `zi`.
- **Expected:** The palette opens with everything listed under headings; the
  initials find the matching command; Enter runs it and closes the palette.
  A command that cannot run right now is greyed with its reason rather than
  hidden. Acceptance criterion 9.
- **Result:**

### SWEEP-22-31 — Zoom and text size are two dials

- **Priority:** P2
- **Steps:** `Ctrl/Cmd` and `+` a few times. Then change text size from
  Settings → Appearance. Then `Ctrl/Cmd+0`.
- **Expected:** Zoom scales the whole interface including row height and lane
  spacing; text size changes text within it; the rows and the lane canvas stay
  aligned at every combination; `Ctrl/Cmd+0` returns **both** to 100%.
- **Result:**

### SWEEP-22-32 — A confirmation survives navigating away

- **Priority:** P2
- **Steps:** Start an operation that asks a question, then — while the dialog
  is open — try to navigate with the nav rail.
- **Expected:** The question stays on screen and stays answerable; it is not
  dismissed silently by the navigation, and its result is reported wherever the
  user has ended up.
- **Result:**

### SWEEP-22-33 — Fetch and push report honestly

- **Priority:** P1
- **Steps:** Fetch. Then push a branch with no upstream.
- **Expected:** Both report through a notice. The push either sets the upstream
  and says so, or fails with git's own message — it never silently succeeds
  while nothing reached the remote. Verify against the remote.
- **Result:**

### SWEEP-22-34 — Nothing is offered that the graph refuses to do

- **Priority:** P3
- **Steps:** Try to drag a *commit* row (not a label) onto another row or lane.
  Try to edit an old commit's message from the graph.
- **Expected:** Neither is possible, and neither is advertised in a menu. These
  are documented non-scope: reordering lives in Interactive rebase, and editing
  an old message needs an interactive rebase.
- **Result:**

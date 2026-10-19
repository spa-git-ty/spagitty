<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-005 — Manual sweep

Test tickets for the Stash screen (1G).

**Fixture.** The repository from `docs/testing.md`, which already carries one
stash entry. Keep a terminal open: most tickets are "does Spagitty agree with
`git stash list`".

**Use a fixture.** Stashing moves files out of the working copy, and bringing
them back is not built yet.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1G-01 — The list matches git

- **Priority:** P1
- **Steps:** Compare against `git stash list`.
- **Expected:** Same entries, same order, `stash@{0}` first. Each row shows the
  message git shows.
- **Result:**

### SWEEP-1G-02 — Each entry hangs off its commit

- **Priority:** P2
- **Steps:** Read the lane drawing and the second line of a row; compare the
  short SHA against `git log --format=%h -1 stash@{0}^`.
- **Expected:** Two nodes joined by a curve, and the row names that commit and
  its summary. A stash is a commit with a parent, and the drawing says so.
- **Result:**

### SWEEP-1G-03 — What is in an entry

- **Priority:** P1
- **Steps:** Select an entry and compare the file list against
  `git stash show --name-only stash@{n}`.
- **Expected:** The same files, and `+n −m` matching `git stash show`. A dotfile
  reads as `.gitignore`, not `gitignore.`.
- **Result:**

### SWEEP-1G-04 — Opening the full diff

- **Priority:** P2
- **Steps:** Press **Open full diff →**.
- **Expected:** The Diff screen opens on the stash commit and shows its hunks —
  a stash is a commit, and the same screen reads it.
- **Result:**

### SWEEP-1G-05 — Stashing

- **Priority:** P1
- **Preconditions:** A tracked file edited.
- **Steps:**
  1. Type a message and press **Stash**.
  2. Check `git stash list` and `git status`.
- **Expected:** A new `stash@{0}` carrying the message, the working copy clean,
  the rail's Stash count up by one, and the Working copy count down. The message
  box clears.
- **Result:**

### SWEEP-1G-06 — Untracked files are left alone unless asked

- **Priority:** P1
- **Preconditions:** One tracked edit and one untracked file.
- **Steps:** Stash without **include untracked**, then check the directory.
- **Expected:** The untracked file is still on disk and still untracked. Only
  the tracked change was taken.
- **Result:**

### SWEEP-1G-07 — Including untracked files

- **Priority:** P2
- **Preconditions:** An untracked file, nothing else changed.
- **Steps:** Turn **include untracked** on and stash.
- **Expected:** The file is gone from disk and is in the entry.
- **Result:**

### SWEEP-1G-08 — Stashing nothing is refused

- **Priority:** P1
- **Preconditions:** A clean working copy.
- **Steps:** Press **Stash**.
- **Expected:** A message saying there is nothing to stash. No new entry
  appears. (`git stash push` exits 0 in this case and creates nothing, which
  from a button would read as a stash that happened and then vanished.)
- **Result:**

### SWEEP-1G-09 — Untracked-only, without including untracked

- **Priority:** P2
- **Preconditions:** Only untracked files changed, **include untracked** off.
- **Steps:** Press **Stash**.
- **Expected:** Refused, and the reason names untracked files. Turning the chip
  on and pressing again works.
- **Result:**

### SWEEP-1G-10 — Restoring says it is not built

- **Priority:** P1
- **Steps:** Point at **Pop**, **Apply** and **Drop**.
- **Expected:** Each says what it would do and that it is not built yet, and
  none does anything when clicked. The panel names the terminal command that
  does it today. A stash you cannot restore is a trap; the screen must not
  pretend otherwise.
- **Result:** Obsolete, 2026-10-19 — FEAT-014 built apply, pop and drop; its sweep checks them.

### SWEEP-1G-11 — An empty stash explains itself

- **Priority:** P2
- **Preconditions:** A repository with no stash.
- **Steps:** Open the screen.
- **Expected:** It says nothing is stashed and explains what a stash is for,
  rather than showing an empty list.
- **Result:**

### SWEEP-1G-12 — Outside changes are noticed

- **Priority:** P2
- **Steps:** With the screen open, run `git stash push` in the terminal, then
  press **Refresh**.
- **Expected:** The new entry appears at the top and the previously open entry
  stays open if it is still there.
- **Result:**

### SWEEP-1G-13 — Both themes

- **Priority:** P2
- **Steps:** Toggle the theme.
- **Expected:** The lane nodes and the elbow stay visible, the selected row is
  still obviously selected, and the disabled chips still read as disabled.
- **Result:**

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-009 — Manual sweep

Test tickets for the Interactive rebase screen (1E).

**Fixture.** A throwaway repository with a branch of at least five commits on
top of `main`, where two of those commits touch the same file and the rest touch
different ones. Keep a terminal open: several tickets check that the repository
was *not* changed, which is the whole point of this screen.

**Nothing on this screen runs.** If any ticket produces a rebase in progress,
that is a P1 failure regardless of what else the ticket says.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1E-01 — The todo list matches what git would open

- **Priority:** P1
- **Steps:** Set **onto** to `main` and press Plan. Compare the list against
  `git rev-list --reverse --no-merges main..HEAD --format=oneline`.
- **Expected:** The same commits, in the same order — oldest at the top, because
  a rebase replays forwards. No merge commits. Acceptance criterion 1.
- **Result:**

### SWEEP-1E-02 — The upstream is offered, not guessed

- **Priority:** P2
- **Steps:** Open the screen on a branch that tracks an upstream, then on one
  that does not. Open the **onto** field's suggestions.
- **Expected:** A tracked branch has its upstream filled in; an untracked one
  leaves the field empty rather than guessing. The suggestions list every branch
  except the one you are on.
- **Result:**

### SWEEP-1E-03 — Squash folds upward

- **Priority:** P1
- **Steps:** Set the second row to **squash**.
- **Expected:** The result loses that row, and the row above it says "+1
  squashed". Squashing folds into the commit above, which is the direction git
  folds. Acceptance criterion 2.
- **Result:**

### SWEEP-1E-04 — A leading squash is refused

- **Priority:** P1
- **Steps:** Set the **first** row to squash.
- **Expected:** The result pane says the plan cannot run and why — there is
  nothing above the first commit to fold into. No result rows are drawn.
- **Result:**

### SWEEP-1E-05 — Drop and reword

- **Priority:** P1
- **Steps:** Drop one commit; reword another.
- **Expected:** The dropped commit stays visible in the plan, drawn dashed, and
  disappears from the result; the footer counts it. The reworded one stays in
  the result marked "reworded" — git asks for the message when the rebase runs,
  which is git's own behaviour and not something this screen collects.
- **Result:**

### SWEEP-1E-06 — Reordering by drag

- **Priority:** P1
- **Steps:** Drag the last row to the top.
- **Expected:** The plan reorders and the result follows immediately. The result
  is always a picture of the plan as it stands.
- **Result:**

### SWEEP-1E-07 — Reordering from the keyboard

- **Priority:** P1
- **Steps:** Click a row's body to focus it, then press `⌥↑` and `⌥↓` several
  times, including at the top and bottom of the list.
- **Expected:** The row moves each time and keeps the focus, so it can be moved
  again without re-clicking. It does not move off either end. A plain arrow key
  moves nothing. Reordering must not require a pointer.
- **Result:**

### SWEEP-1E-08 — Dropping everything

- **Priority:** P2
- **Steps:** Set every row to drop.
- **Expected:** An empty result with a plain statement that the branch would end
  up at the upstream with nothing of its own. A warning, not an error — this is
  a legitimate thing to look at before deciding against it. Acceptance
  criterion 3.
- **Result:**

### SWEEP-1E-09 — "May conflict" appears where it should, and says "may"

- **Priority:** P1
- **Preconditions:** Two commits in the range touch the same file.
- **Steps:** Plan the range and read the result.
- **Expected:** The later of the two is marked **may conflict**. The word is
  "may": whether they actually clash is only known once the merges run. A row
  claiming a clean result it cannot prove is a failure of this ticket.
  Acceptance criterion 4.
- **Result:**

### SWEEP-1E-10 — Nothing is written, ever

- **Priority:** P1
- **Steps:**
  1. Note `git rev-parse HEAD`, `git rev-parse ORIG_HEAD` (if it exists), and
     `git status --porcelain`.
  2. In Spagitty: reorder rows, squash, reword, drop, drop everything, reset,
     re-plan onto a different upstream — as much editing as you can stand.
  3. Check all three again, plus `ls .git/rebase-merge .git/rebase-apply`.
- **Expected:** HEAD unchanged, `ORIG_HEAD` unchanged, status identical, and
  neither rebase directory exists. `git status` must not say "interactive rebase
  in progress". Acceptance criterion 5.
- **Result:** Obsolete, 2026-10-19 — FEAT-015 made Rebase run; its sweep checks the run, the stop and the abort.

### SWEEP-1E-11 — Apply is disabled and says why

- **Priority:** P1
- **Steps:** Hover **Apply**.
- **Expected:** Visibly disabled, with a title naming FEAT-015 and saying this
  screen plans while git executes. It cannot be clicked into doing anything.
  Acceptance criterion 7.
- **Result:** Obsolete, 2026-10-19 — FEAT-015 made Rebase run; its sweep checks the run, the stop and the abort.

### SWEEP-1E-12 — An upstream with no shared history

- **Priority:** P1
- **Preconditions:** An orphan branch (`git switch --orphan stranger`, then one
  commit).
- **Steps:** Plan onto `main` from it.
- **Expected:** A stated refusal naming the reason — there is no merge base, so
  there is nothing to rebase onto. Not an empty list and not a crash. Acceptance
  criterion 6.
- **Result:**

### SWEEP-1E-13 — Nothing to rebase

- **Priority:** P2
- **Steps:** Plan a branch onto itself, or onto an upstream that already
  contains it.
- **Expected:** A plain statement that there are no commits the upstream does
  not already have. No empty table.
- **Result:**

### SWEEP-1E-14 — Reset

- **Priority:** P2
- **Steps:** Make several edits, then press **Reset**.
- **Expected:** The plan returns to the list git would have opened, in the
  original order with every row picked, and the result follows. Reset is
  disabled while nothing has been edited.
- **Result:**

### SWEEP-1E-15 — A long range

- **Priority:** P3
- **Preconditions:** A branch more than 250 commits ahead of its upstream.
- **Steps:** Plan it.
- **Expected:** The first 250 rows, and a footer saying the list was cut and
  why. The cap is stated, never applied silently.
- **Result:**

### SWEEP-1E-16 — Leaving and coming back

- **Priority:** P3
- **Steps:** Build a plan, navigate to another screen, come back.
- **Expected:** The screen comes back ready rather than showing a stale plan
  against a repository that may have moved underneath it.
- **Result:**

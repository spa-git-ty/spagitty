<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-008 — Manual sweep

Test tickets for the Conflicts screen (1D).

**Fixture.** A throwaway repository you can put into a real merge conflict, and
a second clean one. Keep a terminal open in the conflicted repository: almost
every ticket is "does Spagitty agree with `git`".

Build the conflicted state:

```
git init -b main conflicted && cd conflicted
printf 'one\ntwo\nthree\n' > shared.txt
printf 'calm\n' > untouched.txt
git add -A && git commit -m Base
git switch -c theirs
printf 'one\nTHEIRS\nthree\n' > shared.txt && git commit -am 'Their change'
git switch main
printf 'one\nOURS\nthree\n' > shared.txt && git commit -am 'Our change'
git merge theirs        # expected to stop
```

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1D-01 — The conflicted files match git

- **Priority:** P1
- **Steps:** Open the conflicted repository and go to **Conflicts**. Compare the
  pager's chips against `git diff --name-only --diff-filter=U`.
- **Expected:** The same paths, no more and no fewer. `untouched.txt` is not
  listed. Acceptance criterion 1.
- **Result:**

### SWEEP-1D-02 — The rail's Conflicts count stops being a dot

- **Priority:** P1
- **Steps:** Read the Conflicts entry in the nav rail.
- **Expected:** A number, equal to the number of conflicted files, not `·`.
  Resolve one in the terminal (`git checkout --ours shared.txt && git add
  shared.txt`), return to Spagitty and refresh — the count follows. Acceptance
  criterion 2.
- **Result:**

### SWEEP-1D-03 — Ours, theirs and the base are what git says they are

- **Priority:** P1
- **Steps:** With `shared.txt` open, compare each pane against the terminal:
  `git show :2:shared.txt` for Ours, `git show :3:shared.txt` for Theirs, and
  `git show :1:shared.txt` for the Common ancestor disclosure.
- **Expected:** Identical, line for line, including any trailing newline —
  Spagitty shows the index's own content, not a re-rendering of it. Acceptance
  criterion 3.
- **Result:**

### SWEEP-1D-04 — The merged pane is the file on disk

- **Priority:** P1
- **Steps:** Compare the middle pane against `cat shared.txt`.
- **Expected:** The same text, conflict markers included, with the marker lines
  highlighted. This pane is the file you would open in an editor.
- **Result:**

### SWEEP-1D-05 — A file added on both sides

- **Priority:** P2
- **Preconditions:** Build one:
  ```
  git merge --abort
  git switch theirs && printf 'their new\n' > both.txt && git add both.txt && git commit -m 'They add'
  git switch main && printf 'our new\n' > both.txt && git add both.txt && git commit -m 'We add'
  git merge theirs
  ```
- **Steps:** Open `both.txt` and expand the Common ancestor disclosure.
- **Expected:** The base pane says there is no common ancestor because both
  sides added the file. Ours and Theirs both have content. Nothing errors.
  Acceptance criterion 4.
- **Result:**

### SWEEP-1D-06 — A file one side deleted

- **Priority:** P1
- **Preconditions:** Build a delete/modify conflict: one branch deletes a file,
  the other changes it, then merge.
- **Steps:** Open the deleted path.
- **Expected:** The pane for the deleting side says the file was **deleted on
  that side** — not an empty pane. An empty pane would read as "they emptied the
  file", which is a different thing and one that loses work if acted on. The
  pager chip's tooltip says which side deleted it. Acceptance criterion 5.
- **Result:**

### SWEEP-1D-07 — The operation is named, and named correctly

- **Priority:** P1
- **Steps:**
  1. During the merge, read the header and the footer.
  2. `git merge --abort`, then stop a **cherry-pick** on a conflict instead
     (`git cherry-pick <a commit that conflicts>`).
  3. Return to the screen and refresh.
- **Expected:** The header says "merge in progress" in the first case and
  "cherry-pick in progress" in the second, and the footer names the matching
  `git … --abort`. The index looks identical in both cases, so this can only be
  right if the screen is reading the repository's state rather than guessing.
  Acceptance criterion 6.
- **Result:**

### SWEEP-1D-08 — A calm empty state

- **Priority:** P2
- **Steps:** Open the clean repository and go to Conflicts.
- **Expected:** No panes, no error, no pager. A short explanation of what would
  appear here and why. Acceptance criterion 7.
- **Result:**

### SWEEP-1D-09 — Nothing is written

- **Priority:** P1
- **Steps:**
  1. In the terminal: `stat -c %y .git/index` and `git status --porcelain > /tmp/before`.
  2. In Spagitty, page through **every** conflicted file, expand every Common
     ancestor disclosure, and press **Refresh** three times.
  3. `stat -c %y .git/index`, `ls .git/index.lock`, and
     `git status --porcelain > /tmp/after; diff /tmp/before /tmp/after`.
- **Expected:** The index mtime is unchanged, there is no lock file, and the
  status output is identical. Acceptance criterion 8.
- **Result:** Obsolete, 2026-10-19 — FEAT-016 made Conflicts write: take a side, resolve a region, edit, continue and abort.

### SWEEP-1D-10 — The write actions are disabled and say why

- **Priority:** P1
- **Steps:** Hover **Mark resolved** and **Abort**.
- **Expected:** Both are visibly disabled and their tooltips name FEAT-016 and
  the reason. Neither can be clicked into doing anything. The footer states
  permanently that this screen only reads.
- **Result:** Obsolete, 2026-10-19 — FEAT-016 made Conflicts write: take a side, resolve a region, edit, continue and abort.

### SWEEP-1D-11 — Paging between files

- **Priority:** P2
- **Preconditions:** At least three conflicted files.
- **Steps:** Use **Previous** and **Next**, then click chips directly.
- **Expected:** The position reads "n of m" and follows. Previous is disabled on
  the first file and Next on the last — neither wraps. The chip for the open
  file is marked.
- **Result:**

### SWEEP-1D-12 — A binary conflict

- **Priority:** P2
- **Preconditions:** A conflicted binary file (change the same binary on both
  branches, then merge).
- **Steps:** Open it.
- **Expected:** Each pane says the side is binary and how many bytes it is,
  rather than rendering bytes as text. Spagitty calls the same files binary that
  `git diff` does.
- **Result:**

### SWEEP-1D-13 — Resolving outside Spagitty while the screen is open

- **Priority:** P2
- **Steps:** With a file open in Spagitty, resolve it in the terminal
  (`git checkout --ours <path> && git add <path>`), then press **Refresh**.
- **Expected:** The file leaves the pager and the screen moves to the first
  remaining conflict rather than showing three sides of a file that is no longer
  conflicted. When it was the last one, the empty state appears.
- **Result:**

### SWEEP-1D-14 — A deeply nested path and a dotfile

- **Priority:** P3
- **Preconditions:** Conflicts on `src/deep/nested/main.rs` and `.gitignore`.
- **Steps:** Read the chips and the path line above the panes.
- **Expected:** The chip shows the file name and its tooltip the full path; the
  path line shows the full path. `.gitignore` reads as `.gitignore`, not
  `gitignore.`.
- **Result:**

### SWEEP-1D-15 — A large conflicted file stays usable

- **Priority:** P3
- **Preconditions:** A conflict in a file of a few megabytes.
- **Steps:** Open it and scroll each pane.
- **Expected:** The window does not freeze, and the panes scroll independently.
  Above the ceiling the pane says the side is too large rather than trying to
  draw it — note the file size and the behaviour on this ticket.
- **Result:**

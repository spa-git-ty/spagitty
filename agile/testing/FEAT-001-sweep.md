<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-001 — Manual sweep

Test tickets for the Graph screen (1A) and the application chrome. A tester who
did not write the code should be able to run these without asking questions.

**Fixture.** A repository with at least: a merge commit, a branch that is merged
and one that is not, an annotated tag and a lightweight tag, two stash entries,
a file with a long path, and a dotfile. The session fixture used during
development is built by the script recorded in `docs/testing.md`.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1A-01 — A repository opens and paints

- **Priority:** P1
- **Preconditions:** Spagitty running, no repository open.
- **Steps:**
  1. Press **Open repository…** in the nav rail footer.
  2. Choose the fixture repository.
- **Expected:** The repository's tab carries its name and the toolbar its current branch.
  Rows appear progressively rather than all at once after a pause. The rail's
  Graph count rises as rows arrive and settles on the total.
- **Result:**

### SWEEP-1A-02 — Rows keep a fixed pitch

- **Priority:** P1
- **Preconditions:** Fixture open, history longer than one screenful.
- **Steps:**
  1. Watch the first screenful while the rest of the walk streams in.
  2. Scroll to the bottom and back to the top.
- **Expected:** No row moves vertically once drawn. Row spacing is identical
  everywhere. Lane lines meet their nodes exactly, with no drift at the bottom.
- **Result:**

### SWEEP-1A-03 — A merge draws a one-row elbow

- **Priority:** P1
- **Preconditions:** Fixture open, merge commit visible.
- **Steps:**
  1. Find the merge commit.
  2. Look at the band immediately below its node.
- **Expected:** A curved elbow spans exactly one row from the merge's lane out
  to the second parent's lane, drawn in the incoming lane's colour, not the
  merge's.
- **Result:**

### SWEEP-1A-04 — Lane colour is stable

- **Priority:** P2
- **Preconditions:** Fixture open with a branch running several commits.
- **Steps:** Follow one branch's lane from its tip to where it joins.
- **Expected:** The colour never changes along the way.
- **Result:**

### SWEEP-1A-05 — Ref chips and their order

- **Priority:** P2
- **Preconditions:** Fixture open. HEAD on a branch that also carries a tag.
- **Steps:** Look at the refs gutter beside that commit.
- **Expected:** The current branch is first, with an accent border and a check.
  Then other local branches, then remotes, then tags. An annotated tag appears
  on its commit, not on a separate row. `origin/HEAD` is not shown.
- **Result:**

### SWEEP-1A-06 — Selecting and opening a commit

- **Priority:** P1
- **Preconditions:** Fixture open.
- **Steps:**
  1. Click a commit row once.
  2. Press the up and down arrow keys.
  3. Double-click a row.
- **Expected:** One click selects and fills the detail panel. Arrows move the
  selection and the panel follows. Double-click opens the Diff screen for that
  commit.
- **Result:**

### SWEEP-1A-07 — Detail panel content

- **Priority:** P2
- **Preconditions:** A commit with a multi-line message and several changed
  files selected.
- **Steps:** Read the panel top to bottom.
- **Expected:** Full SHA, subject, body, author and committer with times, parent
  list, and the changed files. A dotfile renders as `.gitignore`, not
  `gitignore.`. Long paths elide at the head, keeping the filename visible.
- **Result:**

### SWEEP-1A-08 — Counts do not lie

- **Priority:** P1
- **Preconditions:** Fixture open.
- **Steps:** Read every count in the nav rail.
- **Expected:** Graph, Branches and Stash show real numbers matching
  `git rev-list --count`, `git branch | wc -l` and `git stash list | wc -l`.
  Working copy and Conflicts show `·` — not `0` — because those screens do not
  compute them yet. The footer shows real tag and submodule counts.
- **Result:**

### SWEEP-1A-09 — The graph follows the repository

- **Priority:** P1
- **Preconditions:** Fixture open in Spagitty and in a terminal.
- **Steps:**
  1. In the terminal, commit something on the current branch.
  2. Watch Spagitty without touching it.
  3. In the terminal, run a `git checkout` of another branch.
- **Expected:** The new commit appears at the top within about a second, and the
  branch chip moves with it. Checking out changes the title bar and toolbar
  branch. The view does not flicker or scroll away from where you were.
- **Result:**

### SWEEP-1A-10 — Unborn and empty repositories

- **Priority:** P2
- **Preconditions:** An empty directory initialised with `git init` and nothing
  committed.
- **Steps:** Open it.
- **Expected:** The screen says the repository has no commits yet. It does not
  show an error dialog, and it does not hang.
- **Result:**

### SWEEP-1A-11 — Not a repository

- **Priority:** P2
- **Preconditions:** Any directory that is not inside a git repository.
- **Steps:** Open it.
- **Expected:** A plain message naming the path and saying it is not a git
  repository. The previously open repository, if any, is unaffected.
- **Result:**

### SWEEP-1A-12 — Panels resize and persist

- **Priority:** P3
- **Preconditions:** Fixture open.
- **Steps:**
  1. Drag the splitter between the rail and the graph to its extremes.
  2. Drag the detail panel's splitter likewise.
  3. Close and reopen Spagitty.
- **Expected:** Neither panel can be dragged to uselessness — the rail stops
  between 140px and 340px, the detail panel between 200px and 520px. Widths
  come back after a restart.
- **Result:**

### SWEEP-1A-13 — Both themes

- **Priority:** P2
- **Preconditions:** Fixture open.
- **Steps:** Switch between Light and Dark in Settings → Appearance; read every surface.
- **Expected:** Text stays legible against its background in both themes. Lane
  colours stay tellable apart. The selected row and the active rail item are
  still obviously the selected ones.
- **Result:**

### SWEEP-1A-14 — Window chrome

- **Priority:** P3
- **Preconditions:** Spagitty running.
- **Steps:** Drag the title bar, drag each window edge and corner, then use the
  minimise, maximise and close buttons.
- **Expected:** The window moves and resizes from every edge and corner. All
  three buttons do what they say.
- **Result:**

### SWEEP-1A-15 — A wide history does not crowd out the messages

- **Priority:** P2
- **Preconditions:** A repository with many concurrent branches — `git/git` if
  available.
- **Steps:** Open it and scroll through a busy region.
- **Expected:** The lane column stops widening at twelve columns. Lanes past the
  cap clamp to the last column but keep their own colour. The message column
  stays readable.
- **Result:**

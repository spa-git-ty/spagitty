<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-012 — Manual sweep

Test tickets for the Clone modal (1L).

**What this is.** A modal reached from the toolbar's **Clone** button and from
the **Clone…** button on All repositories. It takes an address and a folder,
shows the exact path the clone will land at, runs `git clone`, streams git's
progress, and offers to open the result.

**Before you start.** Have a scratch folder you do not mind filling, a public
repository URL, and — for SWEEP-1L-05 — a large one. Nothing here needs an
account.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1L-01 — A clone matches what `git clone` produces

- **Priority:** P1
- **Steps:** Open Clone, paste a public repository URL, choose your scratch
  folder, press **Clone**. When it finishes, run `git clone <same url>` into a
  different folder from a terminal. Compare: `git rev-parse HEAD`,
  `git branch -a`, `git remote -v`, and `git status` in both.
- **Expected:** Identical. Same commit, same refs, `origin` pointing at the same
  URL, both clean. Acceptance criterion 1.
- **Result:**

### SWEEP-1L-02 — The path shown is the path created

- **Priority:** P1
- **Steps:** Type an address and choose a folder. Read the **Result** line
  before pressing anything. Then clone and look at where it actually landed.
- **Expected:** Exactly the same path, character for character. Try
  `…/project.git`, `…/project`, `…/project.git/` and a scp-style
  `git@host:owner/project.git` — all four say `<folder>/project`. Acceptance
  criterion 2.
- **Result:**

### SWEEP-1L-03 — A destination that is already occupied

- **Priority:** P1
- **Preconditions:** A folder `<scratch>/project` containing at least one file.
- **Steps:** Set up a clone of a URL whose name is `project` into `<scratch>`.
- **Expected:** The **Clone** button is disabled, and the reason says the path
  already exists and is not empty and that **nothing has been changed**. Check
  your file is still there. Acceptance criterion 3.
- **Result:**

### SWEEP-1L-03b — A destination that exists but is empty

- **Priority:** P2
- **Preconditions:** An empty folder `<scratch>/project`.
- **Steps:** Clone into it.
- **Expected:** Allowed, and it works — `git clone` allows this, and Spagitty
  must not be stricter than the command line.
- **Result:**

### SWEEP-1L-04 — Opening what was cloned

- **Priority:** P1
- **Steps:** After a successful clone, press **Open it**.
- **Expected:** The modal closes, the repository opens, its tab names it,
  the graph paints its history, and it appears in **All repositories**.
  Acceptance criterion 7.
- **Result:**

### SWEEP-1L-05 — Progress moves for a large repository

- **Priority:** P1
- **Steps:** Clone something big enough to take at least twenty seconds.
  Watch the bar and the text.
- **Expected:** The phase changes — counting, compressing, receiving, resolving,
  updating files — and the percentage climbs. It does not sit at 0% and then
  jump to done. Acceptance criterion 4.
- **Result:**

### SWEEP-1L-06 — Cancelling removes what the clone created

- **Priority:** P1
- **Steps:** Start a large clone into a folder where the destination did **not**
  exist. Press **Stop** part-way. Then look in the scratch folder, in a file
  manager or with `ls`.
- **Expected:** git stops. The destination directory is gone entirely — not
  present and empty, not present with half a repository in it. Acceptance
  criterion 6.
- **Result:**

### SWEEP-1L-07 — Cancelling does not remove a folder that was already there

- **Priority:** P1
- **Preconditions:** An **empty** folder `<scratch>/project` that you created.
- **Steps:** Start a large clone into it, then press **Stop**.
- **Expected:** `<scratch>/project` still exists. Spagitty created neither the
  folder nor your right to have it removed. Acceptance criterion 6, and the half
  that is easy to get wrong.
- **Result:**

### SWEEP-1L-08 — Stopping leaves no process behind

- **Priority:** P2
- **Steps:** Start a large clone, press **Stop**, then run `pgrep -af "git
  clone"` (or Task Manager / Activity Monitor).
- **Expected:** Nothing. No orphan `git clone` still writing to disk.
- **Result:**

### SWEEP-1L-09 — A clone survives navigation

- **Priority:** P1
- **Steps:** Start a large clone. Close the modal with **✕** or `Esc`, move
  around the nav rail, then open Clone again from the toolbar.
- **Expected:** The clone is still running and the progress is where it should
  be, not restarted and not cancelled. The modal is a view of the clone, not the
  clone itself.
- **Result:**

### SWEEP-1L-10 — A repository that needs credentials nothing can supply

- **Priority:** P1
- **Preconditions:** A private repository URL, on a machine with no credential
  helper configured for that host — or temporarily unset one.
- **Steps:** Clone it. Wait a full minute.
- **Expected:** It **fails** with git's own message — authentication failed,
  repository not found, or similar. It does **not** hang, and Spagitty never
  shows a password box of its own. Acceptance criterion 5.
- **Result:**

### SWEEP-1L-11 — A credential helper still works

- **Priority:** P1
- **Preconditions:** A private repository you can clone from the command line
  because a helper or an ssh key supplies the credentials.
- **Steps:** Clone it through Spagitty.
- **Expected:** It succeeds, with no prompt, exactly as the command line does.
  This is the reason cloning goes through `git` at all.
- **Result:**

### SWEEP-1L-12 — An address that does not exist

- **Priority:** P2
- **Steps:** Clone `https://example.com/owner/definitely-not-a-repo.git`.
- **Expected:** It fails and shows git's own words. The **Clone** button comes
  back so it can be tried again with a corrected address, and nothing appears in
  **All repositories**. Acceptance criterion 8.
- **Result:**

### SWEEP-1L-13 — Both ways in

- **Priority:** P2
- **Steps:** Open the modal from the toolbar's **Clone**, close it, then from
  **All repositories → Clone…**, and again from the empty state when no
  repository has ever been opened.
- **Expected:** The same modal each time.
- **Result:**

### SWEEP-1L-14 — Nothing outside the destination is touched

- **Priority:** P1
- **Steps:** Note the contents of your scratch folder. Clone into it, then
  cancel a second clone, then let a third fail. Compare the folder afterwards.
- **Expected:** Only the destination directories appear or disappear. Nothing
  else in the folder is created, modified or removed.
- **Result:**

### SWEEP-1L-15 — A second clone while one is running

- **Priority:** P3
- **Steps:** Start a large clone. Without stopping it, try to start another —
  the fields are disabled, so this may need reopening the modal.
- **Expected:** Refused with a message saying one is already running. Nothing is
  queued and nothing silently replaces the first.
- **Result:**

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-004 — Manual sweep

Test tickets for the Branches screen (1F).

**Fixture.** The repository from `docs/testing.md`, plus a remote configured and
a remote-tracking ref with drift:

```sh
git remote add origin https://example.invalid/repo.git
git config branch.main.remote origin
git config branch.main.merge refs/heads/main
git update-ref refs/remotes/origin/main main~2
git branch stale/old-work main~4
```

Keep a terminal open. Most tickets are "does Spagitty agree with `git branch -vv`".

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1F-01 — The list matches git

- **Priority:** P1
- **Steps:** Compare the table against `git branch -a`.
- **Expected:** Every local branch and every remote-tracking branch appears,
  once. `origin/HEAD` does not — it points at another row. The current branch is
  first, marked with a check.
- **Result:**

### SWEEP-1F-02 — Ahead and behind match git

- **Priority:** P1
- **Steps:** Compare the drift column against `git branch -vv`.
- **Expected:** `↑4` for a branch four ahead; `↓n` for behind; both when both;
  "level" when neither. A branch with no upstream shows `—`, not `0`.
- **Result:**

### SWEEP-1F-03 — The counts say how old they are

- **Priority:** P2
- **Steps:** Hover the drift cell; read the footer.
- **Expected:** The tooltip names the upstream and says "as of the last fetch".
  The footer says nothing on the screen talks to a network. A number that looks
  live but is not is worse than no number.
- **Result:**

### SWEEP-1F-04 — An upstream configured while Spagitty is open

- **Priority:** P1
- **Steps:**
  1. With the screen open, run
     `git branch --set-upstream-to=origin/main some-branch` in the terminal.
  2. Press **Refresh**.
- **Expected:** The upstream and its counts appear. (This failed before the
  branches command started re-opening the repository: gix reads config once,
  when a repository is opened.)
- **Result:**

### SWEEP-1F-05 — Merged branches read as spent

- **Priority:** P2
- **Steps:** Compare the dashed rows against `git branch --merged`.
- **Expected:** They agree. The current branch is never drawn dashed, even
  though it is trivially merged into itself.
- **Result:**

### SWEEP-1F-06 — Filters

- **Priority:** P2
- **Steps:** Try each chip alone, then two together, then with text typed.
- **Expected:** `mine` shows local branches only, `origin` remote-tracking only,
  `upstream` those with one configured, `merged` the dashed ones, `stale` those
  untouched for ninety days. Chips compose as AND. The text box matches the
  branch name and the upstream. The hidden count is right, and **clear** resets
  both text and chips.
- **Result:**

### SWEEP-1F-07 — "mine" says what it means

- **Priority:** P3
- **Steps:** Hover the `mine` chip.
- **Expected:** It explains that it means local branches, not "authored by me" —
  which Spagitty cannot know yet.
- **Result:**

### SWEEP-1F-08 — Checking out

- **Priority:** P1
- **Preconditions:** A clean working copy.
- **Steps:** Press **Check out** on another branch, then check the terminal.
- **Expected:** `git rev-parse --abbrev-ref HEAD` is the new branch. The title
  bar, the toolbar picker, the graph and the rail counts all follow. The current
  row moves.
- **Result:**

### SWEEP-1F-09 — A checkout that would lose work is refused

- **Priority:** P1
- **Preconditions:** Uncommitted edits to a file that differs between two
  branches.
- **Steps:** Try to check the other branch out.
- **Expected:** git's own message in the footer, `HEAD` unmoved, and the file on
  disk untouched. Nothing on this screen may discard work; if this fails, stop
  and report it.
- **Result:**

### SWEEP-1F-10 — Creating a branch

- **Priority:** P1
- **Steps:**
  1. Type a name, leave the start point empty, turn **check it out** off, press
     **Create**.
  2. Check `git branch -v` and `git rev-parse --abbrev-ref HEAD`.
  3. Repeat with **check it out** on.
- **Expected:** The branch is created at `HEAD` and appears in the list. With
  the chip off, `HEAD` does not move; with it on, it does. The form clears on
  success.
- **Result:**

### SWEEP-1F-11 — Creating from a chosen start point

- **Priority:** P2
- **Steps:** Type a name and a start point — a SHA, a tag, another branch — and
  create.
- **Expected:** The new branch's tip is that commit.
- **Result:**

### SWEEP-1F-12 — Bad names are refused with a reason

- **Priority:** P1
- **Steps:** Try a name that already exists, then one with a space in it.
- **Expected:** git's own message in the footer, the form keeps what you typed,
  and the existing branch is untouched.
- **Result:**

### SWEEP-1F-13 — Branch from a remote-tracking row

- **Priority:** P2
- **Steps:** Press **Branch from it** on an `origin/…` row.
- **Expected:** It does *not* check anything out — that would detach `HEAD`. The
  form is filled with the name minus the remote, and the start point set to the
  remote-tracking ref. Pressing Create makes a local branch there.
- **Result:**

### SWEEP-1F-14 — Deleting says it is not built

- **Priority:** P2
- **Steps:** Point at a **Delete** control.
- **Expected:** It says deleting branches is not built yet, and does nothing
  when clicked. It reads as a label rather than a live button.
- **Result:** Obsolete, 2026-10-19 — FEAT-013 built deleting and renaming branches; its sweep checks them.

### SWEEP-1F-15 — Empty and narrow states

- **Priority:** P2
- **Steps:** Open a freshly initialised repository with no commits. Then, in the
  fixture, type a filter that matches nothing.
- **Expected:** The first says the repository has no branches yet; the second
  says no branch matches those filters. They are different sentences.
- **Result:**

### SWEEP-1F-16 — Both themes

- **Priority:** P2
- **Steps:** Toggle the theme with merged, current and ordinary rows on screen.
- **Expected:** All three stay tellable apart, and the drift arrows stay legible.
- **Result:**

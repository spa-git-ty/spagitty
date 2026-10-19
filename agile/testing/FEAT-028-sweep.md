<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-028 — Manual sweep

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-028-01 — Commit is where committing happens

- **Priority:** P1
- **Steps:** Stage a file. Look at the toolbar, then go to Working copy.
- **Expected:** Nothing about committing in the toolbar; the Working copy screen
  has the message box, the staged count and its Commit button.
- **Result:**

### SWEEP-028-02 — Fetch and Push do the thing

- **Priority:** P1
- **Steps:** On a repository with a remote, press Fetch. Then commit something
  and press Push.
- **Expected:** Each runs and reports — the same message as running it from the
  command palette — and with "Show the git command behind each action" on, the
  Commands panel shows `git fetch --prune --progress --all` and
  `git push --progress`.
- **Result:**

### SWEEP-028-03 — Grouping and centring

- **Priority:** P2
- **Steps:** Look at the row on a wide window, then narrow the window until the
  columns crowd.
- **Expected:** Three groups with dividers, centred while there is room; as the
  window narrows they stop being centred and flow rather than overlapping the
  pickers.
- **Result:**

### SWEEP-028-04 — What is still unbuilt still says so

- **Priority:** P3
- **Steps:** Hover Undo and Redo.
- **Expected:** "Not built yet" on both. Any other button carrying that tooltip
  while it works is the bug this item fixed, returning.
- **Result:** Obsolete, 2026-10-19 — BUG-030 took Undo and Redo off the toolbar.

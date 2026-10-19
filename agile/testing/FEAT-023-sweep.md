<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-023 — Manual sweep

Test tickets for the graph's nodes, lanes and column.

**What this is.** Every ordinary commit's node is now a portrait generated from
the author's email — locally, with no network. Merges are plain dots. Lanes are
thicker with longer elbows, the lane column has its own background, and hovering
no longer dims anything.

**What these tickets are for.** The tests assert that a face is deterministic
and that the geometry is what the constants say. Whether the graph *reads* — at
a glance, on a real repository, in every palette — is what a person has to
answer.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-023-01 — Faces, and the same face twice

- **Priority:** P1
- **Preconditions:** A repository with several regular contributors.
- **Steps:** Open the Graph. Find two commits by the same person, far apart in
  the list. Compare their nodes, and compare each against that row's Author
  column.
- **Expected:** Identical faces in all four places. One person, one face.
- **Result:**

### SWEEP-023-02 — Different people look different

- **Priority:** P1
- **Steps:** Scan a stretch of history with three or more authors.
- **Expected:** The faces are tellable apart at a glance without reading the
  Author column — which is the entire point of putting them there. Two authors
  you cannot tell apart is a finding worth recording, with both addresses.
- **Result:**

### SWEEP-023-03 — Nothing is fetched

- **Priority:** P1
- **Steps:** Disconnect the machine from the network entirely, then open a
  repository you have not opened before in this session.
- **Expected:** Every face renders exactly as it does online, immediately. No
  placeholder, no pop-in, no gap. If you have a proxy log, it shows no request.
- **Result:**

### SWEEP-023-04 — Merges are dots

- **Priority:** P1
- **Steps:** Find a merge commit — one with two parents, where two lanes join.
- **Expected:** A small filled dot in the lane colour, no face. Ordinary commits
  around it keep theirs.
- **Result:**

### SWEEP-023-05 — The column reads as a surface

- **Priority:** P2
- **Steps:** Look at the lane column against the rows either side of it.
- **Expected:** A distinct background with a hairline down each edge, running
  the full height, aligned with the rows as you scroll. No seam between one
  row's slice and the next, and no lag behind the rows while scrolling fast.
- **Result:**

### SWEEP-023-06 — Lanes and crossings

- **Priority:** P2
- **Steps:** Find a place where a branch leaves and later rejoins.
- **Expected:** Lines thick enough to follow across the column, each crossing a
  smooth S that leaves and arrives vertically. No kinks, no line passing through
  the middle of a face, no two adjacent lanes reading as one band.
- **Result:**

### SWEEP-023-07 — Hover does nothing dramatic

- **Priority:** P1
- **Steps:** Move the pointer across branch labels and rows, quickly and then
  slowly.
- **Expected:** An ordinary row highlight and nothing else. No commits dimming,
  no dashed line appearing, no flicker as the pointer crosses labels.
- **Result:**

### SWEEP-023-08 — The author filter still dims

- **Priority:** P1
- **Steps:** Type a name into the Author column's filter.
- **Expected:** Rows that do not match dim, and their lanes fade with them.
  Clearing it restores everything. This is the one dimming that survives.
- **Result:**

### SWEEP-023-09 — Scrolling stays smooth

- **Priority:** P1
- **Steps:** On a repository with thousands of commits and many authors, hold
  the scrollbar and drag the full height, twice.
- **Expected:** No stutter, no faces appearing a frame late, no growth in memory
  over the run. The portraits are cached; this is the ticket that would catch a
  cache that is not working.
- **Result:**

### SWEEP-023-10 — Every palette

- **Priority:** P2
- **Steps:** Settings → Appearance, in Light and then in Dark,
  returning to the Graph each time.
- **Expected:** Faces repaint in the new palette immediately — no stale colours
  from the previous theme. The column's fill sits a clear step away from the row
  background in each. Every lane colour is still tellable from its neighbours.
- **Result:**

### SWEEP-023-11 — Zoom

- **Priority:** P2
- **Steps:** `Ctrl +` and `Ctrl -` several steps each way, watching the graph.
- **Expected:** Heads, lanes and rows scale together. Faces stay sharp rather
  than blurring — they are re-rendered at the zoomed size, not stretched.
- **Result:**

### SWEEP-023-12 — A commit with no author email

- **Priority:** P3
- **Preconditions:** A repository containing a commit whose author signature has
  no address (`git commit --author="Nobody <>"` on a scratch clone).
- **Expected:** A face generated from the name instead. Not a blank disc, not an
  error.
- **Result:**

### SWEEP-023-13 — The stash overlay

- **Priority:** P3
- **Steps:** Create a stash, return to the Graph.
- **Expected:** The diamond still sits beside the commit it was made on, joined
  by a short stub, scaled to look right beside the larger head.
- **Result:**

### SWEEP-023-14 — Width

- **Priority:** P3
- **Steps:** Open a repository with five or more parallel lanes on a narrow
  window.
- **Expected:** The lane column is wider than before — that is intended — and
  the message column is still readable. Record the window width at which
  messages become too narrow to use.
- **Result:**

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-002 — Manual sweep

Test tickets for the primary button's fill.

**What this is.** Primary buttons carry a travelling glow: an accent fill with a
bright head running around the border. A scoped rule in `Btn.svelte` was
overriding it, so those buttons had no fill and their labels were `--on-accent`
text on the page background.

**What these tickets are for.** The regression test asserts the rule; only a
person can confirm the button *looks* like a button, in every palette, in every
state.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-002-01 — The Commit button is a button

- **Priority:** P1
- **Preconditions:** App running, light theme, any repository.
- **Steps:** Look at the right of the toolbar.
- **Expected:** A filled accent pill reading "Commit", white on blue, with a
  bright point travelling around its edge. Not a floating word.
- **Result:**

### SWEEP-002-02 — Every other button still has its border

- **Priority:** P1
- **Steps:** Look at the ordinary buttons — Open repository, the Settings
  fields, the Clone modal's Cancel, the Commands drawer's Copy all.
- **Expected:** Each keeps its 1.5px outline and transparent fill, and turns
  accent-coloured on hover. The fix must not have taken the border off these.
- **Result:**

### SWEEP-002-03 — Light and dark

- **Priority:** P1
- **Steps:** Settings → Appearance. Switch between Light and Dark, checking the
  Commit button each time. (There were eight palettes; TASK-051 left one family.)
- **Expected:** The label is readable on the fill in both. Any palette where it
  disappears again is the same bug in a different disguise.
- **Result:**

### SWEEP-002-04 — Hover and disabled

- **Priority:** P2
- **Steps:** Hover the Commit button. Then find a disabled primary — the Clone
  modal's confirm before a URL is entered.
- **Expected:** Hovering dims slightly and does *not* paint a hard border over
  the ring. The disabled one keeps a flat accent fill, no animation, and a
  visible label at reduced opacity.
- **Result:**

### SWEEP-002-05 — The quiet primary

- **Priority:** P2
- **Steps:** Trigger a destructive confirmation — right-click a commit, choose
  Reset (hard).
- **Expected:** The dialog's confirm is filled accent but does **not** glow;
  the glow marks the thing to do next, and on that dialog it is not.
- **Result:**

### SWEEP-002-06 — Reduced motion

- **Priority:** P3
- **Steps:** Turn on the system's "reduce motion" setting and restart the app.
- **Expected:** The button keeps its accent fill and a static ring. No
  travelling head, no breathing halo, and above all not an invisible label.
- **Result:**

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-021 — Manual sweep

> **Refreshed 2026-10-19 (TASK-073).** TASK-051 (2026-09-30) folded the four families into one, Pomodoro, in light and dark. Tickets that need several families are obsolete; the rest are checked in Light and Dark.

Test tickets for the themes and the title bar.

**What this is.** Four palette families — Catppuccin, Dracula, Tokyo Night,
Gruvbox — each with a light and a dark variant, chosen in Settings →
Appearance. The title bar's theme toggle and its `⌘K` chip are gone.

**What these tickets are for.** Contrast is measured by `themes.test.ts` in all
eight palettes, so nothing below needs to check whether text is readable in
principle. What a test cannot answer is whether a palette looks *right* — and
whether every surface actually follows it, which is where a hard-coded colour
would show up.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1M-01 — The title bar lost both chips

- **Priority:** P1
- **Steps:** Look at the title bar.
- **Expected:** Repository name, branch, licence and version, window buttons.
  No `⌘K`, no `dark`/`light` chip. Acceptance criterion 8.
- **Result:** Obsolete, 2026-10-19 — FEAT-043 moved the repository and branch out of the title bar and FEAT-082 redrew it; the chips are still gone.

### SWEEP-1M-02 — Every family, both modes

- **Priority:** P1
- **Steps:** Settings → Appearance. Choose each of the four families in Light,
  then switch to Dark and go through all four again.
- **Expected:** Eight distinct looks, applied the moment they are chosen with no
  reload. The chosen family is marked, and the variant name beside "Theme"
  follows — Latte, Mocha, Alucard, Dracula, Day, Night, Light, Dark.
  Acceptance criteria 1 and 2.
- **Result:** Obsolete, 2026-10-19 — TASK-051 left one theme family, Pomodoro.

### SWEEP-1M-03 — The graph follows the theme

- **Priority:** P1
- **Preconditions:** A repository open with a branchy history, on the Graph
  screen with lanes visible.
- **Steps:** Open Settings → Appearance in a way you can get back from, switch
  family, and return to the Graph. Do it for all four.
- **Expected:** The lane colours change with the theme. This is the one surface
  that reads colours through JavaScript rather than CSS, so it is the one that
  can be left behind. Acceptance criterion 2.
- **Result:**

### SWEEP-1M-04 — Every screen follows the theme

- **Priority:** P1
- **Steps:** Choose Dark and visit
  every screen on the rail plus the Clone modal.
- **Expected:** No surface still showing the old white or grey — no panel, no
  border, no button, no empty state, no scrollbar area. Anything that does not
  change is a colour hard-coded outside the token system, and worth reporting
  with the exact element.
- **Result:**

### SWEEP-1M-05 — The swatches are the theme they preview

- **Priority:** P2
- **Steps:** In Light, look at the four swatch rows. Switch to Dark and look
  again.
- **Expected:** Each family's swatches are its colours *in the mode currently
  on* — dark swatches in dark mode. Choosing that family produces what the
  swatch showed.
- **Result:** Obsolete, 2026-10-19 — TASK-051 left one theme family, Pomodoro.

### SWEEP-1M-06 — The choice survives a restart

- **Priority:** P1
- **Steps:** Choose Dark. Quit Spagitty completely, start it again.
- **Expected:** Dark, from the first frame. Acceptance criterion 3.
- **Result:**

### SWEEP-1M-07 — An old installation keeps its light or dark

- **Priority:** P1
- **Steps:** Quit. In the webview's local storage, remove `spagitty.theme.family`
  and set `spagitty.theme` to `dark` — the state an installation from before this
  change is in. Start Spagitty.
- **Expected:** Dark, in Catppuccin. Nobody loses their setting to an upgrade.
  Acceptance criterion 3.
- **Result:** Obsolete, 2026-10-19 — TASK-051 left one theme family, Pomodoro.

### SWEEP-1M-08 — The first frame is not a flash of white

- **Priority:** P2
- **Steps:** With Dark chosen, quit and start Spagitty several times,
  watching the very first frame.
- **Expected:** It comes up in a theme, not in white and then a theme. The boot
  values in `src/app.css` are the default family, so the first frame is Latte or
  Mocha even before the store runs. Acceptance criterion 7.
- **Result:**

### SWEEP-1M-09 — A hand-edited theme setting does not break anything

- **Priority:** P2
- **Steps:** Quit. Set `spagitty.theme.family` to `solarized` and `spagitty.theme`
  to `chartreuse`. Start Spagitty.
- **Expected:** It starts, in Pomodoro, light or dark by the system
  preference. No window with no colours in it. Acceptance criterion 4.
- **Result:**

### SWEEP-1M-10 — The look, which is what a test cannot check

- **Priority:** P2
- **Steps:** Spend a few minutes working in each theme — stage something, read a
  diff, scroll the graph.
- **Expected:** Judgement. Is the accent doing too much work? Are selected rows
  distinguishable from striped ones? Does the dashed "nothing to do here"
  treatment still read as dashed? Report anything that looks wrong even though
  it passes a contrast ratio.
- **Result:**

### SWEEP-1M-11 — Light mode in a bright room, dark mode in a dark one

- **Priority:** P3
- **Steps:** Exactly that, for each family.
- **Expected:** No palette that is only usable in the lighting it was designed
  in. This is the one thing a monitor in a dark room hides from whoever wrote
  the palette.
- **Result:**

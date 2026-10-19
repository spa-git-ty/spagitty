<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-020 — Manual sweep

**Item:** [`agile/items/TASK-020-the-glass-material-settled.md`](../items/TASK-020-the-glass-material-settled.md)

**Preconditions for every ticket:** a build of the branch, and a repository open
with enough history to fill the graph.

---

## TASK-020-T1 — The values that were chosen are the values that shipped

**Priority:** high — the whole item is a transcription, and transcriptions go wrong.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open a menu over the commit list | Frost present and light — it suggests what is behind rather than hiding it. |
| 2 | Look at the rim for a colour split | None. `chromaticAberration` is 0. |
| 3 | Look at the colour of the backdrop through the pane | Grey. `saturate` is 0, so no tint of the commits comes through. |
| 4 | Look at the corner of any menu, dialog or card | 8px — the same corner a text field has, not the rounder 14px card corner. |

**Result:**

---

## TASK-020-T2 — The corner moved everywhere it should

**Priority:** medium — `--r-panel` is a surface token, not a glass one.

| # | Step | Expected |
| --- | --- | --- |
| 1 | A menu | 8px corner. |
| 2 | A confirmation dialog | 8px. |
| 3 | A card on any screen — the empty states, the Settings sections | 8px. |
| 4 | The window's own corner | **Unchanged.** That is `--r-window`, a different token, and it should still be the larger radius. |
| 5 | A button and a chip | Unchanged — `--r-button` and `--r-pill` were not touched. |

**Result:**

---

## TASK-020-T3 — At another text size and zoom

**Priority:** medium — the radii are published through the metrics and scale with zoom.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Ctrl `=` a few times to zoom the interface in | Corners grow in proportion; nothing becomes square or fully round. |
| 2 | Ctrl `-` back down | Same in reverse. |
| 3 | Ctrl `0` to reset | Back to 8px. |
| 4 | Change the text size in Settings → Appearance | Corners do not follow the text size — only the zoom, which is deliberate. |

**Result:**

---

## TASK-020-T4 — Across themes

**Priority:** medium — the material is one set of numbers for every palette.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open a menu in a dark theme | Readable, frosted, edge visible. |
| 2 | Switch to a light theme and repeat | Same judgement. A frost tuned in the dark can vanish in the light — say so if it does. |
| 3 | ~~Try two more palettes~~ — one family since TASK-051; nothing more to do | — |

**Result:**

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-039 — Manual sweep

**Item:** [`agile/items/TASK-039-one-colour-system-in-every-dialog.md`](../items/TASK-039-one-colour-system-in-every-dialog.md)

Every ticket here is run in the real Tauri window, not a browser preview: the
dialogs use `backdrop-filter` over a transparent, undecorated window, and that
composite is the thing being judged.

**Run each ticket at least twice — once on a light family and once on a dark
one.** The defect was invisible on Catppuccin Mocha, which is the family the
literals were copied from, and obvious on every light theme. A sweep run only on
the default family would confirm nothing.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A repository with at least two worktrees, one locked | Settings → theme = Catppuccin **Latte**. Open the worktrees manager | The dialog is a light dialog: dark text on a light glass panel, cards a step above the panel, badges as quiet ink washes. Nothing is `#eee` on `#141416`. The locked pill uses the family's own amber and the prunable pill its own red — not `#ffc107` and `#f44336`. | High | |
| SWEEP-002 | The same, on **Nord Snow Storm** and **Solarized Light** | Repeat | Each family paints its own accent on the main-worktree pill. The brand amber appears in none of them. This is the assertion the old code failed most visibly: one hard-coded amber across eight palettes. | High | |
| SWEEP-003 | Any light family | Open each of: Add worktree, Submodules, Create pull request, Settings → Profiles | The scrim behind each recedes without going near-black. Compare against a confirmation dialog from `DialogHost` opened over the same screen — the two backdrops should read as the same material. Before this change the four modals were visibly darker. | High | |
| SWEEP-004 | A commit touching a PNG with transparency, and a binary file | Open the image diff, then the binary diff | The checkerboard is a pair of neutrals drawn from the current theme, and on a light family it is *light*. The divider handle's label is legible against the accent fill in every family — check a pale accent and a dark one. | High | |
| SWEEP-005 | Any theme | Settings → text size 130%. Reopen every dialog above | All of the type grows with it. Before this change these eleven components were the only things on screen that did not. Check especially the Profiles key pills and the Worktrees path line, which were the smallest. | High | |
| SWEEP-006 | Text size 130%, window narrowed to ~900px | Reopen the worktrees manager and Create pull request | Nothing is clipped and no row overlaps. The 640px modals are `max-width: 90vw`, so this is the case where the type growth and the fixed width meet. | High | |
| SWEEP-007 | File history on a file with several authors | Open it and hover a blame row | The row highlight is the family's accent tint, not amber. The blame gutter is a well in the theme's own colours rather than a black wash — on a light family it should be a *lighter* recess, which is the reverse of what it was. | Medium | |
| SWEEP-008 | Submodules present | Open the submodules dialog with at least one out-of-date entry | The three status tints are the palette's `--ok`, `--warn` and `--danger`. Take a screenshot beside the same three colours as used on the Working copy screen: they must be the same colours, which was the point of adding the semantic tokens in the first place. | Medium | |
| SWEEP-009 | An imported palette, if one is configured | Open two of the dialogs | They follow it. An imported palette is the case that made this most visible, because it shares no colours with the fallbacks. | Medium | |
| SWEEP-010 | Any dialog with a destructive action — remove worktree | Tab to the buttons | Keyboard focus is visible on every control in the dialog, on both a light and a dark family. Nothing here changed focus handling, and the ticket exists because a colour pass is exactly when a focus ring quietly stops being visible. | High | |

## Negative paths this sweep deliberately covers

- **SWEEP-002 and SWEEP-009** are the tickets a light-theme-only or
  default-theme-only sweep would miss, and missing them is how the defect
  survived eleven components and five features.
- **SWEEP-006** is the one that can fail because of this change rather than
  despite it: type grew inside fixed-width dialogs.
- **SWEEP-010** checks something not changed, on the reasoning that a
  wholesale colour edit is the likeliest way to lose a focus ring without
  touching a line of focus code.

## Contrast

Measure the **composited** result, not the palette entry: these chips are
translucent washes over glass over the window. Ordinary readable text needs
4.5:1 and an essential non-text state indicator 3:1. The two places worth
measuring first are `--muted` labels on `--surface` cards, and a `--soft` chip's
text on the glass panel, in each family's light variant.

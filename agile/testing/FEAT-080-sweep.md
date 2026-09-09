<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-080 — Manual sweep

**Item:** [`agile/items/FEAT-080-follow-omarchy.md`](../items/FEAT-080-follow-omarchy.md)

Run in the real Tauri window on a machine running Omarchy, and — for the last
two tickets — on a machine that is not.

**Change the theme with `omarchy-theme-set`, not by editing `colors.toml`.**
Editing the file in place exercises a code path that does not happen in life and
skips the one that does: the script builds a `next-theme` directory and replaces
`current/theme`, which is what breaks every implementation that watches the
palette file. A sweep that edits the file by hand would pass while the feature
is broken.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Omarchy, any theme | Open Settings → Appearance | A **Desktop** row with a **Follow Omarchy** chip, and the desktop's own theme name beside it — `sushi-dark-palette`, not "Mocha". | High | |
| SWEEP-002 | The same | Press Follow Omarchy | The whole window repaints in the desktop's colours immediately. The Theme row says the desktop's name. No family swatch is marked active any more. | High | |
| SWEEP-003 | Following, on the Graph screen with a busy history | Look at the graph | Lane colours are the desktop's hues, and the five lanes are **five different colours**. Author portraits are redrawn in the new palette rather than left over from the old one — this is what the palette revision is for, and it is the thing `theme.id` could not see. | High | |
| SWEEP-004 | Following, window open, Graph on screen | Run `omarchy-theme-set` for a **different dark** theme | The window repaints within a second, DOM and canvas together, with **no intermediate frame** of Catppuccin. Two dark themes in a row is the case that matters: the mode does not change, so anything keyed on light/dark would see nothing. | High | |
| SWEEP-005 | The same | Now switch to a **light** Omarchy theme | It repaints, and `data-theme` follows: dialogs, the diff gutter and the status strip are all light. Nothing is left dark. | High | |
| SWEEP-006 | Following, on a light desktop theme | Read a commit list, a diff and the Farm screen | Secondary text — timestamps, paths, hashes — is comfortably readable. This is the role that fails at 2.75:1 when copied literally, and a light palette is where a too-subtle derivation shows first. | High | |
| SWEEP-007 | Following | Press a family swatch — Nord, say | It switches to Nord and **stops following**. Restart: still Nord. Press Follow Omarchy again: back to the desktop's colours, and that survives a restart too. | High | |
| SWEEP-008 | Following, then quit Spagitty and run `omarchy-theme-set` while it is closed | Launch Spagitty | It opens on the *new* desktop palette. The first frame is the previous session's cached colours (BUG-031) and the corrected one arrives without a visible flash — if there is a jarring change here, note it: this is the seam between the two features. | Medium | |
| SWEEP-009 | An Omarchy theme whose `colors.toml` omits several hues | Follow it | Every screen is fully painted. Missing hues fall back to ANSI slots and then to the default family, never to nothing. Record which theme, because the derivation has been exercised against two palettes and this is a third. | Medium | |
| SWEEP-010 | A Linux machine **without** Omarchy | Open Appearance | There is **no** Desktop row at all. No error, no console warning, no empty control. The eight families behave exactly as before. | High | |
| SWEEP-011 | macOS or Windows | The same | The same. The backend answers without touching a filesystem. | High | |
| SWEEP-012 | Following, then `mv ~/.local/state/omarchy ~/.local/state/omarchy.off` with the window open | Watch the window | The colours **stay**. Omarchy vanishing mid-session must not blank the theme; the last valid palette holds for the session, and Appearance can say why. | Medium | |
| SWEEP-013 | Following, a machine with `fs.inotify.max_user_watches` set very low | Follow Omarchy, then change the desktop theme | The palette that was read still applies; only live updates are lost. Nothing crashes and nothing reports an error the user cannot act on. | Low | |
| SWEEP-014 | Following, Omarchy's tiled layout with the window flush to the screen edges | Look at the window's edges | Unchanged from before this feature: the window policy from BUG-029 still applies, and following a palette must not have added a frame, a border or a shadow. | Medium | |

## Contrast, measured rather than looked at

For at least one dark and one light desktop theme, measure the **composited**
results on screen rather than the palette entries:

- secondary text on the ground and on a raised card — 4.5:1;
- a `--soft` chip's label over the glass panel — 4.5:1;
- the accent as a border on the ground, and each lane on the graph column —
  3:1;
- a filled primary button's label on its accent — 4.5:1.

The derivation guarantees the first, third and fourth against `--bg`. What it
cannot guarantee is a translucent chip over glass over the window, which is why
this is a sweep and not a test.

## Negative paths this sweep deliberately covers

- **SWEEP-004** is the one that separates a working implementation from one
  that watched the file. Two dark themes in a row also defeats anything keyed
  on light/dark.
- **SWEEP-010 and SWEEP-011** are the machines this feature must be invisible
  on, and they are the ones nobody thinks to check.
- **SWEEP-012** is Omarchy going away underneath a running window.
- **SWEEP-014** exists because a colour feature is exactly when a window-chrome
  regression sneaks back in unnoticed.

## What must not be true afterwards

Nothing under `~/.config/omarchy` or `~/.local/state/omarchy` has been written
to. Confirm with `find ~/.local/state/omarchy -newermt '-1 hour'` after a
session of following: Spagitty follows the desktop, it does not edit it.

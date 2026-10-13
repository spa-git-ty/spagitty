<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-134 — Automated test record

**Item:** [FEAT-134](../items/FEAT-134-pin-or-hide-what-is-on-the-rail.md)

| Suite | Cases |
| --- | --- |
| `src/lib/nav.test.ts` | The default is everything pinned and draws the rail as before. **Hiding**: a hidden screen is off the rail and the rest keep their places; the open screen stays, hidden or not; Farm and Settings cannot be hidden by any stored list; Conflicts and Badges keep their rules; every other row offers the choice; an unknown code is ignored. **The More row**: absent with nothing hidden; holds the hidden screens in rail order; sits at the end of Tools, before Settings; starts the group when every tool is hidden; goes with *Palette only*; still lists the hidden screen that is open; never offers Badges with the delight layer off. **Reordering**: a row moves down and back, and the built-in order is stored as an empty list; a move out of the group is refused; a drop onto a row of the same group is taken and onto another group's is not; a scrambled stored order still gives each group one run in its place; a screen the order does not name follows the row it follows in `NAV_ITEMS`; the rail is drawn in the chosen order. |
| `src/lib/chrome/chrome.test.ts` | Mounted `NavRail`: no More button until something is hidden; hidden screens are gone and More is before Settings; More opens a menu of the hidden screens with their codes, goes to the one chosen, and closes when pressed again; a hidden screen is on the rail and marked while it is open; no More with *Palette only*; rows are drawn in the chosen order; as a preview the rail is inert, named as a preview, and never offers to open a repository. |
| `src/lib/settings/rail-section.test.ts` | Mounted `RailSection`: every screen has a row in its group with its code; everything starts pinned with **Reset** disabled; Farm and Settings read *Always* with no choice; Conflicts shows its rule; **Hide** and **Pin** write the choice and the row follows; pressing the choice already made writes nothing; *Palette only* is written without losing what is hidden; Alt+↓ moves a row and Alt+↑ on the first row of a group does not; the arrows without Alt are left alone; **Reset** writes the default; the preview is the rail drawn from the choice, lists what is under More beside it, drops the list and the button with *Palette only*, and follows a change as it is made. |
| `src/lib/settings/store.test.ts` | The rail is the default before anything is stored; it reads back what was stored; a choice is stored with the rest of the settings and shown at once; it is put back when the write fails; a backend that answers without the key gives the default. `/settings#rail` lands on Rail, the chip after Appearance. |
| `src/lib/settings/sections.test.ts` | Unchanged, and now covers Rail: every chip in `SECTIONS` has a branch in `routes/settings/+page.svelte`, and the route names no section the index lacks. |
| `src/lib/palette/commands.test.ts` | Every screen in `NAV_ITEMS` is reached by a *Go to* command. |
| `src-tauri/src/settings.rs` | Every screen is pinned by default; the rail is stored as `"rail":{"hidden":[…],"order":[…],"where":"…"}`; a `rail` that cannot be read, or a place this build does not know, costs only the rail; a code this build does not know stays in the list; the whole of `Settings` reads back as written. |

## Not covered

- **Dragging a row.** `moveOnto` is tested; the drag events on the row are not driven.
- **The file on disk.** The Rust tests parse and serialise text. Writing `settings.json` and reading it after a restart was not run.
- **The menu's place on screen**, which was looked at in headless Chrome and is not asserted.

## Results — 2026-10-13

- `bun run test`: 191 files, 4192 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run coverage`: passed its thresholds.
- `bun run build`: succeeded.
- `cargo test -p spagitty settings` (Windows): 18 passed, 1 ignored.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-134 — Sweep

**Item:** [FEAT-134](../items/FEAT-134-pin-or-hide-what-is-on-the-rail.md)

Rows 01, 02, 03, 05, 06, 08, 09 and 14 were run on 2026-10-13 against the built frontend in headless Chrome at 1440×900, dark and light, with a stand-in for the backend. None has been run in the running application, where the choice is written to `settings.json`.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT134-01 | A new install | Open Settings, press **Rail** | Every screen has a row in Farm, Repository, Tools or Spagitty, each with its icon, label and code. Every choice reads *Pin*. The count says nothing is hidden and **Reset** is disabled | P1 | Passed in headless Chrome. Not run in the application. |
| SWEEP-FEAT134-02 | As above | Press **Hide** on Rebase, Stash, Tags, Reflog and All repositories | Each leaves the rail at once and its row is quieter. A **More** button appears at the end of Tools, above Settings. The preview shows the same rail, with the five listed beside it | P1 | Passed in headless Chrome. Not run in the application. |
| SWEEP-FEAT134-03 | Five screens hidden | Press **More** on the rail | A menu lists the five with their codes. Choosing one goes to it. Pressing **More** again, or Escape, closes the menu | P1 | Passed in headless Chrome. Not run in the application. |
| SWEEP-FEAT134-04 | As above | Tab to **More**, press Enter, move with the arrow keys, press Enter | The menu opens, the arrows move through it, and Enter goes to the screen | P1 | Not run. |
| SWEEP-FEAT134-05 | Stash hidden | Go to Stash from **More** | Stash is on the rail and marked as where you are. Leave it and it is gone from the rail again | P1 | Passed in headless Chrome. Not run in the application. |
| SWEEP-FEAT134-06 | Five screens hidden | Press **Palette only** | **More** leaves the rail and the list leaves the preview. The palette still goes to each hidden screen | P1 | Passed in headless Chrome for the rail and the preview. The palette was checked by its test only. |
| SWEEP-FEAT134-07 | Anything | Look at Farm, Settings, Conflicts and Badges (Badges with Personality on) | Farm and Settings read *Always*. Conflicts reads *When there are any* and Badges *While open*. None offers Pin or Hide | P2 | Not run. Covered by mounted tests. |
| SWEEP-FEAT134-08 | Anything | Focus a row's grip and press Alt+↑ or Alt+↓ | The row moves one place within its group, the focus stays on it, and the rail and the preview follow. The first and last rows of a group do not leave it | P1 | Passed in headless Chrome for Alt+↑. Not run in the application. |
| SWEEP-FEAT134-09 | Screens hidden and reordered, **Palette only** chosen | Press **Reset** | Everything is pinned, in the built-in order, with **Under ⋯ More** chosen | P1 | Passed in headless Chrome. Not run in the application. |
| SWEEP-FEAT134-10 | Anything | Drag a row by its grip onto another row of its group; then onto a row of another group | It takes that row's place. The other group's row does not accept it | P1 | Not run. Dragging was not driven. |
| SWEEP-FEAT134-11 | Screens hidden and reordered | Close Spagitty and open it again | The rail is as it was left | P1 | Not run. Needs the application. |
| SWEEP-FEAT134-12 | A `settings.json` whose `rail` is not an object, or names a screen that does not exist | Open Spagitty | The rail is the default, or ignores the unknown screen. Every other setting is as it was | P2 | Not run. Covered by Rust tests. |
| SWEEP-FEAT134-13 | A window 1024 wide | Open Settings › Rail | The preview goes under the lists. Nothing is cut off | P2 | Not run. |
| SWEEP-FEAT134-14 | Light theme | Open Settings › Rail | The rows, the choices and the preview read as clearly as in dark | P2 | Passed in headless Chrome. Not run in the application. |

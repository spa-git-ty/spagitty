<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-029 — Automated tests

**Item:** [`agile/items/BUG-029-the-window-is-ringed-with-blurred-desktop.md`](../items/BUG-029-the-window-is-ringed-with-blurred-desktop.md)

## What was tested

Two files, eight tests. The defect was one boolean too few in a policy nothing
could ask a question of, so the fix is a pure function and the tests are a table
over it.

| Test | File | Asserts |
| --- | --- | --- |
| draws its own card on the hosts that decorate nothing | `chrome/window.test.ts` | macOS and Windows still get `floating`. This is the half that must not change. |
| squares itself against the screen when maximized | `chrome/window.test.ts` | FEAT-037's original behaviour survives the new branch. |
| draws no card at all on Linux, where the compositor drew one | `chrome/window.test.ts` | The fix. |
| stays flush on Linux when the window is restored | `chrome/window.test.ts` | **The one that would have caught this.** A tiled Hyprland window is not maximized, which is exactly why the existing `maximized` branch did not fire. |
| reads the platform off the agent string, and only whole words | `chrome/window.test.ts` | `\bLinux\b`, so `Linuxish/1.0` in a UA string cannot decide how the window is drawn. |
| falls back to a drawn card when nothing says what the host is | `chrome/window.test.ts` | An empty agent is not a Linux agent. Without this, a test environment with no `navigator` would silently hide the card from every other test. |
| changes transparency and nothing else | `tools/window.test.ts` | The Linux config's window object equals the base one but for `transparent`. Tauri's merge replaces arrays wholesale, so the object is duplicated; this is what stops the copies drifting. |
| is opaque, so the compositor has no margin to blur | `tools/window.test.ts` | The two values, named, in the file that explains why they differ. |
| still draws its own title bar | `tools/window.test.ts` | `decorations: false` on both. Only the edge moved; the traffic lights are still Spagitty's. |

**What would have to break for these to fail.** Somebody adding a fourth
platform state without deciding what Linux does; a UA test loosened to a
substring; the two window objects being edited apart; or `transparent` being put
back on Linux.

## What the tests cannot reach

The bug itself is a compositor compositing. No test in this repository can
observe a blurred band around a window — that needs a Wayland session, a
compositor with blur on, and eyes. `SWEEP-001` and `SWEEP-002` are the real
verification, and they are the reason this item is "awaiting sweep" rather than
"done".

What the tests do cover is every decision that leads to the pixels: which state
each host gets, that the state survives a restore, and that the window really is
created opaque. If those four are right and the band is still there, the cause
is somewhere this fix never claimed to reach.

## Run against the broken state first

`shellState` did not exist, so the four Linux tests were written against
`watchMaximized`'s original boolean first: with `dataset.window` set from
`isMaximized()` alone, a Linux host reports `floating`, and the two tests that
name `flush` fail. `tools/window.test.ts` fails at `readFileSync` with no
`tauri.linux.conf.json` in the tree.

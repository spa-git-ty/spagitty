<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-042 — One control scale, and a window that knows its platform

**Status:** Done.
**Screen:** the shared UI kit, the chrome, and the window itself.
**Raised by:** a UI review: "typography and target sizes change by screen, while
incidental motion makes dense work feel less steady", and "on macOS this does
not match familiar window controls and can feel like a web page inside a custom
frame".

## Problem

**Three defects, and the first two are the same defect twice.**

**Type that could not be resized.** `scale.svelte.ts` implements the text-size
preference by rewriting the `--fs-*` tokens and nothing else. TASK-039 fixed 83
fixed pixel sizes in eleven dialogs; eight remained, in the places most likely
to be looked at — the title bar at 12px and its controls at 11px, the toolbar's
branch value at 12px, a badge's pin at **9px**, the repositories hero at 24px,
and two badge glyphs at 22 and 54. Somebody who raised the text to 130% got a
larger application and these, unchanged. A 9px label is also below anything the
type scale offers and below what anybody should be asked to read.

**Controls that moved while they were being aimed at.** Every button, chip and
rail row lifted a pixel or two on hover. Each is a small thing and the sum is
not: a pointer crossing a dialog's row of buttons, or running down fourteen rail
destinations to reach the eleventh, sets off a shift under itself at every step.
The rail rows slid two pixels *sideways*, which is the worst version — the
target moves along the axis the pointer is travelling.

**A macOS window that is not a macOS window.** `tauri.conf.json` sets
`decorations: false` and `transparent: true` for every platform, and
`TitleBar.svelte` draws the same three neutral glyph buttons on the right
everywhere. On macOS that is not what a window looks like: the controls are
traffic lights, they are on the left, and they are the system's. `ResizeEdges`
then laid eight invisible resize regions over a frame that was not there — which
on a decorated window would swallow the pointer just inside a border that
already works.

## Change

- **Every remaining pixel type size is a token or a multiple of one**, and a new
  assertion in `flat.test.ts` forbids `font-size: Npx` in any component. `em`
  and `calc(var(--fs-title) * n)` both pass, because both follow the scale; it
  is the absolute unit that does not.
- **Hover changes colour; press keeps its motion.** That is the line, and it is
  the one the review asked for: motion for something somebody did, not for where
  their pointer happens to be. A key going down is a state change; a hover is a
  position. Applied to `Btn`, `Chip` and the rail's rows — the controls that
  repeat down a list — and asserted for the shared kit and the chrome.
  Repository cards and badges still lift, because a single object on a browsing
  surface is not a target in a row and that affordance is doing work.
- **`src-tauri/tauri.macos.conf.json`** (new) gives macOS real decorations with
  `titleBarStyle: "Overlay"` and `hiddenTitle: true`. The system draws its own
  traffic lights *over* Spagitty's bar, so the workspace bar survives and only
  the controls change — plain decorations would have put a second title bar
  above the application's. Everything else about the window is identical to the
  base config, and a test checks that it is.
- **The title bar draws no controls on macOS** and reserves 78px on its left for
  the system's. The outer columns of its grid are equal, so the name stays
  centred in the window rather than in what is left.
- **`ResizeEdges` renders nothing on macOS**, where the system supplies its own.
- **`decoratesItself` is now "not Linux and not macOS"**, so the window draws no
  card on either — the same second-card argument BUG-029 made about Hyprland's
  border, applied to a real macOS frame where it would be more obviously wrong.
  An unrecognised host still gets the card, which is the honest picture for
  something that may have no frame at all.

## What is not verified

**None of the macOS half has run on a Mac.** The configuration is what Tauri
documents, the merge is a platform-specific config file Tauri supports, and the
frontend side is tested. Whether the traffic lights land where 78px expects,
whether the drag region still moves the window, and what happens in full screen
are `SWEEP-005` to `SWEEP-009`, and they are unrun.

## Acceptance criteria

- No component stylesheet names a pixel type size.
- Raising the text size resizes the title bar, the toolbar's branch value, the
  repositories hero and the badges.
- No button, chip or rail row moves on hover; all of them still answer a press.
- On macOS the window has system traffic lights at the top left, no Spagitty
  controls, no drawn card, and no custom resize edges.
- Linux and Windows are unchanged.

## Non-scope

- **`platform.rs`.** The review asks for the Linux renderer and the disabled
  accessibility bridge to be profiled on representative GPUs and for an
  accessible path to be investigated. Neither is possible here: there is one
  machine, and removing a known stability fix on appearance grounds is exactly
  what the review says not to do. Recorded as TASK-043 rather than guessed at.
- **A macOS menu bar.** A Mac application is expected to have one, and Spagitty
  has none on any platform. That is a feature, not a chrome fix.

## Dependencies

BUG-030 built `src/lib/platform.ts`, which is how both components know where
they are. BUG-029 is the argument this extends to a second platform. TASK-039
tokenized the first 83 sizes.

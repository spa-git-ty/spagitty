<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-090 — Settings › Reading

**Status:** Open — built on `feature/FEAT-090-settings-reading`, not yet merged.
**Branch:** `feature/FEAT-090-settings-reading`
**Screens:** 1K (Settings › Reading); 1B, 1C, 1O and 1R read what it sets.
**Raised by:** the author, 2026-10-04, as slice 7 of the Review handoff
(`design_handoff_review/Settings Reading.dc.html`): reading code and review
comments as a dyslexic reviewer.

## Problem

Code was set one way everywhere: the desktop's monospace, 14.4px, tight, with
every changed row a block of red or green. There was nothing to choose but the
text size.

## Change

- **Settings › Reading**, built like Appearance — labelled chip rows and
  sliders — with a live preview drawn by the same tokens it changes:
  - **Code font**: Atkinson Hyperlegible Mono, OpenDyslexic Mono, Lexend,
    JetBrains Mono, System monospace — each chip set in its own face, with one
    line on what it is for.
  - **Interface font**: System, Atkinson Hyperlegible, Lexend.
  - **Code size**, **Line spacing**, **Letter spacing**.
  - **Focus ruler**: Off, One line, Whole chunk (the Review room uses it).
  - **Diff colours**: Calm or Classic, and Highlight changed words.
- **Published as tokens** — `--code-font`, `--fs-code`, `--code-lh`,
  `--code-ls`, `--font-ui`, `--diff-add-bg`, `--diff-del-bg`, `--diff-add-hl`,
  `--diff-del-hl`, `--diff-del-ink`, `--diff-marker`, `--ruler` — and read by
  Diff, Working copy, File history and Review. The code size composes with
  zoom and text size.
- **Calm colours** (the default): a thin marker down the side and a faint
  tint, so no row is a solid block; removed lines go quiet. Classic is the
  full rows.
- **Changed words**: within each run of removed and added lines, each removed
  line is compared with the added line it most resembles, and only the words
  that differ are marked — one contiguous mark preferred over scattered ones,
  nothing marked when a line was rewritten rather than edited.
- **The defaults are the design's reading set** for code — Atkinson
  Hyperlegible Mono, 15px, 1.85 line spacing, 0.02em — with the interface left
  on the desktop's face until chosen.
- **The faces are bundled** under `assets/fonts/`, never fetched: the
  application works offline. All are SIL OFL 1.1 except OpenDyslexic Mono,
  whose only monospaced cut is under the Bitstream Vera licence (permissive,
  GPL-compatible). NOTICE records each, and now also Sora, which it had said
  was not shipped.

## Dependencies

Five font files, about 530 KB, added because the handoff asks for these faces
and the application must not reach for a font service.

## Non-scope

- The Review room's use of the ruler and `Aa`: the room's item.
- The Pull requests screen's own diff pane, which the handoff leaves as it is.

## Acceptance criteria

- Reading settings change Diff, Working copy, History and Review together.
- With Calm colours on, no diff row is a solid red or green block.
- Every offered face renders with no network.

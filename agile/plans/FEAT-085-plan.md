<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-085 — Plan

**Item:** [`agile/items/FEAT-085-a-new-mark-palette-and-wordmark.md`](../items/FEAT-085-a-new-mark-palette-and-wordmark.md)

## Approach

**Concepts first.** Three drawings of the pasta (Twirl, Untangle, Bucatini S),
three palettes (Pomodoro, Basilico, Nero), two wordmarks (Fraunces, Sora) and a
theme mock were put on one canvas for the author to choose from. Nothing in the
tree changed until they chose.

**One source, a small subset of SVG.** The mark is written so the Pillow
renderer can read it with regular expressions: one `rect`, `path`s with
`data-part="strand"` or `"gap"`, `circle`s, absolute `M`/`C`/`L`. The renderer
draws in document order; a gap paints the plate's colour on the full mark and
erases on the plate-less marks, so the strands still cross on a tray. Strokes
are polylines (48 segments a curve) with a disc at every vertex: Pillow draws a
wide line segment by segment, and the slivers between segments showed as
hairlines after the downscale.

**Deterministic bytes.** Generated in WSL with the Pillow pip installs (12.3.0),
the same wheels CI's gate 2 installs, and checked with both `--check` modes.
Small sizes are rendered at their size (supersampled 8×), not shrunk from
512 px.

**The name in the app.** A `--brand` token, a `Wordmark` component, and Sora
through an `@font-face` pointing at the bundled file, named by the wordmark
alone.

## Files

| File | Change |
| --- | --- |
| `assets/brand/mark.svg`, `src-tauri/icons/mark.svg` | The new mark. |
| `tools/make-icons.py` | Renders the subset: strokes, gaps, dots. |
| `tools/make-brand.py` | Sora, the git letters, new inks, new preview page. |
| `assets/brand/font/` | Sora and its licence in; Inter out. |
| `src-tauri/icons/*`, `assets/brand/**` | Regenerated. |
| `src/app.css` | `--brand`; Sora's `@font-face`. |
| `src/lib/ui/Wordmark.svelte`, `BrandMark.svelte` | The name; a square mark. |
| `src/lib/chrome/TitleBar.svelte`, `src/routes/repos/+page.svelte` | Use them. |
| `docs/branding.md`, `README.md` | The new identity. |
| tests | Title row, brand token and face. |

## Risks and rollback

- **Pillow versions.** A different Pillow could round a pixel differently and
  fail gate 2. The files were generated with the version pip installs today.
- Rollback is a revert, which brings the old mark and Inter back.

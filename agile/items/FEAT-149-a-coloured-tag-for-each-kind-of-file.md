<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-149 — A coloured tag for each kind of file

**Status:** Done — on `release/3.3.0`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Screens:** 1U (Files)
**Raised by:** the author, 2026-10-19: "in files view put simple colored icons for common file types or glyphs".

## Change

Files' tree was one column of grey names. Each file gains a small tag in front
of its name — `TS`, `RS`, `{}`, `MD`, `SV` — on a tint of its own hue, and each
folder a folder, open or shut.

## Acceptance criteria

- Common languages, data formats, images, fonts, archives and the files known by
  their whole name (`Dockerfile`, `LICENSE`, `Cargo.lock`, `.gitignore`, `.env.*`)
  have a tag. Anything else has a plain grey sheet.
- Tags are letters, not logos: no icon font and no borrowed brand marks. Git
  files are grey, never Git orange.
- The tag's colour is a hue only; its strength is mixed into the theme's ink and
  ground, so it reads the same in light and dark.
- The tag is drawn, not written: a row's text, and what a reader or a copy
  takes from it, is only the name.
- The tree and its search results both show it.

## What was built

- `files/kind.ts`: `fileKind(path)`, by whole name first, then extension.
- `files/FileGlyph.svelte`, used by `FileTree.svelte`.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-049 — Small monospace text ignores the reading font

**Status:** Fixed.
**Branch:** `bugfix/BUG-049-small-monospace-text-ignores-the-reading-font`
**Screens:** All; seen on Reflog (1M).
**Raised by:** the author, with a screenshot of the reflog: "also this font
seams not correct".

## Problem

Settings › Reading publishes `--code-font`, which diffs and the review room
use. Everything else monospaced — the `.mono` class and 23 direct uses: commit
ids, counts, ref names, the footer's version — reads `--font-mono`, a fixed
system stack that resolves to Consolas on Windows. So one window had two
monospace faces, and one of them nobody chose.

In the reflog, "created at" sits inside the ids cell, inherits that face and
wraps onto two lines in its 150 px column.

## Scope

The author chose one monospace face everywhere over fixing the reflog row
alone.

- `--font-mono` follows the code face from Settings › Reading. When the code
  face is Lexend, which is not monospaced, it is Atkinson Hyperlegible Mono
  instead, because ids and hex dumps are monospaced so that they line up.
- The stylesheet's own `--font-mono` leads with Atkinson Hyperlegible Mono, the
  default code face, so the first frame does not change face.
- The reflog's "created at" is set in the interface face, and the ids cell keeps
  to one line like the cells beside it.
- The review room's plain `Aa` setting still sets code in the desktop's
  monospace: it is the code as it was set before Reading existed, on purpose.

## Acceptance criteria

- No text outside the review room's plain setting is set in Consolas unless
  System monospace is chosen in Settings › Reading.
- Changing the code face changes ids and counts too; Lexend leaves them in
  Atkinson Hyperlegible Mono.
- "created at 8103b44" is one line in the reflog.

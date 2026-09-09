<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-039 — One colour system in every dialog

**Status:** Done.
**Screen:** Worktrees, Submodules, Pull requests, Settings → Profiles, Diff
(binary and image), File history, and the status strip.
**Raised by:** a UI review, which found that a polished main screen can open a
dialog painted from a different colour system, and that the light themes make
this obvious.

## Problem

`src/lib/ui/flat.test.ts` carried a `KNOWN` list of **eleven components** that
read CSS custom properties nothing defines — `--fg`, `--dim`, `--bg-1`,
`--bg-2`, `--bg-3`, `--bg-hover`, `--border-soft`, `--text`, `--text-secondary`,
`--fs-body`. Every one of them fell through to a hard-coded fallback:
`var(--fg, #eee)` is valid CSS that works, so the Worktrees dialog painted
`#eee` on `#141416` whatever theme was on. Eight palettes, sixteen variants, and
these dialogs had one: a dark one, borrowed from an early Spagitty that no
longer exists.

The list was written as a shrinking debt rather than an exemption, and this is
where it ends. It is empty.

**The literals were the larger half, and nothing could see them.** The same
eleven files also wrote colours as plain values, which read no token at all and
so passed every assertion in that file:

- `rgba(0, 0, 0, 0.45)` as the modal scrim, in four dialogs. `DialogHost`
  already derives its backdrop from the theme's own ink, for the reason written
  beside it: on a light theme a near-black wash is a hole in the page rather
  than a recession.
- `rgba(238, 176, 77, …)` — the brand amber — as "the accent", in the blame
  gutter's row highlight and the main-worktree pill. That is Catppuccin Mocha's
  accent and nobody else's; `themes.ts` gives every family its own for exactly
  this reason, and these two rows overrode it.
- `#ffc107` and `#f44336` for locked and prunable worktrees, and
  `rgba(46,125,31,…)`, `rgba(188,106,0,…)`, `rgba(210,15,57,…)` for submodule
  status. Material Design's palette and a frozen copy of Catppuccin Latte's,
  used where `--warn`, `--danger` and `--ok` exist and are contrast-checked per
  family.
- A transparency checkerboard in two fixed near-blacks, so an image with an
  alpha channel was checked against a dark grid on the light themes, where it
  was the darkest thing on screen.
- `color: #111` on the image-diff divider handle and `color: #ffffff` on two
  filled buttons in `PRDiffPane`, where `--on-accent` is the token whose entire
  job is being contrast-checked against the fill underneath it.

**Type in these files could not be resized.** `scale.svelte.ts` implements the
text-size preference by rewriting the `--fs-*` tokens and nothing else. These
eleven components wrote 83 fixed pixel sizes between 10px and 16px, so somebody
who raised the text size to 130% got a larger application and eleven components
that stayed exactly where they were — with 10px labels in Profiles and 11px
pills in Worktrees, against a 15.6px default everywhere else.

## Change

- The eleven components read the real semantic tokens. The mapping is the
  obvious one — `--fg`→`--ink`, `--dim`→`--muted`, `--bg-1`→`--bg`,
  `--bg-2`→`--surface`, `--bg-hover`→`--hover`, `--border-soft`→`--line`,
  `--text`/`--text-secondary`→`--ink`/`--muted`, `--fs-body`→`--fs-ui` — except
  `--bg-3`, which was one fallback shade doing three different jobs and is
  resolved per use: `--soft` for badges, pills and icon buttons, `--sunken` for
  wells (the command-output box, the mode-tab strip), `--surface-2` for the
  raised timeline card, `--accent-soft` for a selected segmented control.
- Every literal colour is a token or a `color-mix` of one. The checkerboard is
  derived from `--sunken` and `--soft` rather than added to the palette, on the
  argument `--graph-bg` already makes in `app.css`: a checkerboard is not a
  colour anybody chooses per theme, and eight hand-picked pairs is eight
  chances for one to drift.
- Fallbacks on tokens that *do* exist — `var(--line, #333)` and eleven others —
  are gone. They never fired, so they were not a bug; they were a second colour
  written next to the real one, waiting for the next rename to turn a broken
  build into a silent dark literal.
- All 83 fixed type sizes become `--fs-*`. The decorative 32px glyph in
  `BinaryDiff` becomes `2em` so it follows its own pane.
- `KNOWN` in `flat.test.ts` is empty, and a **new assertion** forbids a literal
  colour anywhere in any component's stylesheet — the half of the defect that
  file was structurally unable to see.
- `CloneModal` and `PRDiffPane` are fixed too, though neither was on the list:
  the scrim and the filled-button text are the same defect, and leaving two
  known instances so the count reads eleven would be arithmetic rather than
  work.

## Acceptance criteria

- `flat.test.ts`'s recorded list is empty and its stale-row assertion passes.
- No component stylesheet contains a hex or `rgb`/`rgba` literal.
- Every dialog above renders in the current family's colours on all sixteen
  variants, dark and light, including an imported palette.
- Raising the text-size preference resizes the type in all eleven.

## Non-scope

- **The layout of these dialogs.** Padding, widths and the arrangement of their
  controls are untouched; only colour and type size changed.
- **The rest of the application's fixed sizing.** Control heights, hover
  motion and the chrome's own metrics are a separate item.

## Dependencies

FEAT-068 is where the defect was first found and named, in
`ExternalToolsSection`; the assertion written for it is the one this finishes.

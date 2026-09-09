<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-080 — Automated tests

**Item:** [`agile/items/FEAT-080-follow-omarchy.md`](../items/FEAT-080-follow-omarchy.md)

## What was tested

21 Rust tests in `src-tauri/src/desktop.rs`, 26 in `src/lib/colour.test.ts`, 23
in `src/lib/omarchy.test.ts`, and 13 added to `src/lib/theme.test.ts`.

**The fixture is the real file.** `colors.toml` from the machine this was raised
on, `sushi-dark-palette`, in both the Rust tests and the frontend ones. A parser
tested only against its author's idea of the format is a parser tested against
itself, and the whole premise of this feature is that a real palette turned out
to carry two roles that are wrong for an application.

### Finding and parsing (Rust)

| Test | Asserts |
| --- | --- |
| reads the real file | Every named role comes back from the genuine article. |
| keeps ANSI slots in their places | A gap stays a gap. Slot 4 is blue and slot 5 is magenta; compacting would silently recolour the graph. |
| refuses a file with no ground or no text | Not a palette. Reporting it available would mean following a desktop into an unreadable window. |
| falls back to the short names | `bg`/`fg` as well as `background`/`foreground`. |
| ignores what it does not understand | Numbers, booleans, non-bare keys, single-quoted values. |
| **stops dead at a table header** | The one thing a flat scanner must not do is read a nested key as a top-level one. Under-reading is recoverable; mis-reading paints the wrong colours and looks like it worked. |
| refuses values that are not colours | `url(...)`, `red`, short hex, `rgb()`, an embedded `;`. |
| normalises case and the missing hash | One spelling reaches the frontend. |
| drops a value too long to be a colour | Bounded. |
| keeps no more than the cap | Bounded. |
| looks in the state directory first and the old one after | With Omarchy's own defaults, not `dirs`' opinion of them. |
| honours the XDG variables when they are set | |
| treats an empty XDG variable as unset | Several shells export one by accident. |
| finds an installed theme and names it | End to end against a temporary home. |
| **reports nothing where there is nothing** | Detection is evidence. An empty home is not Omarchy. |
| tells an unreadable palette from an absent one | Two different sentences; only one is worth showing. |
| reads the legacy layout and takes its name from the link | The older layout has no `theme.name`. |
| prefers the state layout over the legacy one | An upgraded installation keeps the old directory; following it would track a theme nothing else on the machine uses. |
| **follows the directory being replaced** | The `next-theme`-then-swap dance `omarchy-theme-set` actually performs, reproduced in the test. |
| refuses a name that is not a name | `../../etc/passwd` is not a label. |
| refuses a palette that is not a bounded file | A directory where a file should be. |

### The arithmetic (`colour.test.ts`)

Pinned to WCAG's published bounds — black on white is 21:1, anything on itself
is 1:1 — and to the four measurements the whole feature rests on, recomputed
here rather than taken from the report that supplied them:

```
foreground on background  17.22:1
muted on background        2.75:1   fails ordinary text
foreground on accent       3.70:1   fails a filled button's label
background on accent       4.65:1   passes
```

Deliberately **not** shared with `themes.test.ts`, which keeps its own copy: a
test that checked the palettes using the application's own contrast function
would agree with itself and pass whatever the function did.

Two behavioural assertions are worth naming. `picks the label a filled accent
can actually carry` is the one that says white would have been wrong. `lifts an
unreadable muted onto the 4.5:1 line` also asserts the result is **not** simply
the foreground — the walk stops at the first step that clears, so the desktop's
own hue survives as far as it can.

### The derivation (`omarchy.test.ts`)

Held to the same bars `themes.test.ts` holds the eight built-in families to,
across two palettes and a deliberately bare one:

- ordinary text clears 4.5:1 on both `bg` and `panel`;
- accent, danger, warn, ok and all five lanes clear 3:1;
- the five lanes are five *different* colours;
- every one of the fourteen tokens is set, even from a palette that names
  almost nothing — a missing token falls through to the previous theme, which
  is how half a theme ends up on screen;
- an ANSI slot is used only for the hue that slot means.

**Two defects were found by these tests rather than by reasoning**, and both are
recorded because the reasoning had been written down first and was wrong:

- `onAccent` came back `#000000` rather than the desktop's own `#191724`,
  because the first implementation took the *most* readable candidate. Contrast
  is a threshold to pass, not a score to win; `firstReadable` replaced it.
- The light desktop produced four lane colours, not five: Rosé Pine Dawn gives
  `blue` and `cyan` the same `#56949f`, and two branches would have been drawn
  identically. Lanes are de-duplicated now.

### The store (`theme.test.ts`)

The third source, and what makes the revision move:

| Test | Asserts |
| --- | --- |
| is not offered until a palette has actually been read | No control that would do nothing. |
| paints the desktop palette once it is being followed | And `--muted` is **not** the desktop's raw value. |
| takes the mode from the desktop palette | Or `app.css`'s boot values fight the inline properties. |
| **keeps the last good palette through an unreadable reading** | The mid-swap window. Falling back would be a flash of Catppuccin in the middle of a theme change. |
| stops following when a family is chosen | Two answers to one question. |
| leaves the system source alone when a family is chosen | A family and light/dark are not one question. |
| says the desktop name rather than a variant it is not | |
| the revision moves when the family changes | |
| **the revision moves between two desktop palettes that share an id** | The case `id` cannot see, asserted alongside `expect(theme.id).toBe(id)` so the test states the problem as well as the fix. |
| the revision stands still when nothing actually changed | An event that re-reads an identical palette is not a repaint. |

## Run against the real machine

Both sides, once, on the machine that raised this.

**The backend**, `detect()` against the real `$HOME`, returned `available:
true`, `name: "sushi-dark-palette"`, `layout: State`, and the complete palette
including all sixteen ANSI slots.

**The derivation**, run over the same file:

```
bg #191724   panel #13101e   ink #fcfcfd   muted #86868a   accent #cf6348
onAccent #191724   danger #bda8a4   warn #cd9071   ok #8fa487
lanes #778291 #a1758a #bda8a4 #4a89a4 #8fa487

ink on bg 17.22    muted on bg 4.87 (raw 2.75)    label on accent 4.65
lanes on bg 4.53 4.56 7.82 4.54 6.58
```

Every ratio clears its bar, and `muted` moved from failing to passing while
keeping the desktop's own neutral. That is the feature working, on the machine
it was asked for, measured rather than looked at.

## The whole suite

130 files, 2,760 tests, all passing. `bun run check`: 0 errors, 0 warnings over
1,149 files. `cargo test --workspace`: 95 passed. `cargo clippy --workspace
--all-targets -- -D warnings`: clean. `cargo fmt --all --check`: clean.

## What is not covered, and why

- **The watcher firing.** `notify` needs a real filesystem event and a running
  Tauri app to emit into. The *consequences* are tested — the debounce compares
  against the last valid palette, and the store drops a failed reading — but
  that inotify delivers the event is `SWEEP-004`.
- **The canvas repainting.** The revision is asserted; that `LaneCanvas`
  redraws is a rendering fact. `SWEEP-005`.
- **Other Omarchy versions and other generators.** One desktop, two palettes,
  one machine. `SWEEP-009`.
- **macOS and Windows.** The backend returns "Omarchy is a Linux desktop"
  without touching a filesystem; that no error reaches a user there is
  `SWEEP-010`.

## Coverage

`src/lib/colour.ts` and `src/lib/omarchy.ts` are first-party and each has its
own file covering every branch, including all three fallback rungs of the lane
selection and both directions of every contrast decision. `src-tauri/src/desktop.rs`
carries its tests inline, as every module in that crate does.

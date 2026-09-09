<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-080 — Follow Omarchy

**Status:** Done.
**Screen:** Settings → Appearance, and every screen that reads a colour.
**Raised by:** the author, on a machine running Omarchy: Spagitty should be able
to take its colours from the desktop it is running on.

## Problem

Spagitty carries eight palette families and no way to say "whatever the desktop
is using". On Omarchy the active palette is a fact on disk that every other
application on the machine already follows — the terminal, the editor, the bar,
the notification daemon — and Spagitty is the window that does not match.

## Change

**A third theme source.** BUG-031 separated *where* the light/dark answer comes
from (`manual`, `system`) from what it currently is; this adds `omarchy`, which
supplies the whole palette rather than only the mode. It is offered in
Appearance only when a palette has actually been read, so a machine that is not
running Omarchy never sees a control that would do nothing.

**`src-tauri/src/desktop.rs`** is the whole reading side: find the palette,
parse it, validate it, return a typed answer, and say when it changes. Three
commands — `desktop_theme`, `desktop_theme_watch`, `desktop_theme_unwatch` —
and **no path among them**. The frontend cannot ask the backend to read a file;
it can ask what the desktop is painted with.

**`src/lib/omarchy.ts`** turns the desktop's vocabulary into Spagitty's fourteen
tokens, and **`src/lib/colour.ts`** is the arithmetic it needs at runtime. That
split is deliberate: finding and parsing is a filesystem problem and belongs in
Rust; deciding what colour a secondary label is has contrast arithmetic in it
and belongs where `themes.test.ts` can hold it to the same ratios as the eight
built-in families.

**A palette revision.** `theme.id` is `family-mode`, so every followed palette
is `omarchy-dark` and a switch between two dark desktop themes is invisible to
anything keyed off it. `LaneCanvas` invalidates its lane colours and its entire
portrait cache from exactly that string, so it would have kept painting the
previous desktop theme's graph. `theme.revision` counts changes to the palette's
*values* and is what the canvas reads now.

## A literal copy would be unreadable, and this is measured

Derived from the palette on the machine this was raised on
(`sushi-dark-palette`), by `src/lib/colour.ts` and confirmed against the report
that asked for the feature:

| Pair | Ratio | Verdict |
| --- | --- | --- |
| `foreground` on `background` | 17.22:1 | excellent |
| **`muted` on `background`** | **2.75:1** | fails ordinary text |
| light `foreground` on `accent` | 3.70:1 | fails a filled button's label |
| dark `background` on `accent` | 4.65:1 | passes — so this is the label colour |

Two of the four roles a terminal palette carries are wrong for an application
the moment they are copied across, and one of them is `muted`, which is what
every timestamp, path and secondary label in Spagitty is painted with. A
terminal's `muted` is a comment colour in a wall of monospace; a UI's is a
timestamp beside a commit subject. They are not the same requirement.

So the rule is **take the desktop's hues, derive the app's roles**: `muted`
keeps the desktop's colour and is walked toward the foreground until it clears
4.5:1, and `onAccent` is measured between the palette's own extremes rather than
assumed to be white. What this actually produces on that machine:

```
bg #191724   panel #13101e   ink #fcfcfd   muted #86868a   accent #cf6348
onAccent #191724   danger #bda8a4   warn #cd9071   ok #8fa487
lanes #778291 #a1758a #bda8a4 #4a89a4 #8fa487

ink on bg 17.22    muted on bg 4.87 (raw was 2.75)    label on accent 4.65
lanes on bg 4.53 4.56 7.82 4.54 6.58
```

## Detection is evidence, not inference

`XDG_CURRENT_DESKTOP=Hyprland` proves nothing: most Hyprland installations are
not Omarchy, and Omarchy under a different session would be missed. What counts
is an Omarchy-shaped directory **and** a palette in it that parses. Two layouts
are tried, newest first:

| Layout | Palette | Name |
| --- | --- | --- |
| state | `$XDG_STATE_HOME/omarchy/current/theme/colors.toml` | `current/theme.name` |
| legacy | `$XDG_CONFIG_HOME/omarchy/current/theme/colors.toml` | the `current/theme` symlink's target |

with the defaults Omarchy's own scripts use. "Installed but unreadable" and "not
installed" are reported as different sentences, because only one of them is
worth putting in front of anybody.

## Watching survives the directory being replaced

`omarchy-theme-set` does not edit `colors.toml`. It builds a `next-theme`
directory and **replaces** `current/theme`, so a watch registered against the
palette file follows an inode that stops being the current palette after the
first change — it would work once, in testing, and never again. The watch is on
`current` recursively and on the root above it, and the palette is re-read from
the path each time.

A partial update — the moment after the old directory has gone and before the
new one is in place — reads as unavailable. That reading is **dropped**, not
applied: falling back to the built-in family for that moment would be a flash of
Catppuccin in the middle of a theme change, which is the opposite of the
feature. The same behaviour covers Omarchy being uninstalled while the window is
open.

## What it does not do

- **It does not edit the desktop.** No hooks are installed, nothing under
  `~/.config/omarchy` or `~/.local/state/omarchy` is written.
- **It does not source or execute anything.** The theme directory also holds
  `hyprland.lua`, `gum_env.lua`, `neovim.lua` and a 55 KB `vscode-theme.json`.
  Only `colors.toml` is read, and only as data.
- **It does not import CSS or take the desktop's fonts.** Spagitty follows the
  desktop's colours; it does not become a terminal.
- **It adds no dependency.** `notify` was already used for repository watching.
  The palette parser is written here — forty lines against a generated file with
  no tables, arrays, datetimes or nesting — rather than pulling a TOML crate
  into the binary to parse a language this file is one sentence of.

## Acceptance criteria

- Follow Omarchy appears in Appearance on a machine running Omarchy, and
  nowhere else.
- Choosing it repaints the DOM **and** the graph canvas, immediately.
- `omarchy-theme-set` while the window is open repaints it again, with no
  intermediate frame of the built-in palette.
- A light desktop palette and this dark one both clear 4.5:1 for ordinary text
  and 3:1 for indicators and lanes.
- Choosing a family opts out, and the choice survives a restart.
- macOS, Windows and a Linux machine without Omarchy behave exactly as before,
  with no errors.

## Dependencies

BUG-031 separated the theme's source from its resolved mode and made the
first-paint cache hold resolved properties rather than a family name; both are
prerequisites, and the cache is why a followed palette survives a cold start.
FEAT-079 put the portrait cache in `LaneCanvas` that the revision invalidates.

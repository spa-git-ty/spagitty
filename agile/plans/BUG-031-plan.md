<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-031 — Plan

**Item:** [`agile/items/BUG-031-the-window-opens-in-the-wrong-theme.md`](../items/BUG-031-the-window-opens-in-the-wrong-theme.md)

## Approach

The flash and the frozen preference look like two bugs and are one shape: the
theme was a single value carrying two different meanings, resolved at the wrong
time. So the storage model is separated first — source, mode, family, and a
cache of the resolved result — and the boot script falls out of it, because once
the resolved palette is written down there is something for a first frame to
read.

The order matters. Writing the boot script first would have meant caching a
family name and reloading the palette table in the boot path, which is the
expensive answer to a timing problem.

## Decisions

- **Cache the resolved properties, not the family and mode.** The boot path must
  not import `themes.ts`, and "whatever was on screen last time" is the actual
  requirement. It also generalises: a palette from outside the built-in table
  caches identically, which is what FEAT-080 needs and what a family name could
  never express.
- **A file, not an inline script, and not a CSP hash.** Argued in the item. The
  short version: `default-src 'self'` already allows it, and the two
  alternatives each trade a real policy or a silent-breakage join for one saved
  request against a local bundle.
- **A Vite plugin rather than moving the assets directory.**
  `svelte.config.js` scopes assets to `assets/brand/favicon` because
  `tools/make-brand.py --check` regenerates exactly those four paths. Putting a
  script in there, or repointing the directory and moving the art, are both
  bigger changes than twenty lines of plugin.
- **`generateBundle`, not `writeBundle`.** The file has to be part of the bundle
  for the static adapter to carry it into `build/`.
- **Validate on the way in, drop per token.** A bad entry costs that token, not
  the theme. The value pattern is a whitelist of shapes rather than a blacklist
  of characters: `(` and `)` have to be allowed for `rgb()` and `color-mix()`,
  and a rule permitting those while excluding `url(` is a filter to get wrong.
- **Choosing a mode sets the source.** The alternative — a separate "follow the
  system" switch that has to be turned off first — produces a control that
  appears to work and is then overruled by the next system change.
- **A third chip, not a switch above the pair.** The three are one decision.
- **The migration reads an orphan mode as `manual`.** Both mistakes are
  possible; this one is recoverable in one click and the other loses a real
  preference silently.
- **`addEventListener` with an `addListener` fallback.** The AppImage is built
  against an old glibc on purpose, so it can meet an old WebKitGTK. Both are
  feature-detected; a `MediaQueryList` with neither is handled by simply not
  following, which is what the existing test stub already looks like.

## Files

- `src/theme-boot.js` (new), `src/theme-boot.test.ts` (new)
- `src/lib/theme.svelte.ts`, `src/lib/theme.test.ts`
- `src/app.html`, `vite.config.ts`
- `src/lib/settings/AppearanceSection.svelte`, `src/routes/+layout.svelte`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. Split source from mode in the store; write the resolved cache on every apply.
2. The boot script, and the plugin and markup that deliver it.
3. Live system following, and its teardown from the shell.
4. The third chip in Appearance.
5. Tests: the store's new behaviour, the script evaluated as a browser runs it,
   and the three-file wiring join.
6. Changelog, this set and the index row.

## Risks and rollback

- **The boot script runs with no bundler around it.** It is plain ES5-shaped
  JavaScript in an IIFE, reads `window.matchMedia` rather than a bare global,
  and every branch is exercised by a test that evaluates the real file. A syntax
  error here is a blank window, so it is the one file in the frontend that is
  read off disk and executed by its own test rather than imported.
- **A stale cache paints one wrong frame** if the palette table changes and the
  cache does not. The window is one frame — `theme.init()` corrects it — and the
  alternative, validating the cache against the table, would need the table in
  the boot path, which is the cost being avoided.
- **Following the system is a listener that outlives a screen.** It is created
  only while that source is selected, torn down when it is not, and disposed by
  the shell's cleanup alongside every other listener there.
- Rollback is reverting the five source files; the markup and plugin are inert
  without the script.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-031 — Automated tests

**Item:** [`agile/items/BUG-031-the-window-opens-in-the-wrong-theme.md`](../items/BUG-031-the-window-opens-in-the-wrong-theme.md)

## What was tested

Twelve new assertions in `src/lib/theme.test.ts` and seventeen in the new
`src/theme-boot.test.ts`. The fifteen assertions the theme store already had
are unchanged and still pass, which is the first thing worth reporting: the
migration was designed so that every existing stored state keeps behaving as it
did.

### The store

| Test | Asserts |
| --- | --- |
| follows the desktop while it is set to, and keeps following | A change in the media query changes the mode **while the window is open**. This is the defect: the preference was sampled once and written down as a decision. |
| keeps the family while the desktop changes the mode | Nord stays Nord; only light/dark moves. |
| stops following as soon as a mode is chosen | Choosing sets the source. A control that changed the mode and left the source alone would be overruled by the next system change. |
| drops the listener when it stops following, and on dispose | No listener outlives the source that wanted it. |
| does not add a second listener when told to follow twice | The teardown runs before the setup, so re-entering is idempotent. |
| remembers the source across a restart | `spagitty.theme.source` round-trips. |
| treats a mode stored before the source existed as a decision | **The migration.** An orphan mode is `manual`, because demoting a real preference is the worse of the two possible mistakes. |
| starts a fresh install following the desktop | And keeps following, rather than freezing the first answer. |
| ignores a stored source that is not a source | Hand-edited storage falls back rather than breaking. |
| the cached palette: written on every change, resolved rather than named | The boot script's input is the resolved token map, not a family name. |
| the cached palette: follows a mode the desktop chose | Written from `apply()`, so a system change caches too — not only a user's click. |
| the cached palette: carries every token the boot script would need | The names are `app.css`'s, so the script needs to know nothing about families. |

### The boot script

Read off disk and evaluated with `new Function`, which is as close to a
`<script src>` tag as a test can get and fails if the file is renamed out from
under `vite.config.ts`. Most of the assertions are **refusals**:

| Test | Asserts |
| --- | --- |
| is on the root element before anything else runs | The cached palette lands, mode and colours. |
| overrides the document markup rather than waiting for the layout | `data-theme="light"` in the markup becomes `dark` before anything paints. The defect, in one assertion. |
| drops … and leaves the stylesheet alone (5 cases) | Not JSON, JSON that is not an object, an invalid mode, absent tokens, tokens that are not an object. |
| drops a token whose name is not a custom property | `background: red` is refused; the `--bg` beside it still applies. A bad entry costs that entry. |
| drops a value that is not shaped like a colour | A `url()`, a value with an embedded `;` and declaration, and a number. The good entry beside them still lands. |
| allows the derived forms a palette legitimately uses | `rgba()` and `color-mix()` — the filter is a whitelist of shapes, and it has to not reject the real palette. |
| survives storage being unreadable | A throwing `getItem` is a private-mode webview, not a crash. |
| takes the desktop preference with nothing cached | A fresh install on a dark desktop opens dark, and sets **no colours** — `app.css` has them. |
| leaves the markup alone on a light desktop / with no `matchMedia` | Both no-op cases. |
| how it reaches the page (3 tests) | The document asks for the script **before** `%sveltekit.head%`; `vite.config.ts` emits it under that name from that source; and the CSP still needs no `unsafe-inline`. |

That last group is the one that matters most in six months. The script is wired
through three files and is inert if any of them stops agreeing — and inert means
the flash comes back, which nobody reports as a bug because it looks like the
application being slow.

## What made the old behaviour untestable

Nothing here could have been caught by the fifteen existing assertions, and it
is worth being precise about why: they all called `theme.init()` and then read
`theme.mode`. The sampled-once behaviour produced the *correct* mode. The defect
was that it produced a correct mode **and a stored decision**, and there was no
third value to assert against — `source` did not exist. Adding the value is what
made the bug expressible.

## The whole suite

128 files, 2,695 tests, all passing. `bun run check`: 0 errors, 0 warnings over
1,145 files. `bun run build` emits `build/theme-boot.js` and `build/index.html`
references it.

## What is not covered, and why

- **The flash itself.** A test cannot see a frame. `SWEEP-001` and `SWEEP-002`
  are a camera and a stopwatch on a real window.
- **Cold-launch timing.** Asked for by the review; it is a measurement, not an
  assertion. `SWEEP-003`.
- **The dev-server middleware.** `configureServer` needs a running Vite.
  `SWEEP-007`.
- **CSP enforcement in the packaged app.** The test checks the policy still
  permits a same-origin script; that WebKit agrees is `SWEEP-002`.

## Coverage

`src/lib/theme.svelte.ts` is first-party and its new branches — both sources,
both listener APIs, the absent-listener case, and each migration path — are
covered by the store's own file. `src/theme-boot.js` sits outside
`coverage.include` (`src/lib/**`, `src/routes/**`) and so moves neither the
numerator nor the denominator, which is correct: it is the shell, like
`app.html`, and it has its own file of tests regardless.

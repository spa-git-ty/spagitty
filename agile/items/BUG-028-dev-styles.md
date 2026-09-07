<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-028 — Development styles depend on request order

**Status:** Fixed — the guard was incomplete and was corrected; extended sweep pending.
**Branch:** `bugfix/BUG-028-dev-styles`
**Screens:** All screens.

## Problem and reproduction

On Linux, the author supplied screenshots showing the window title and toolbar
stacked at the left and Farm's background ending above the window bottom.
The development server returned entire Svelte source files as CSS modules.

With the installed Vite 6.4.3 / Svelte plugin 5.1.1, request a component's
`?svelte&type=style&lang.css` before its JavaScript. The plugin's empty CSS cache
falls through to a source-file read. Vite caches that response; requesting the
component afterwards does not repair it. A standalone cold-request probe and
the running app's HTTP responses both confirmed this failure. The exact event
that first produced that request order in the author's session is unknown.

## Scope and acceptance criteria

- Development components carry their compiled, scoped CSS without a separate
  stylesheet request, including after a cold stylesheet request.
- Production continues extracting compiled CSS into assets.
- Title, toolbar and sparse-content screens retain their existing layout.
- Regression tests, Svelte checks and the frontend build pass.

## Non-scope

Redesigning screen layouts, changing stored farms or upgrading dependencies.
The author authorized visual testing in this session. Native UI automation is
unavailable. The supplied screenshots establish the before state; after the
server reloaded with the fix, the author confirmed "both fixed" for centering
and Farm background filling the window.

## Reopened and re-fixed, 2026-09-07

The fix was `emitCss: process.env.NODE_ENV !== 'development'` in
`svelte.config.js`, and it did not work for the case it was written for.

`@sveltejs/kit/vite` reads `svelte.config.js` **eagerly**, while `vite.config.ts`
is still being evaluated and before Vite has set `NODE_ENV` for the run. A bare
`vite dev` — which is what `bun run dev`, and therefore `tauri dev`, was —
reached the guard with the variable unset. `!== 'development'` answered
*production*, `emitCss` stayed on, and every development run armed the exact
failure the guard exists to disarm. It was found in a `tauri dev` log:

```
[postcss] .../CommitRows.svelte?svelte&type=style&lang.css:3:11: Unknown word graph
  3  |  	import { graph } from '$lib/graph/store.svelte';
```

— the raw component source being parsed as CSS, which is BUG-028's exact
signature, on a build that was supposed to be immune to it.

`tools/dev-styles.test.ts` passed throughout, because both of its tests set
`NODE_ENV` before compiling. The case that was wrong is the one they could not
express.

**What changed.** The question is now asked as `=== 'production'`, so an unset
variable takes the *development* path — the safe direction, where a wrong guess
costs styles inlined in the JS rather than a broken page. `package.json` names
the environment on both `dev` and `build`, so the answer is never inferred.
`tools/dev-styles.test.ts` gained a fourth describe block that loads
`svelte.config.js` under each environment directly, including absent, and
asserts the scripts still say which one they are. That block fails against the
old condition.

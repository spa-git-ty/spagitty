<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-028 — Development styles depend on request order

**Status:** Fixed — author confirmed centering and window fill; extended sweep pending.
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

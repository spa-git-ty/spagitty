<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-028 — Automated validation

`tools/dev-styles.test.ts` exercises the installed Vite/Svelte plugin in isolated
processes using the repository's actual Svelte configuration and title bar.

- Development: request CSS before JavaScript; assert the component still carries
  scoped grid styles and has no separate CSS import.
- Production: assert CSS remains extracted, scoped, and free of Svelte markup.

Before the fix, the running server returned raw Svelte markup for TitleBar's
stylesheet. After the fix, HTTP probes of TitleBar and the app layout both
returned injected styles with no external component-style imports.

Baseline countercheck: overriding `emitCss` to its former value (`true`) gives
`baselineInjected: false`, `baselineExternalImport: true`, `baselineRawCss: true`.
The new development assertions therefore reject the original behavior.

Full frontend coverage run: 2,551 tests passed across 122 files, including both
new compiler probes and the record checks. Statements 79.60%, branches 70.95%,
functions 77.86%, lines 81.43%; every 70% threshold passes. Svelte check reports
zero errors and warnings. `bun run build` passed; emitted CSS assets contain
the scoped title-bar grid and shell flex rules. Final record checks passed
(550 tests), and `git diff --check` is clean.

Local diff review completed: change is limited to the development compiler
option, regression probes and records. No dependency or layout CSS changed.

Only build configuration changes; no first-party runtime logic is added. The
child compiler process required execution outside the sandbox after EPERM.

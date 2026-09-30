<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-032 — A path test that only passes on Unix

**Status:** Fixed.
**Branch:** `bugfix/BUG-032-a-path-test-that-only-passes-on-unix`
**Screens:** none. It is a test, and it fails the suite on Windows.

## Problem

`bun run test` on Windows fails one test out of 2,913:

```
FAIL  src/lib/requests/requests.test.ts > the promises this screen makes > makes its requests from exactly one file
AssertionError: expected [ 'forge\http.rs' ] to deeply equal [ 'forge/http.rs' ]
```

The test walks `crates/spagitty-core/src` for files that mention `ureq` and
compares the paths it finds against `forge/http.rs`. It cuts the prefix off with
`path.slice(core.length + 1)`, which leaves the platform's own separator in the
rest. The pipeline runs this suite on Ubuntu only, so it has never failed there.

## Scope

- Compare the paths in `/` form whatever the platform, the way
  `tools/case.test.ts` already does.

## Non-scope

- Running the frontend suite on Windows in the pipeline.

## Acceptance criteria

- The test passes on Windows and Linux, and still fails if a second file in the
  core mentions `ureq`.

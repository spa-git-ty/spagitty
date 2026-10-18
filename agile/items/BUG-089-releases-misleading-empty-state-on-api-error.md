<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-089 — Releases shows "No releases match this filter" after an API error

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1X (Releases)
**Raised by:** the author, 2026-10-18, on 3.1.0: after a 302 error the sidebar says "No releases match this filter." / "All 0".

## What happens

When the Releases request fails (e.g. HTTP 302), the list is empty and the view falls through to "No releases match this filter.", implying a successful empty result rather than an error.

## Cause

`ReleasesScreen.svelte` line 63 renders the filter-empty message whenever `visible` is empty and not reading, without checking `releases.errors`. An error leaves an empty list, so the misleading message appears.

## Acceptance criteria

- On an API error, the error is shown (already via the alert at line 46) and the filter-empty message is not shown.

## Fix

- `src/lib/releases/ReleasesScreen.svelte`: gate the filter-empty message on `!releases.errors` (or any error being absent).

## Not done

- No retry button added; the existing Refresh button remains.
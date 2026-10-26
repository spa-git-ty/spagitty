<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-101 — Every edit in Calls says "Could not save … already exists"

**Status:** Fixed — on `fix/BUG-100-bruno-yaml-import`, in release 4.0.1. Covered by a mounted test.
**Branch:** `fix/BUG-100-bruno-yaml-import`.
**Screens:** Calls (2C).
**Raised by:** the author, 2026-10-26, with a screenshot: "also evryword i type it pop this notification". The notification read *Could not save .spagitty/calls/dxp-dashboard.json — .spagitty/calls/dxp-dashboard.json already exists*.

## Cause

An edit is saved a moment after the typing stops. That save passed the collection's stem with `was` empty. The store reads an empty `was` as *a new collection*, and refuses to write over a file that is already there. So every save after the first failed, and nothing typed was kept.

## Fix

The save passes the stem as `was` too: the file is this collection's own, to write over. The store's guard is unchanged, so a new collection still cannot take an existing collection's file.

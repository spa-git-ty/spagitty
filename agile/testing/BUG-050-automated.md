<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-050 — Automated test record

**Item:** [`agile/items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md`](../items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md)

## What was tested

`tools/release-macos.test.ts`, *agrees to the image's licence when it mounts
it*: the bundle sets a licence file, and the verify action's `hdiutil attach`
line answers `Y` on stdin. Checked against the action as it was before the fix,
whose attach line it finds and rejects.

The action still parses as YAML, and its step parses as bash (`bash -n`).

## Test command and output

On Windows 11: `bunx vitest run tools/release-macos.test.ts` — 40 passed.

## What is not covered automatically

Whether `hdiutil` on a Mac runner takes the answer. Only a run on GitHub's Mac
runners shows that. See the sweep.

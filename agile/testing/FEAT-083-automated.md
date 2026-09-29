<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-083 — Automated test record

**Item:** [`agile/items/FEAT-083-the-content-in-the-spatial-language.md`](../items/FEAT-083-the-content-in-the-spatial-language.md)

## What was tested

This item changes how things are drawn and nothing about what they do, so no
behaviour test was added: every existing Graph, chrome and chip assertion
still holds, which is the check that no class, role or geometry the screen
depends on moved.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1164 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  2945 passed (2945)
```

## What is not covered automatically

How it looks, and that the lanes did not move. Screenshots of the Graph with
this repository open were taken from the release build on Windows and from a
headless browser in both themes during the work; the sweep repeats them.

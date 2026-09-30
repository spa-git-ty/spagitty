<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-084 — Automated test record

**Item:** [`agile/items/FEAT-084-the-other-screens-in-the-spatial-language.md`](../items/FEAT-084-the-other-screens-in-the-spatial-language.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/ui/flat.test.ts` | Branches, Tags and Reflog rows are the lists' size, rounded, with no rule, and their actions are hidden until the row is hovered or holds focus. A tag's name cell clips and ends in an ellipsis. The pane clears `--band-rule`, and every header or footer rule in `src/routes` that uses the old colour reads it through the token. |

The three tests were run against the screens before the change with the new
test file in place: all three fail. With the change they pass.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  3008 passed (3008)
```

## What is not covered automatically

How the screens look. Checked in the Windows release build on a repository
with 4 local and 1,024 remote-tracking branches, 18 tags and 129 reflog
entries — see the sweep.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-036 — Automated test record

**Item:** [`agile/items/BUG-036-the-commit-list-a-size-above-the-rest.md`](../items/BUG-036-the-commit-list-a-size-above-the-rest.md)

## What was tested

`src/lib/ui/flat.test.ts`: the Graph's message and text cells, and the file
names every other list uses, all declare `--fs-secondary`.

## Test command and output

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  2973 passed (2973)
```

## What is not covered automatically

How it reads. Checked in the Windows release build on a repository with a
branching history: the messages are the size of the rest of the screen, and the
lanes and nodes are where they were.

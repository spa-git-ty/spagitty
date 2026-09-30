<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-051 — Automated test record

**Item:** [`agile/items/TASK-051-one-theme.md`](../items/TASK-051-one-theme.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/themes.test.ts` | One family with two variants; Giorno and Notte by name; `catppuccin` and `nord` are not families; Pomodoro's tokens by name; every contrast rule for both variants. |
| `src/lib/theme.test.ts` | A stored `nord` opens on Pomodoro in its stored mode and is written back as `pomodoro`; the variant is Notte or Giorno; every other store behaviour against Pomodoro. |
| `src/lib/settings/sections.test.ts` | Appearance offers no family, only the modes. |
| `src/lib/omarchy.test.ts` | A desktop palette is told apart from Pomodoro. |

Removed with what they tested: moving between families, the one-time move from
Catppuccin, each family's accent differing from the others', and Appearance's
family grid.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1166 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Tests  3025 passed (3025)
```

## What is not covered automatically

What Appearance looks like with one theme. See the sweep.

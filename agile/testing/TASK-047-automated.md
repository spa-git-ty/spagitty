<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-047 — Automated test record

**Item:** [`agile/items/TASK-047-a-commit-bar.md`](../items/TASK-047-a-commit-bar.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/changes/panes.test.ts` | The description is absent until asked for, opens on "Add description", and the button goes once it is open; a body that holds text is shown without asking; subject and body still write to the store; the body keeps its three rows and is capped at 12em. Signing notes and amending pass unchanged. |
| `src/routes/changes/page.test.ts` | A clean working copy offers no commit bar and no Commit button. Committing a staged file, and a write failure, pass unchanged with the button in the bar. |

### Tests changed, and why

The body tests typed into a field that is now hidden until asked for, so they
ask first. The height test checked the old well's 40% cap; the well is gone,
and it checks the body's own cap. The clean-repository test asserted a
disabled Commit button; there is no bar to hold one.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  2966 passed (2966)
```

## What is not covered automatically

How much of the screen the diff gets. Checked in the release build.

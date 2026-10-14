<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-083 — Automated tests

**Item:** [BUG-083](../items/BUG-083-hiding-the-blame-in-files-breaks-every-line.md)

| Test | File | What it checks |
| --- | --- | --- |
| keeps the number and the text in their own columns, with the blame or without it | `src/lib/files/files.test.ts` | `.number` is in column 2 and `.text` in column 3. Fails on 3.0.0. |
| is widened by dragging its edge, which goes when it does | `src/lib/files/files.test.ts` | The blame's divider is there with the blame and gone without it. |
| names the file, the revision and the line | `src/lib/files/open.test.ts` | The `/files?…` address. |
| reads a file a commit deleted where its parent left it | `src/lib/files/open.test.ts` | `revOfChange`. |
| says why a file has nothing to read | `src/lib/files/open.test.ts` | `fileAttrs` and its reason. |
| reads a pull request's file at its head, and a deleted one where the head left the target | `src/lib/files/open.test.ts` | `pullFileAttrs`. |
| opens a file at the revision its row shows | `src/lib/files/open.test.ts` | A right-click inside a marked row opens the menu, and the entry goes to Files there. |
| shows why a file cannot be opened, and does not open it | `src/lib/files/open.test.ts` | A disabled entry with its reason. |
| leaves alone a right-click that is not on a file, or that a row has answered | `src/lib/files/open.test.ts` | No menu over another one, or off a file. |
| reset | `src/lib/panels.test.ts` | The stored panels include `filesBlame`. |

`bun run check` and `bun run test` pass.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-085 — Automated test record

**Item:** [`agile/items/FEAT-085-a-new-mark-palette-and-wordmark.md`](../items/FEAT-085-a-new-mark-palette-and-wordmark.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `tools/make-icons.py --check` | Every application icon matches what the mark renders to; the two `mark.svg` copies are identical. |
| `tools/make-brand.py --check` | Every lockup, favicon, tray and menu bar mark, the banner and the preview page match. |
| `src/lib/ui/flat.test.ts` | `--brand` is defined for light and dark; the wordmark draws "git" in it; only `Wordmark.svelte` names Sora. |
| `src/lib/chrome/chrome.test.ts` | With nothing open the title row is labelled "Spagitty" and reads "spagitty"; it is still the middle of three columns. |

## Test command and output

In WSL, Pillow 12.3.0 from pip (the version gate 2 installs):

```
$ python tools/make-icons.py --check
icon set matches the committed sources
$ python tools/make-brand.py --check
brand collateral matches the committed sources
```

On Windows 11:

```
$ bun run check
COMPLETED 1166 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Tests  3021 passed (3021)
```

## What is not covered automatically

How the mark and the wordmark look, at size and in the application: previews
were rendered at 512, 128, 64, 32 and 16 px and on dark and light surfaces, and
the application is checked in the release build — see the sweep.

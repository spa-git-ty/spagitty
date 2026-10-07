<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-053 — What the author found sweeping

**Status:** Fixed — merged into `main`; the manual sweep is not yet run.
**Branch:** `bugfix/BUG-053-what-the-author-found-sweeping`
**Screens:** 1H, 1A, chrome.
**Raised by:** the author, 2026-10-07, sweeping the 1.0.1 release.

## Problem

1. Pull requests still pulsed grey placeholder rows while loading, not the loader every other screen uses (BUG-052).
2. Graph nodes, portraits and merge dots, read small beside GitKraken's.
3. A merge's line to its other parent ran down half a row beside the dot before it turned — a short parallel stub. GitKraken leaves the dot sideways and turns once.
4. Double-clicking the Branch / Tag column's border only reset it; long branch names stayed cut.
5. Author pictures came from the forge only on GitHub. On GitLab — the author's self-hosted work server included — and Bitbucket every ordinary address fell back to Gravatar, so most authors drew generated faces.
6. The status strip said `GPL-3.0 · v0.1.0` under 1.0.1: the number was a typed fallback that never followed the release, and the author wants no licence label there.

## Change

1. The loader replaces the placeholder rows.
2. Portraits are 24px across (22), merge dots 11px (9); the lane pitch is 28 (26) so two heads still clear each other.
3. A line that leaves a node, or reaches one, in another lane runs straight across from the node's own centre and turns once into the other lane (`turnsAt`). Only a lane passing between two nodes that are neither of its ends turns in the middle of the band, as before.
4. Double-clicking the Branch / Tag divider fits the column to the widest labels in every row walked so far, like a spreadsheet column (capped at 640px); other columns still reset.
5. On GitLab the instance's `avatar` endpoint is asked by address (with a connected token, so a private instance answers), its identicon stand-in is asked for as a miss and its `no_avatar` placeholder is ignored; on Bitbucket the commit's `author.user` names the account and picture. Their image hosts are allowed.
6. The version is the package's own (`package.json`, or the running build's from `about`), and the strip shows only `v1.0.1`; the SPDX identifier stays in its tooltip and in Settings → About.

## Acceptance criteria

- None of the above is visible in a release build; GitLab authors with an uploaded picture show it.

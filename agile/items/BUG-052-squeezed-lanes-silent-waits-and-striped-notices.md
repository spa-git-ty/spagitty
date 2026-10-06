<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-052 — Squeezed lanes, silent waits and striped notices

**Status:** Fixed — merged into `main`; the manual sweep is not yet run.
**Branch:** `bugfix/BUG-052-squeezed-lanes-silent-waits-and-striped-notices`
**Screens:** 1A, all, chrome.
**Raised by:** the author, 2026-10-06, after BUG-051: the lanes were still close unless the column was dragged very wide; nothing animated while anything loaded, only words; and the notices, with a coloured stripe down one edge, did not match the glass design.

## Change

- **Lanes**: the comfortable graph compresses no tighter than 22px (was 14), close to its 26px rest; deeper lanes meet at the column's edge until it is dragged wider. Compact keeps its own floor of 14.
- **Loading**: `Loader.svelte`, the brand's three strands weaving under a glass orb, replaces every *Reading…* text — in the middle of an empty pane, and as small strands in a header. Buttons take `busy` and show the strands while their action runs (Merger's writes, Mark resolved). Reduced motion stills them.
- **Notices** are glass ornaments, the kind shown by a filled circle in its colour — a tick for done, an exclamation for a failure — with no stripe.

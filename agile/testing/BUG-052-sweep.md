<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-052 — Manual sweep

**Item:** [`agile/items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md`](../items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG052-01 | A busy repository | 1. Graph at the default width | Lanes about as far apart as GitKraken's | P1 | |
| SWEEP-BUG052-02 | Any repository | 1. Open screens while they read | The strands animate where the words were | P1 | |
| SWEEP-BUG052-03 | Any | 1. An action that succeeds, one that fails | Glass notices with a green tick or a red exclamation, no stripe | P1 | |
| SWEEP-BUG052-04 | Reduced motion on | 1. As 02 | The strands are still | P3 | |

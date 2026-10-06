<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-049 — Manual sweep

**Item:** [`agile/items/BUG-049-small-monospace-text-ignores-the-reading-font.md`](../items/BUG-049-small-monospace-text-ignores-the-reading-font.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG049-01 | Release build on Windows; Reading left at its defaults; a freshly cloned repository | 1. Open Reflog | `HEAD@{0}` and the id are in Atkinson Hyperlegible Mono; "created at 8103b44" is one line, "created at" in the interface face | P1 | |
| SWEEP-BUG049-02 | As 01 | 1. Look at the graph's dates, the Tags count and the footer's version | All in Atkinson Hyperlegible Mono; none in Consolas | P1 | |
| SWEEP-BUG049-03 | As 01 | 1. Settings › Reading › JetBrains Mono 2. Back to Reflog | The ids are in JetBrains Mono | P2 | |
| SWEEP-BUG049-04 | As 01 | 1. Settings › Reading › Lexend | Diffs are in Lexend; ids stay in Atkinson Hyperlegible Mono | P2 | |
| SWEEP-BUG049-05 | As 01 | 1. Settings › Reading › OpenDyslexic Mono 2. Reflog, Branches, Graph | No id is cut off without an ellipsis, and no row wraps | P2 | |

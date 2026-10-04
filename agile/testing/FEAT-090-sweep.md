<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-090 — Manual sweep

**Item:** [`agile/items/FEAT-090-settings-reading.md`](../items/FEAT-090-settings-reading.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT090-01 | — | 1. Settings → Reading | Every chip in its own face; the preview set in the reading set, calm, changed words marked | P1 | Pass, 2026-10-04, dev server in headless Chrome |
| SWEEP-FEAT090-02 | Offline | 1. Settings → Reading 2. Choose each code font | Each renders in its own face | P1 | |
| SWEEP-FEAT090-03 | A commit with changes | 1. Choose OpenDyslexic Mono, 18px 2. Diff, Working copy, File history | All three follow | P1 | |
| SWEEP-FEAT090-04 | As 03 | 1. Classic 2. Highlight changed words off | Full-strength rows; no word marks | P2 | |
| SWEEP-FEAT090-05 | As 03 | 1. Zoom to 150% | Code grows with the zoom | P2 | |
| SWEEP-FEAT090-06 | Light and dark | 1. Settings → Reading in each | The preview reads in both | P2 | |

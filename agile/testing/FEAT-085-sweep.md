<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-085 — Manual sweep

**Item:** [`agile/items/FEAT-085-a-new-mark-palette-and-wordmark.md`](../items/FEAT-085-a-new-mark-palette-and-wordmark.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT085-01 | Windows build | 1. Look at the taskbar, the window's icon and the installer | The new mark, clear at 16 and 32 px | P1 | Pass in part, 2026-09-30: the window and taskbar icon in the Windows release build; the installer not built |
| SWEEP-FEAT085-02 | macOS build | 1. Dock, Finder, menu bar | The new mark; the menu bar mark is strands only and follows the bar's colour | P1 | |
| SWEEP-FEAT085-03 | Linux build | 1. Launcher and tray | The new mark | P2 | |
| SWEEP-FEAT085-04 | No repository open | 1. Look at the title row | The mark and "spagitty", "git" in tomato | P1 | Pass, 2026-09-30, Windows release build: mark and wordmark centred, set in Sora, git in #b8321f |
| SWEEP-FEAT085-05 | No repositories known | 1. All repositories | The mark and the wordmark over the empty state | P2 | |
| SWEEP-FEAT085-06 | — | 1. Open `assets/brand/preview.html` in a browser, light and dark | Every image present; the lockups sit on one centreline | P2 | |
| SWEEP-FEAT085-07 | GitHub | 1. The README | The banner reads on GitHub's dark and light pages | P3 | |

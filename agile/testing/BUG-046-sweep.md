<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-046 — Manual sweep

**Item:** [`agile/items/BUG-046-a-console-window-opens-for-git.md`](../items/BUG-046-a-console-window-opens-for-git.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG046-01 | Windows release build; a repository with a pull request | 1. Review 2. Start review | The room opens; no console window appears | P1 | |
| SWEEP-BUG046-02 | Windows release build | 1. Fetch 2. Stage a file 3. Commit | No console window appears at any step | P1 | |
| SWEEP-BUG046-03 | Windows release build; a difftool configured | 1. Open a file in the external diff tool | The tool opens; no console window behind it | P2 | |

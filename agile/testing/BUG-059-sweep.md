<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-059 — Sweep

**Item:** [BUG-059](../items/BUG-059-windows-watcher-never-sees-refs.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG059-01 | Windows 11, a repository open | Commit from a terminal | The graph re-walks within a second; the commit count follows | P1 | Not run. Needs Windows. |
| SWEEP-BUG059-02 | Windows 11, a repository open, idle | Listen to `repo-changed` from the webview for 10 s | No events | P1 | Not run. Needs Windows. |
| SWEEP-BUG059-03 | Windows 11 | Save a tracked file in an editor; then touch an ignored one | The rail count follows the first and not the second | P2 | Not run. Needs Windows. |

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Sweep

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG063-01 | A pull request the host reports as conflicting (`maxmya/trattoria-demo#3`) | Look at Review's inbox, the preview and the room; then Pull requests | It is marked in each; *Open in Merger* offers *Resolve the conflicts here* | P1 | Not run. |
| SWEEP-BUG063-02 | The same, after the branch is resolved, pushed and the list refreshed | Look again | The marks are gone | P1 | Not run. |

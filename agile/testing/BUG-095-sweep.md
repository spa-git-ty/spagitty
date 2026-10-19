<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-095 — Sweep

**Item:** [BUG-095](../items/BUG-095-the-terminal-s-plus-says-no-server-running.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG095-01 | Windows, psmux in use; Terminal open on a repository | Type `exit` in the only window, then press `+` | A shell comes back in the same session; no *no server running* | Not run; the author is checking it |
| SWEEP-BUG095-02 | Same, with the session running | Press `+` | A second window appears in the header | Not run |
| SWEEP-BUG095-03 | macOS or Linux, tmux in use | Repeat 01 | The same | Not run |

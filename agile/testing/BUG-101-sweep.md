<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-101 — Sweep

**Item:** [BUG-101](../items/BUG-101-every-edit-in-calls-says-could-not-save.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG101-01 | A collection in Calls | Type in a parameter, a header and the URL, pausing between | No *Could not save* notification; the file in `.spagitty/calls/` has the edits | Not run; the author does the sweep |
| SWEEP-BUG101-02 | As above | Close and reopen the repository | The edits are still there | Not run |

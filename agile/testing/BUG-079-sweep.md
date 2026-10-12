<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-079 — Sweep

**Item:** [BUG-079](../items/BUG-079-all-resolved-is-drawn-as-a-button.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG079-01 | A repository stopped on a conflict, in Conflicts | Resolve every conflict | The pill's last place reads *All resolved* in green with a tick. The pointer does not change over it and it has no hover | P1 | Not run. |
| SWEEP-BUG079-02 | As above | Press Tab through the pill | *All resolved* is never focused | P2 | Not run. |
| SWEEP-BUG079-03 | As above | Press **Cancel** on one conflict | The last place is the **Next unresolved** button again | P2 | Not run. |
| SWEEP-BUG079-04 | A merge with conflicts, in Merger | Resolve every conflict | As 01 | P2 | Not run. |

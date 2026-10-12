<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-125 — Sweep

**Item:** [FEAT-125](../items/FEAT-125-a-choice-on-a-conflict-can-be-cancelled-and-pick-lines-starts-empty.md)

None of these rows has been run. The behaviour is covered by mounted tests; nothing was looked at in a browser or in the application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT125-01 | Two branches that conflict, opened in Merger's resolver | Press *Take A* on a conflict, then *Cancel* at the end of the same row | The conflict is *Unresolved* again, the result column says *Choose what goes here*, the count goes down by one, and *Cancel* is gone | P1 | Not run. |
| SWEEP-FEAT125-02 | As above | Press *Pick lines* | The boxes show with none ticked. The conflict is still *Unresolved*, every line is at full strength, and the result column says *Tick the lines to keep* | P1 | Not run. |
| SWEEP-FEAT125-03 | As above | Tick one line, then untick it | Ticked: *Picked lines*, the line is in the result, the others fade. Unticked: *Unresolved* again | P1 | Not run. |
| SWEEP-FEAT125-04 | As above, every other conflict resolved | Press *Pick lines* on the last and tick nothing | *Complete merge* stays disabled and says one is left | P1 | Not run. |
| SWEEP-FEAT125-05 | A file with three conflicts | Press *All from A*, then *Cancel all* | All three are unresolved; *Cancel all* is gone | P1 | Not run. |
| SWEEP-FEAT125-06 | As above, one conflict edited by hand | Press *Cancel all* | It asks first and says how many hand edits go; *Cancel them* clears all, the other button keeps them | P2 | Not run. |
| SWEEP-FEAT125-07 | A merge stopped on conflicts, opened in Conflicts | Repeat 01, 02 and 05 | The same; *Mark resolved* is disabled while a pick has nothing ticked | P2 | Not run. |
| SWEEP-FEAT125-08 | An agent proposing a resolution in Merger | Press *Pick lines* on its conflict, tick nothing, press *Cancel* | The proposal is still offered, with *Accept* | P2 | Not run. |

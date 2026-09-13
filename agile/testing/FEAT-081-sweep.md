<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-081 — Manual sweep

**Item:** [`agile/items/FEAT-081-the-graph-resizes-like-a-hand-moves.md`](../items/FEAT-081-the-graph-resizes-like-a-hand-moves.md)

Run in the Tauri app, on a repository with at least ten lanes in view, at a
window near 1572 × 910 so the result can be compared with the reference
recording. Match the reference's theme and zoom, and prefer a repository with a
similar staircase of lanes, so the comparison is about geometry.

Not run yet. The first pass was marked Done with every row below empty; the
review reopened it, and a recording of the second pass reopened it again. Fill
each Pass/Fail with what was observed, not with what the tests say.

Rows SWEEP-011 to SWEEP-014 use **Flea** (`appshelf-packages`), which
reproduces both the fold and the avatar defects and has a known GitHub
association for its `GM` commits. Start them with the avatar cache empty
(`~/.cache/<app id>/avatars` removed) and real pictures on in Settings.

| Ticket | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- |
| SWEEP-001 | Drag the divider between Graph and Commit Message slowly from wide to the 40px minimum and back | The boundary stays under the pointer. Lanes that fit do not move. When the boundary reaches a lane, its track and its node fold onto the boundary together; no node is ever without its lane, and nothing is cut off. At the minimum every path and node is on lane 0. Nothing snaps sideways. Widening releases lanes to where they were, deepest last. Portraits stay round and full size. | High | |
| SWEEP-002 | Same, fast, with several reversals, recorded at 60fps | No blank canvas frame, no header/body misalignment in any frame, no bounce after release. Report the recording's dropped-frame count. | High | |
| SWEEP-003 | Release the drag outside the window; separately, Alt-Tab mid-drag | The drag ends. Moving the pointer afterwards does not resize anything. | High | |
| SWEEP-004 | Select a commit, scroll halfway, then drag the column narrow and wide | Selection, detail panel and vertical scroll are unchanged. | High | |
| SWEEP-005 | Drag to the narrowest, restart the app | The narrow width is remembered for that repository. Double-click the divider: the column sizes itself again. | Medium | |
| SWEEP-006 | At the narrowest width | The header shows the graph icon, hovering it says "Graph", and the divider still drags. | Medium | |
| SWEEP-007 | Rest the pointer on a commit subject with a body and trailers | After about half a second the whole message appears with its paragraph breaks. The detail panel does not change. Move quickly to another row: the old message never appears over the new row. | High | |
| SWEEP-008 | Hover a bare commit on a topic branch, then one on main | Each row alone shows its branch name faintly in the gutter, not as a chip; nothing else on screen dims. Check the name against `git name-rev <sha>`. | Medium | |
| SWEEP-009 | Repeat SWEEP-001 at 150% interface zoom | Same as SWEEP-001; the icon threshold scales with zoom. | Low | |
| SWEEP-010 | Record a slow drag from wide to narrow beside the reference, frames at 0s and 4.5s in each | In both apps the leading tracks keep their screen x and spacing between the two frames, and the lanes past the boundary are folded onto it in both. | High | |
| SWEEP-011 | Flea: open the graph and wait for identities to settle | The repeated `GM` circles become the real `thisisgm` picture. The graph node, the Author column and the commit detail all show that same picture for that author. | High | |
| SWEEP-012 | Flea: scroll between shallow and deep parts of history | Ordinary commit circles keep one diameter throughout; only merge dots are smaller. The Author column and detail marks are the node's size. | High | |
| SWEEP-013 | Flea: drag wide to minimum slowly, reverse quickly, release outside the header | As SWEEP-001: fitting lanes still until reached, paths and nodes fold together, lane 0 at minimum, expanding restores the graph. | High | |
| SWEEP-014 | At wide, intermediate and minimum widths: hover nodes, look at a stash mark, and hover a bare commit for its ghost path | Hover targets sit on the drawn circles, the stash diamond sits beside its folded node, the ghost path joins the folded nodes. | High | |
| SWEEP-015 | Disconnect the network, clear the avatar cache, open Flea; reconnect after a few minutes and keep the app open | Initials while offline; pictures appear within about ten minutes of reconnecting without restarting, rather than staying initials for the session or for 90 days. | Medium | |

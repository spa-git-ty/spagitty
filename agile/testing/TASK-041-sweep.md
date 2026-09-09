<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-041 — Manual sweep

**Item:** [`agile/items/TASK-041-the-frame-stops-competing-with-the-work.md`](../items/TASK-041-the-frame-stops-competing-with-the-work.md)

Every ticket here is a look at a real window, because every one of them is a
question the tests cannot ask: the tests know the numbers changed and by how
much, and none of them knows whether the result reads better.

**Take a before capture first.** Build `0.7.0`, screenshot the Graph screen at
1280×800 with the detail panel open on a repository with three lanes, then
build this branch and take the same shot. Same size, same theme, same
repository, same selected commit. Everything below is easier to judge against
that pair than on its own.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Any repository | Look at the toolbar | Icon and label both fully visible, with air above and below. It is 40px rather than 50; if anything is clipped or the row feels cramped, that is the ticket failing and the number is the fix. | High | |
| SWEEP-002 | The same, zoom at 150% | Look again | 60px, with the controls scaled to match. A 40px bar with big controls in it means the metric stopped scaling. | High | |
| SWEEP-003 | Rail expanded | Look at it without reading the labels | Four blocks: the Farm alone, then three runs under quiet headings. The question is whether the eye can find "the screens I use all day" without reading — that is the whole ticket. | High | |
| SWEEP-004 | Rail collapsed | Look at it | Dividers where the headings were, no text. Then expand it: every row is where it was before this change. Nothing has moved. | High | |
| SWEEP-005 | A machine with no stored panel widths (fresh profile) | Open at 1280 wide, then at 1600 wide | The 1280 opens with a visibly narrower rail and detail panel. Compare the room left for commit subjects against the before capture. Then drag the rail, restart at a different size: **the dragged width is what comes back**. | High | |
| SWEEP-006 | A repository two or three lanes deep, Graph screen | Settings → Appearance → Graph → **Compact** | The lane column narrows by roughly 77px and the commit subjects get it. Nodes become marks rather than faces. Lanes stay followable from row to row — if they read as one band, the pitch is too tight and that is the finding. | High | |
| SWEEP-007 | The same, on `git/git` or another deep history | Switch densities both ways | Compact still compresses past its own cap rather than folding lanes on top of each other, and no node is drawn outside the column. Switch back and forth several times: nothing accumulates. | High | |
| SWEEP-008 | Compact, with author pictures on | Look at the graph, then open a commit's detail | The node is a mark; the **detail panel still shows the real portrait**. That is the trade the item describes, and the ticket is whether it feels like a trade or like a loss. | High | |
| SWEEP-009 | Compact, then drag the graph column narrower | Watch the nodes | They do **not** change size with the drag. Node size follows how deep the history is, which FEAT-039 decided deliberately and this task nearly reversed. | Medium | |
| SWEEP-010 | Reduced motion enabled in the desktop | Click between five screens in the rail | Nothing slides. Before this change every navigation animated regardless of the preference, and the comment in the layout claimed otherwise. Turn the preference off and the slide comes back. | High | |
| SWEEP-011 | Window narrowed to ~900px, tiled | Look at the toolbar and the rail | Labels drop, carets and icons stay, group headings are gone, nothing overlaps. | Medium | |
| SWEEP-012 | Any repository, both densities | Scroll the graph hard | No flicker, no jump in the message column, no lane misalignment between the canvas and the rows. Both densities, because the canvas and the row cells compute their geometry separately and this task gave them a new shared input. | High | |
| SWEEP-013 | Compact, text size at 130% | Look at the graph | Row pitch grows with the text; lane geometry does not, and should not. Nodes stay inside their rows. | Medium | |

## Before/after captures the review asked for

At **1280×800**, **1440×900** and a narrow **~900px** tiled window; at 100%,
125% and 150%; on one light and one dark theme; on a small history and a complex
one. For each: Graph with detail open, Working copy dirty, and a Diff. Identical
repository and selection in each pair.

These are the review's stage-3 evidence and they cannot be produced here. What
this task changes is the geometry those captures would be of.

## Negative paths this sweep deliberately covers

- **SWEEP-005**'s second half is the one that can go wrong quietly: adaptive
  defaults that overwrote a dragged width would look correct on a fresh profile
  and be infuriating on a used one.
- **SWEEP-009** is a regression this task actually introduced once and two
  existing tests caught. It is here because a test catching it is not evidence
  that the shipped build is right.
- **SWEEP-012** is where a shared geometry model breaks: the canvas and the rows
  both take the density now, and they must take the same one.

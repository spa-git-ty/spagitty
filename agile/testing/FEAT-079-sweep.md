<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-079 — Manual sweep

**Item:** [`agile/items/FEAT-079-a-node-that-says-who.md`](../items/FEAT-079-a-node-that-says-who.md)

Two of these matter more than the rest. **SWEEP-002 and SWEEP-003 watch what
actually leaves the machine**, with a packet capture, because the automated
tests assert what the code *decides* to send and nobody has yet watched it send
it. **SWEEP-009** is the one that catches this feature being expensive.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A repository with commits by several people, at least one of whom has a GitHub account and a picture | Open it on Graph, with the preference on and the cache empty | Within a second or two, nodes change from coloured blobs to real pictures. The ones that stay blobs are addresses with no picture, which is normal. Nothing flickers, and the list scrolls the whole time. | High | |
| SWEEP-002 | The same, cache emptied (`~/.cache/dev.spagitty.app/avatars` or the platform equivalent), and `tcpdump`/Wireshark or `mitmproxy` running | Open the repository and watch the traffic | Requests to `avatars.githubusercontent.com`, `github.com` and `gravatar.com` and **nowhere else**. No API host, no `Authorization` header on any of them. | High | |
| SWEEP-003 | The same capture | Read the Gravatar request lines | The path carries a 64-character hex hash, `s=96` and `d=404`. **No email address appears anywhere in any request**, in any header, in any form. | High | |
| SWEEP-004 | A repository whose authors commit as `someone@localhost` or with a dotless host | Open it with the capture running | No request is made for those addresses at all. They draw the generated face. | High | |
| SWEEP-005 | Any repository on Graph | Hover the circle on a commit row. Then hover the lane line just beside it, and the empty lane column further right | Over the circle: a tooltip with the name, the address, and `@handle` where there is one. Beside it and elsewhere: nothing. A tooltip that appears while the pointer is over a *different* row's lane is the failure this ticket is for. | High | |
| SWEEP-006 | A repository with merge commits | Hover a merge node — the small plain dot | No author tooltip. A merge is not one person's work, which is why it has no face. | Medium | |
| SWEEP-007 | The same repository, Author column shown | Compare each row's node with its Author column face | The same face, on every row. If a picture is on one and a blob on the other, the two resolutions have drifted. | Medium | |
| SWEEP-008 | A repository open, pictures loaded | Settings → Behaviour, turn "Show authors' real pictures" off. Return to Graph | Every node is a generated face again, immediately — not after a restart. Then check the cache directory: it is gone. | High | |
| SWEEP-009 | Preference back on, cache emptied, capture running, a repository with **many** authors — `torvalds/linux` if one is to hand | Open it, then fling the graph up and down for thirty seconds | One request per distinct author, and **no further requests while scrolling**. A burst that grows with the scrolling means the canvas is asking rather than the rows, which is the mistake this design exists to avoid. | High | |
| SWEEP-010 | The same repository, second launch | Close Spagitty, reopen it, open the same repository, with the capture running | **No requests at all.** Everything comes off the disk cache. | High | |
| SWEEP-011 | A machine with no route to the internet | Open a repository on Graph | Generated faces, no error, no notice, no stall. Scrolling is as fast as it ever was. This is a supported state, not a degraded one. | High | |
| SWEEP-012 | An author whose picture has changed at the host within the last 30 days | Open the repository | The old picture. **Correct** — the cache is fresh for thirty days, and this ticket exists so that "it did not update" is a known fact rather than a bug report. Emptying the cache via SWEEP-008 forces it. | Low | |
| SWEEP-013 | Any repository, zoom changed with `Ctrl` `+` and `Ctrl` `−` | Hover a node at 70% and at 150% | The tooltip appears over the circle at both, and the circle is where the pointer is. The target is computed from the zoomed geometry; a target that lags the drawing shows up here first. | Medium | |
| SWEEP-014 | A repository, graph column dragged much narrower than its default | Hover a node | Still over the circle. Node size follows the *depth*, not the drag (FEAT-046), and both the drawing and the target have to agree about that. | Medium | |

## Negative paths this sweep deliberately covers

- **SWEEP-003 and SWEEP-004** are the privacy claims on the item document,
  checked rather than argued. The item states plainly what leaves the machine;
  these are what make that statement a finding.
- **SWEEP-009 and SWEEP-010** are the cost. A feature that fetches per author is
  fine; one that fetches per author per frame is not, and the difference is
  invisible on a repository with three contributors — which is why the ticket
  names a repository with thousands.
- **SWEEP-011** covers the state this feature must never make worse. The graph
  worked offline before and has to work offline now, at the same speed.
- **SWEEP-012** is the honest one. A thirty-day cache means somebody's new
  picture does not appear, and that is a decision rather than a defect.
- **SWEEP-005, SWEEP-013 and SWEEP-014** are all one question — does the hover
  target sit where the circle is drawn — asked at the three settings that move
  the circle.

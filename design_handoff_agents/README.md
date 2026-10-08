# Design handoff — Spagitty 2.0: agents in Review and Merger

The spec is [`docs/proposals/agents-in-review-and-merge.md`](../docs/proposals/agents-in-review-and-merge.md). The screens show it. When they disagree, the spec wins on behaviour and the screens win on layout.

What 2.0.0 ships is [FEAT-115](../agile/items/FEAT-115-agents-in-review-and-merge.md). Where the item and the spec differ, the item is the authority: it defers undoing a landed merge and agent-handled rebase stops, which the spec describes.

In the repo, this folder sits beside `design_handoff_review/` and `design_handoff_merger/`.

## Contents

```
agents-in-review-and-merge.md   the feature spec (18 sections)
screens/                        PNG renders, 1440×900 @2x, dark theme
source/                         the editable artboards (.dc.html) + canvas.json
```

## Screens

| PNG | Source | Spec | Shows |
| --- | --- | --- | --- |
| `01-settings-agents` | `Main.dc.html` | §4 | Local agents found on this machine, API agents, per-job chips, defaults, this repo's rules card |
| `02-add-api-agent` | `AddApiAgent.dc.html` | §4, §11 | Provider, key (keychain), model + Test, jobs, token limits, what leaves the machine |
| `03-review-assign` | `ReviewAssign.dc.html` | §5, §8 | Inbox with *Start review ▾* open: pick agent and level |
| `04-review-room-waiting` | `ReviewRoomLive.dc.html` | §7, §8 | 1.3 room unchanged + agent findings inline + Agent tab, gate waiting for you |
| `05-review-room-reading` | `ReviewRoomLive.dc.html` (`moment: reading`) | §7, §8 | Same room while the agent is still reading |
| `06-merger-agent-resolving` | `MergerLive.dc.html` | §7, §9 | Resolver unchanged + agent proposal, fourth origin badge, Agent card stopped on an unsure conflict |
| `07-levels-and-gates` | `LevelsAndGates.dc.html` | §6 | Where each of the four levels stops, what always stops, what is never allowed |

## Reading a `.dc.html`

- Plain HTML inside `<x-dc>`. Inline `style` holds the real values.
- `{{name}}` holes are filled by `renderVals()` in the `<script type="text/x-dc">` block at the bottom. `<sc-for>` repeats, `<sc-if>` branches.
- Colour tokens are in `.t-dark` / `.t-light` in each file's `<helmet><style>`. The `theme` prop switches them; dark is the reference.
- `<a href="X.dc.html">` links artboards (e.g. *Add* → the API agent dialog).
- Data in the scripts (agents, PR #214, code lines, times) is illustration.

## Rules to keep

- **Fidelity** is structure, copy and colour meaning. Exact sizes and colours come from `metrics.ts` and `app.css`, not from these files.
- **No agent, no trace**: with no agent set up, Review and Merger render exactly as 1.3 (spec §2).
- **Agent identity**: colour `--agent` (dark `#86e3d2`, light `#075c52`) and a hexagon mark. In Merger the hexagon is the fourth origin beside A, B and ✎.
- **Agent output is human material**: findings are pending comments with an author; resolutions are resolver choices (spec §2).

## Notes

- PNGs were rendered offline, so Atkinson Hyperlegible fell back to system fonts. Use the app's fonts.
- Spec §18 lists open questions for the author. Settle them before implementing the parts they touch.

# Design handoff — Spagitty 3.0: the workbench and recipes

The spec is [`docs/proposals/workbench-and-recipes.md`](../docs/proposals/workbench-and-recipes.md). The screens show it. When they disagree, the spec wins on behaviour and the screens win on layout.

What 3.0.0 is to ship is five backlog items:

| Item | Screen |
| --- | --- |
| [FEAT-130](../agile/items/FEAT-130-a-terminal-on-the-rail.md) | Terminal (1T) |
| [FEAT-131](../agile/items/FEAT-131-files-any-revision-read-never-written.md) | Files (1U) |
| [FEAT-132](../agile/items/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line.md) | Investigate (1O) |
| [FEAT-133](../agile/items/FEAT-133-recipes-git-steps-in-blocks-run-with-an-agent.md) | Recipes (1V) |
| [FEAT-134](../agile/items/FEAT-134-pin-or-hide-what-is-on-the-rail.md) | Settings › Rail |

Where an item and the spec differ, the item is the authority. Take them in the spec's order (§12). FEAT-133 waits for the author's answer to the spec's question 1.

In the repository, this folder sits beside `design_handoff_agents/`, `design_handoff_review/` and `design_handoff_merger/`.

## Contents

```
screens/  PNG renders, 1440×900 @2x, dark theme
source/   the editable artboards (.dc.html) + canvas.json + support.js
```

Open any `source/*.dc.html` straight in a browser: `support.js` is a small stand-alone viewer that fills the template. Add `?theme=light` to the address for the light theme. The two interactive artboards work there too. `support.js` is not part of Spagitty and nothing in the app should import it.

## Artboards

PNGs in `screens/`, in this order: `01-terminal`, `02-files`, `03-investigate`, `04-recipe-builder`, `05-recipe-run`, `06-settings-rail`.

| Source | Item | Spec | Shows |
| --- | --- | --- | --- |
| `Main.dc.html` | FEAT-130 | §5 | Terminal: tabs, two tmux panes, Sessions with two farm agents, *Follow repository*, links in output |
| `Viewer.dc.html` | FEAT-131 | §6 | Files at `main`: tree, tabs, breadcrumb, tree-sitter colour, blame gutter, the peek on an agent-written line, the status line |
| `Investigator.dc.html` | FEAT-132 | §7 | Investigate a file: the life card (chart, events, authors), the event list, the selected rewrite |
| `Recipes.dc.html` | FEAT-133 | §8.1–8.3, §8.5 | The builder. **Interactive**: add a block by clicking it, move it up, remove it; the validator and the brief update live |
| `RecipeRun.dc.html` | FEAT-133 | §8.4 | A run stopped at a merge conflict: steps, the agent's output, the stop, the restore point |
| `SettingsRail.dc.html` | FEAT-134 | §9 | Settings › Rail. **Interactive**: Pin and Hide change the preview rail and its More menu |

## Reading a `.dc.html`

- **Markup**: plain HTML inside `<x-dc>`. Inline `style` holds the real values.
- **Data**: `{{name}}` holes are filled by `renderVals()` in the `<script type="text/x-dc">` block at the bottom. `<sc-for>` repeats and `<sc-if>` branches.
- **Colour**: the tokens are in `.t-dark` and `.t-light` in each file's `<helmet><style>`, and the `theme` prop switches them. Dark is the reference.
- **Links**: `<a href="X.dc.html">` links artboards. *Investigate line* goes to Investigate, and *Run* goes to the run.
- **Illustration**: the data in the scripts (commits, people, agents, times) is made up.

## Rules to keep

- **The chrome is context, not design.** The title bar, the rail, the toolbar and the status strip were drawn to match `TitleBar.svelte`, `NavRail.svelte`, `Toolbar.svelte` and `StatusStrip.svelte` as of 2.2.1. Do not change those components to match the artboards. The only chrome change in 3.0 is the rail's new rows and its More button (FEAT-134).
- **The artboards' rail is a person's choice, not the default.** Rebase, Stash, Tags, Reflog and All repositories are hidden there to show the More button. The default keeps every screen pinned (TASK-056).
- **Fidelity** means structure, copy and colour meaning. Exact sizes and colours come from `metrics.ts` and `app.css`.
- **Agent identity** is unchanged from 2.0: the `--agent` colour and the hexagon.
- **Recipes are calm**: neutral block cards with a small muted square per group. The author rejected a toy look (bright Scratch-like bricks).
- **Files cannot write.** It reads git objects at a revision ([spec §2](../docs/proposals/workbench-and-recipes.md#2-rules-that-shape-the-rest)). The *VIEW* in its status line is a fact, not a mode.

## Notes

- The artboards load Atkinson Hyperlegible from Google Fonts. The PNGs were rendered offline, so they use system fonts. Use the app's bundled fonts.
- The icons for Files, Recipes, Investigate and More are in each artboard's script. `icons.ts` already has `terminal`.

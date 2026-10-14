# Design handoff — Spagitty 3.1: issues, hooks, pipelines and releases

The spec is [`docs/proposals/forge-views.md`](../docs/proposals/forge-views.md). The screens show it. When they disagree, the spec wins on behaviour and the screens win on layout.

3.1.0 is four backlog items, to be recorded once the author answers the spec's §13:

| Item | Screen |
| --- | --- |
| FEAT-136 | Issues (1W) |
| FEAT-137 | Hooks (1X) — Runs and Hooks |
| FEAT-138 | Pipelines (1Y) — Preview and Live |
| FEAT-139 | Releases (1Z) |

Take them in the spec's order (§11). Nothing that writes to a host is built before §13 question 1 is answered.

In the repository, this folder sits beside `design_handoff_workbench/` and the other handoffs.

## Contents

```
screens/  PNG renders, 1440×900 @2x, dark theme
source/   the editable artboards (.dc.html) + canvas.json + support.js
```

Open any `source/*.dc.html` straight in a browser: `support.js` is the small stand-alone viewer from `design_handoff_workbench/` that fills the template. Add `?theme=light` to the address for the light theme. `support.js` is not part of Spagitty and nothing in the app should import it.

## Artboards

PNGs in `screens/`, in this order: `01-issues`, `02-hooks-run`, `03-hooks-edit`, `04-pipeline-preview`, `05-pipeline-live`, `06-releases`.

| Source | Item | Spec | Shows |
| --- | --- | --- | --- |
| `Main.dc.html` | FEAT-136 | §5 | Issues: host switch, filters, list, issue #214 with thread, timeline, details, *Start branch*, *Hand to agent* |
| `HooksRun.dc.html` | FEAT-137 | §6.1–6.2 | Runs: pre-commit at step 4 of 6, live log of *Tests · payments*, the restage notice, staged files, earlier runs |
| `HooksEdit.dc.html` | FEAT-137 | §6.3–6.5 | The builder. **Interactive**: pick *Tests · payments* or *Commit message rules* to switch the editor |
| `Pipeline.dc.html` | FEAT-138 | §7.1–7.2 | Preview of `ci.yml` for a push to main with the working copy |
| `PipelineLive.dc.html` | FEAT-138 | §7.3 | Live: watched branches and requests, run #1873, run strip, the running job's log |
| `Releases.dc.html` | FEAT-139 | §8 | Releases: list, draft v3.1.0, generated changelog, files |

## Reading a `.dc.html`

- **Markup**: plain HTML inside `<x-dc>`. Inline `style` holds the real values.
- **Data**: `{{name}}` holes are filled by `renderVals()` in the `<script type="text/x-dc">` block at the bottom. `<sc-for>` repeats and `<sc-if>` branches.
- **Colour**: the tokens are in `.t-dark` and `.t-light` in each file's `<helmet><style>`, and the `theme` prop switches them. Dark is the reference.
- **Links**: `<a href="X.dc.html">` links artboards: the Runs / Hooks and Preview / Live switches.
- **Illustration**: the data in the scripts (issues, people, runs, times, sizes) is made up.

## Rules to keep

- **The chrome is context, not design.** Title bar, rail, toolbar and status strip are drawn from the 3.0 handoff. Do not change `TitleBar.svelte`, `NavRail.svelte`, `Toolbar.svelte` or `StatusStrip.svelte` to match the artboards. The only chrome change in 3.1 is four rail rows.
- **Fidelity** means structure, copy and colour meaning. Exact sizes and colours come from `metrics.ts`, `app.css` and `themes.ts`.
- **Agent identity** is unchanged from 2.0: the `--agent` colour and the hexagon (*Hand to agent*).
- **Hook blocks are calm**, like Recipes: neutral cards with a small muted square per kind. Kind colours: format `--b-fmt`, tests `--b-test`, message `--b-msg`, script `--b-run` (defined in the Hooks artboards).
- **Status colours mean one thing everywhere:** `--ok` passed, `--danger` failed, `--warn` waiting for a person, `--lane-5` running, muted for skipped and queued. Skipped jobs also get a dashed border, so the state does not rest on colour alone.
- **Host names appear only where the person picks a host** (the switches, *Open on GitHub ↗*). Everything else uses Spagitty's words.
- **Never say live when it is polling.** The status strip says *updated N s ago*.

## Notes

- The artboards load Atkinson Hyperlegible from Google Fonts. The PNGs were rendered offline, so they use system fonts. Use the app's bundled fonts.
- The icons for Issues, Hooks, Pipelines and Releases are in each artboard's script (`ICONS`).
- `canvas.json` is the canvas layout the artboards were drawn on; the app does not read it.

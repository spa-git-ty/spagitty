<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-090 — Pipelines does not look like its artboards, and Preview takes half a minute

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged. Not looked at in the running application; the author is checking it.
**Screens:** 1Y (Pipelines · preview and live)
**Raised by:** the author, 2026-10-18, on 3.1.0: the Pipelines screens in `Pictures/Screenshots/bugs` ("recreate the ugly broken screen of pipelines").

## What happens

- **Preview** sat on "Reading workflows…" for a long time on `core-banking-services`, and the host switch did not appear until it finished.
- **Neither view is drawn as panes.** The file list, graph and job panel are text on the background, unlike `04-pipeline-preview.png`. Empty group headings (GITHUB ACTIONS with no files) show anyway.
- **Live uses half the window.** The run, graph and log end in the middle of the screen.
- **Live, GitLab:** the head reads "Pipeline #133194 #133194"; stages are drawn right to left when the run's file cannot be read; "Updated 0 s ago" never counts up.
- **Graph:** GitLab's stage order is drawn as an edge from every job to every job of the stage before; empty stages leave empty columns; columns are titled "Depth 1"; the app's global `.card` hover lift applies to job cards.
- **A remote or component include** turns every job "decided on the host", even a job whose own rules and the file's own `workflow:` rules decide it.

## Cause

- `pipelines::preview` found the current branch's upstream through `branches::list`, which counts ahead/behind for every branch: 27 s on a repository with 1050 branches (debug build). The store read the hosts only after the preview.
- `PipelinesScreen.svelte` used a `.pane` class that exists nowhere, and its root had no `flex: 1`, so Live shrank to its content.
- GitLab runs were named `Pipeline #N` and the number was written again; live stage order came from GitLab's newest-first job list; `age` was computed from a clock that only moved on refresh.
- `layout.ts` drew every `needs` entry, including the ones the model made up for stage order.

## Fix

- **Core:** `branches::current_upstream` reads one ref (0.2 s on the same repository). Jobs carry `implicit` when their `needs` stand for stage order. A job is decided locally despite an unread include when the file has its own `workflow:` and the job its own `rules`/`only`/`except`. When the workflow's own rules exclude the event, the reason says so. GitLab runs use `workflow: name` or "pipeline".
- **Screen:** rebuilt from the artboards as `PipelinesScreen` (head, view switch, errors, source panel), `PreviewView` and `LiveView`, with readable markup. Panes, the simulation bar with Stages/Needs, the legend with counts, one problem shown with the rest behind *N more*, the job panel with its reason tinted by state and numbered steps. Live: watched rows with a state dot, the latest run and its age; the run head with a state chip and elapsed time, the run strip, the graph, then the log. A one-second clock while Live is open drives "updated N s ago".
- **Graph:** `states.ts` gives one tone and one glyph per state for both hosts; empty columns close up; stage-order edges are not drawn (the columns show it); cards sit near the cards they need; GitLab opens in Stages, GitHub in Needs. Job cards are `.job`, not `.card`.
- **Store:** hosts and the preview are read at the same time.

## Tests

- Core: the include rule in `remote_data_is_opaque_and_local_escape_and_cycles_are_refused`.
- Frontend: stage order drawn as columns without the edge tangle; live stage order by creation; one state reads the same on both hosts.

## Not done

- **Run this job locally** stays out (spec §13, question 6).
- Not looked at in the built application. Checked with `bun run check`, vitest, clippy and the core tests.

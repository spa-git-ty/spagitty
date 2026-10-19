<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-141 — Starting a farm, one step at a time

**Status:** Done — on `release/3.2.0`; unmerged. Covered by Rust and mounted tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.2.0`.
**Screens:** 1Q.
**Designs:** [Home, starting a farm, and the notification center](../../design_handoff_home_farm_notifications/README.md), section 2 and `source/Farm*.dc.html`, `Steps.dc.html`, `SoFar.dc.html`.

## Change

The Farm's Setup page (FEAT-111) was one long form: the crew, the goal half way
down, the rules, and only then a way to start. The thing that changes every time
was buried under the things that almost never change. Starting a farm becomes a
wizard of four steps — Goal, Start from, Crew & rules, Start — and only a one-line
goal is required. Planning, the board, a task's journey and wrap up do not
change.

## Acceptance criteria

- **The frame:** the question on the left (at most 780 px wide), `Step n of 4`
  and a large headline; **Your farm** on the right, filling in as the person
  goes (Goal, Start from, Crew, Rules, Tasks), with the remembered crew and
  rules dimmed and tagged `last time` until step 3; a glass bar at the bottom
  with Back, Skip where a step is optional, and the primary Continue. Enter
  continues. While the wizard is open the header shows its four steps instead
  of the phase tracker.
- **Step 1, Goal:** one large field and an optional *Anything the agents should
  know*. **Or pick up a past farm** lists this repository's earlier farms: a
  stopped one offers Resume, a finished one *Use as a starting point* (its goal,
  context and rules). *How a farm runs* is a closed fold. *Skip to start* goes to
  step 4 with the remembered crew and rules.
- **Step 2, Start from (optional):** any mix of issues (a searchable list with
  checkboxes, from the Issues screen's providers), briefs (one or more `.md`
  files from the repository), and a branch in progress (the farm's tasks start
  from it and merge back into it).
- **Step 3, Crew & rules:** "Same crew and rules as last time?", naming the farm
  they came from. Crew tiles (broken and missing agents as Setup showed them),
  the five autonomy levels as one segmented control with the *You come in* line,
  checks, agents at once, tries, *Where each task works* (a worktree per task,
  or a slice per task once FEAT-143 is built), the `AGENTS.md` line, and
  CodeRabbit and leftover worktrees behind *More rules*.
- **Remembered per repository:** the crew and rules are saved in `.spagitty/`
  when the farm starts and read back the next time.
- **Step 4, Start:** three ways to make the tasks — plan it with an agent (with a
  planner picker), one task per issue (only when issues were picked; the drafts
  go straight to Plan review), or an empty plan. The primary button says what it
  will do. **The planner gets** lists exactly what goes into the planning
  prompt: the goal and notes, each issue, each brief, `AGENTS.md` when there is
  one, and a map of the repository.
- **The planning prompt** carries the issues, briefs and branch the person
  picked, and a map of the repository's top-level layout.
- **After Setup** the tracker reads Plan → Build → Wrap up.
- **Tests:** the wizard's state (what each step shows, what Skip and Continue do,
  the button's words, the planner chips) is pure and unit-tested; the Rust side
  has tests for the remembered setup, past farms and the prompt's new sections;
  the screen mounts through all four steps.

## Not in this item

- Dropping a file onto the window as a brief. Tauri takes file drops before the
  page sees them; the picker reads files from the repository and from disk.

## Decided on the way

The handoff's open question 1 is answered: with Crew folded into the wizard the
tracker after it is Plan → Build → Wrap up.

## What was built

- **The goal's sources** in `spagitty-farm`'s `model/goal.rs` (`GoalSource`: issue, brief, branch), carried into the planning prompt's `STARTING POINTS` with a `REPOSITORY MAP` built from the index (`context::repository_map`). A farm from a branch gives its tasks that branch as their start and merge target.
- **Remembered setup and past farms** in `model/setup.rs` and `persistence/store.rs`: `setup.json`, and `past/<farm>.json` for a replaced farm that had work in it (the newest twenty are kept). The service's `create_from`, `remember`, `remembered`, `past` and `resume`; the Tauri commands `farm_remembered`, `farm_remember`, `farm_past`, `farm_resume`, `farm_briefs` and `farm_brief` (Markdown, 256 KB at most).
- **The wizard** in `src/lib/farm/wizard.ts` and `src/lib/farm/components/wizard/`, the crew tiles in `CrewTiles.svelte`, and the segmented autonomy and *Where each task works* in `RulesEditor.svelte`, which the Rules sheet shares. `Setup.svelte` is gone.
- **The farm page** shows the wizard's steps in the header while it is open and Plan → Build → Wrap up after; `?start=1` opens the wizard, which Home's *New farm* uses.

## Differences from the handoff

- **Step 3's headline** is "Same crew and rules as last time?" only when something was remembered; a repository's first farm asks "Who works on it, and by which rules?".
- **Agents at once and Tries** keep the existing segmented choices rather than − and + steppers.
- **A brief** is picked from the repository's Markdown files or with the file picker; a file dropped on the window is not read (see above).
- **Resume** puts the stopped farm back as it was; the farm it replaces is kept with the others.

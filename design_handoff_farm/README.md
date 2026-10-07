<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Handoff: Farm — a farm you can follow at a glance

## Why

Raised by the author, 2026-10-07. The Farm screen (1Q, FEAT-073) works, but it
is hard to use. Looking at it, you cannot tell:

- which step the farm is on, and what comes next;
- how a task actually moves from an idea to a merge;
- what each agent is doing right now;
- what is done, what is still left, and what is waiting on *you*.

The decision: **redesign the Farm from the concept up**, in the spatial shell
(FEAT-082). Keep the backend (`crates/spagitty-farm`, `src/lib/farm/api.ts`,
`store.svelte.ts`, `types.ts`). Replace the screen and most of its components.

The concept in one line: **a farm is a journey with five phases, and every
screen says which phase you are in.**

```
Crew → Goal → Plan → Build → Wrap up
```

## About the design files

| File | What it is |
| --- | --- |
| `screens/*.png` | Every screen, rendered at 1440 × 900 and 2× density. Dark theme unless the name says `light`. **These are the spec.** |
| `screens/anim-*.gif` | The three screens that move: the planner streaming, the board with its live loaders (with *Land it* and *Pause* clicked), and a working task. |
| `*.dc.html` + `support.js` | The Design Component sources, in the same format as `design_handoff_review/`. Open them to read exact copy, structure and behaviour. Treat the markup as a layout spec only. Throw away the DC runtime. |

Build it with the repo's own components (`Btn`, `Chip`, `Icon`, `Splitter`,
`Loader`, `.card`, `.ornament`) and tokens. The goal ("Gitea support for pull
requests"), tasks, times, commit hashes and counts are all made up.

**Fidelity: high** for structure, copy, and the meaning of every colour. Take
exact pixel values from `metrics.ts` and `app.css`, not from the images. The
scrollbars in the images already follow the `app.css` scrollbar block.

> **Do not copy the rail from the images.** It was taken from
> `design_handoff_review/`. `src/lib/nav.ts` is the authority. Farm stays where
> it is: `1Q`, `/farm`, group `farm`, the top slot. Nothing in this handoff
> changes the rail. The only rail change is the dot (see *Needs you*).

## The screens

| # | Screen | Image | What it is for |
| --- | --- | --- | --- |
| 1 | Setup | `01-setup.png`, `11-setup-light.png` | No farm yet. Crew, goal, and rules on one page. |
| 2 | Planning | `02-planning.png`, `anim-02-planning.gif` | The planner is running. |
| 3 | Plan review | `03-plan-review.png`, `03b-…left-out.png` | The proposed tasks, grouped in waves, before anything runs. |
| 4 | Building (home) | `04-building.png`, `04b-…`, `04c-…`, `12-…light.png`, `anim-04-building.gif` | The farm's home while it runs. |
| 5 | Wrap up | `05-wrap-up.png` | Everything landed. A summary and clean-up. |
| 6 | Task | `06-task-working.png`, `07-task-ready-to-land.png`, `08-task-stuck.png`, `13-task-light.png`, `anim-06-task-working.gif` | One task's journey. |
| 7 | Crew | `09-crew.png` | The agents on this machine. |
| 8 | Activity | `10-activity.png`, `14-activity-light.png` | Who did what, and when. |

### Which screen shows

The screen follows the farm's state. There is no tab to pick it.

| State | Shows |
| --- | --- |
| No repository open | Keep today's "A farm lives in a repository" empty state. |
| `farm === null` | **Setup** |
| A planning run is in flight (`farmStore.planningRun`) | **Planning** |
| Planning finished and drafts exist (`farmStore.drafts.length > 0`), nothing ready yet | **Plan review** |
| Any task beyond draft | **Building**, with the `Board · Crew · Activity` chips |
| `farm.status === 'completed'` | **Wrap up** |
| `/farm?task=T-04` | **Task** (deep link; the board's cards lead here) |

`Crew` and `Activity` are views of Building, kept in `?pane=` the way `pane`
works today. Settings move into the **Rules** sheet (see Building).

## Shared pieces

### The phase tracker (`FarmPhases.svelte`, new)

Five steps in the header, right-aligned on every Farm screen: Crew, Goal, Plan,
Build, Wrap up.

- Each step has a dot and a label, plus a one-line sub-label underneath. For
  example: `4 agents`, `Gitea support`, `9 tasks`, `7 to go`.
- **Done** steps: an `--ok` check. **Current** step: an `--accent` ring that
  breathes (the existing `breathe`/halo motion). **Upcoming** steps: a number
  in `--muted`.
- The bar between two steps turns `--ok` once the step on its left is done.
- Phase = Crew/Goal while there is no farm. Plan while planning or reviewing
  drafts. Build while any task is not terminal. Wrap up once the farm is
  `completed`.

### Agent identity

Each agent has one colour, used everywhere: its badge, its timeline bars, the
halo around it.

| Provider | Colour | Badge |
| --- | --- | --- |
| `claudeCode` | `--lane-1` | `CC` |
| `codex` | `--lane-5` | `CX` |
| `cursor` | `--lane-4` | `CU` |
| `ohMyPi` | `--lane-3` | `PI` |
| `custom` | `--muted` | first two letters of `displayName` |

The badge is a two-letter monogram in a tinted circle. **Never draw a vendor's
logo.** An agent that is running gets a slow halo (`box-shadow` pulse) on its
badge.

### Loaders

Use `Loader.svelte` as it is. Do not add a new spinner.

- `size="inline"` (the three strands) goes next to anything moving: a working
  card, a crew row, a check that is running, the current step of a task.
- `size="pane"` (the weave orb) is used only on Planning, scaled up (132 px
  orb).

### The task track

Six short bars at the bottom of every task card. They are the same six steps as
the task stepper:

`Queued · Worktree · Working · Checks · Review · Landed`

Bar colours: done `--ok`, current `--accent` (glowing), failed `--danger`,
waiting `--warn` at 70%, not reached `--soft`. Give the track a `title` that
names the steps. One pure function maps a `Task` to its six bar states; put it
in `describe.ts` and test it.

| `TaskStatus` | Track |
| --- | --- |
| `draft`, `ready`, `waiting`, `blocked` (by dependency) | `w o o o o o` |
| `assigned` | `d n o o o o` |
| `running` | `d d n o o o` |
| `verification` | `d d d n o o` |
| `review` | `d d d d n o` |
| verified and approved, waiting for a person to merge | `d d d d d y` (`y` = an `--ok` "your turn" step) |
| `done` | `d d d d d d` |
| `failed` / `blocked` after max attempts | the failing step is `s` |
| `cancelled` | all `--soft`, card at 60% opacity |

## 1 · Setup

There is no farm yet. One scrolling column (max 1020 px wide) holding:

1. **A headline and one sentence** saying what a farm does. Under them, a
   six-chip flow that explains how a farm runs: `Goal → Plan → Task in its own
   worktree → Your checks → Another agent reviews → Lands on main`. This strip
   is the answer to "how is this running".
2. **Your crew.** One tile per detected agent: badge, name, path, ready state,
   what it is good at (from `capabilities`), and a checkbox for "on the crew".
   Includes `Look again` (`detectAgents`) and `Add a CLI agent`
   (`saveAgent` with `provider: 'custom'`). A broken agent shows its reason. A
   missing agent goes in a single quiet line underneath, not a tile.
3. **The goal.** The title and the notes (`create(title, description)`).
4. **The rules.**
   - The five `AUTONOMY_LEVELS` as a row of five cards. Under them, one live
     line that says **where you come in**:

     | Level | "You come in:" |
     | --- | --- |
     | manual | you start every task and every merge yourself. |
     | assisted | after each task, before it is reviewed or merged. |
     | semiAuto | only to approve merges, and when a task is stuck. |
     | auto | only when a task is stuck after its tries. |
     | yolo | at the end. Nothing waits for you. |

   - The checks (the `verification` commands), shown as `$ cmd` lines.
   - Agents at once (`maxParallel`) and tries before a task needs you
     (`maxAttempts`).
   - A line saying whether AGENTS.md was found. If it wasn't, show `Write a
     starter AGENTS.md`.
   - CodeRabbit and stale worktrees go behind `More rules`.
5. **A sticky glass bar** at the bottom, with `I'll write the tasks myself` and
   the primary `Plan it with <planner>`. The planner is the first enabled agent
   with the `planning` capability.

Behaviour: `Plan it` is `create(...)`, then `configure(...)`, then
`plan(agent)`. Today's `Starter.svelte` and the Settings pane both collapse into
this page. Its tests (`starter.test.ts`) move with it.

## 2 · Planning

- In the middle, `Loader` at pane size, with the title "`<Agent>` is planning"
  and an elapsed clock. Keep the one clock rule from FEAT-077: a single 5 s
  interval, only while something runs.
- Under it, **What the planner is doing**: `farmStore.planning` lines, with a
  kind label in front of each (`reading`, `thinking`, `task`) and a rise-in
  animation as each arrives. Only the last four are at full opacity; older
  lines fade. The count on the right says how many tasks have been proposed so
  far.
- A right inset card says what the planner was given, what each proposed task
  will contain, and that nothing runs until you accept. At the bottom is `Stop
  planning` (`cancelPlan`).
- When the run ends with drafts: the title becomes "The plan is ready", the orb
  stops breathing, and `Review the plan` appears. Go to Plan review on your
  own only when the window has focus.

## 3 · Plan review

- One sentence of explanation, then the drafts grouped into **waves**:
  - Wave 1 = tasks that need nothing.
  - Wave *n* = tasks whose deepest `dependsOn` is in wave *n − 1*.
  - Write `waves(tasks)` in `describe.ts` and test it, including cycles: a
    cycle goes in a final "Can't be ordered" column with a warning.
- Each card shows: a checkbox, id, kind, title, `allowedPaths`, the suggested
  agent (`assignedAgent`, or "First free agent"), and `needs T-…`.
- If you leave a task out and another task still needs it, that card gets a
  `--warn` border and says "needs T-01, left out".
- A floating pill at the bottom: `N kept · M left out`, `Discard`, `Plan
  again`, and the primary `Start building · N tasks`.
  - `Start building` = `readyTasks(kept)`, then `discardTasks(leftOut)`, then
    `start()`.
- The right card shows **Who does what** (a bar per agent), **When it starts**
  (the checks, how many at once, and the autonomy level, with a link to the
  rules), and **Worth a look**. Worth a look is generated: the task that waits
  the longest, and pairs of tasks whose `allowedPaths` overlap.

This replaces the "N tasks were proposed" band and `PlanningCard`.

## 4 · Building (the home)

From top to bottom:

1. **Header:** `Farm`, then the chips `Board · Crew · Activity`, then the phase
   tracker.
2. **The "now" card.**
   - The goal title, small.
   - **One sentence** saying what is happening, built from the counts. For
     example: "Two agents are working, one task is being checked, and two need
     you." Spell out numbers under ten. When paused: "Paused. Nothing new
     starts; the agents already working are left to finish." (This is what
     `pause_farm` does: it sets the status and runs continue.)
   - Under the sentence: when it started, the autonomy level, and where you
     come in.
   - On the right: `N of M landed`, plus one segment per task in the order
     landed, ready-to-land, stuck, working, checking, waiting. Segment colours:
     `--ok`, `--ok` at 45%, `--danger`, `--accent` (glowing), `--lane-5`,
     `--soft`. A legend sits underneath.
3. **Needs you.** Hidden when empty. One card per task that is waiting on a
   person:
   - **Ready to land** (`--ok` tint and merge icon). Show the checks result,
     the reviewer's verdict, and the commit/line counts. Buttons: `Land it`
     (`mergeTask`) and `See the changes` (opens Task on its Review tab).
   - **Stuck** (`--danger` tint and warning icon). Show the attempts and the
     last failure in one line. Buttons: `Retry with <another agent>`
     (`runTask(id, other)`, where *other* is the best free agent that has not
     failed it yet) and `Open the task`.
   - Use `farmStore.needsYou`. Extend it if it doesn't cover "verified and
     approved, waiting for merge" under assisted/semiAuto.
   - The rail's Farm item shows an `--accent` dot while this list is not
     empty.
4. **The board.** Four columns, not eleven statuses:

   | Column | Statuses | Caption under the title |
   | --- | --- | --- |
   | Up next | `ready`, `assigned`, `waiting`, `blocked` by dependency | Waits for a free agent or an earlier task |
   | Working | `running` | Each agent in its own worktree |
   | Checking | `verification`, `review` | Your commands run, then another agent reviews |
   | Landed | `done` | Merged into main (or the merge target) |

   Tasks that are stuck or ready to land show in Needs you, not on the board.
   `cancelled` tasks are hidden behind a "Show cancelled" toggle. Container
   tasks (FEAT-076) show as one card with `done/total` on its track. Clicking a
   container expands its children in place.

   Each card shows: id, kind chip, title, the agent's badge, one status line,
   and the task track.
   - Working: `6 min · editing <last file>`, with the strands loader.
   - Waiting: the reason, `Waits for T-05`, from `dependsOn` or
     `snapshot.waiting`.
   - Quiet: `quiet for 4 min` in `--warn` when `lastOutputMs` is older than
     three minutes.
5. **The right inset card.**
   - **Crew:** one row per agent with its badge, name and current task id, a
     status line, and the strands loader when busy. Free agents say what they
     will pick up next. The header shows "2 of 3 busy" with slot pills.
   - **Lately:** the last five events as one-line sentences
     (`describe.eventLine`), each with a coloured dot (start `--accent`, check
     `--lane-5`, landed/approved `--ok`, failed `--danger`, quiet `--warn`),
     and `All activity` at the end.
6. **A floating farm pill** at the bottom of the board column (a second
   ornament, like Review's).
   - Contents: `Pause`/`Resume`, `Autonomy <level> ▾`, slot pills with `N at
     once`, `+ Task` (opens today's `TaskEditor`), and `Rules`.
   - `Rules` opens the Setup "rules" section as a sheet: autonomy, checks, at
     once, tries, CodeRabbit, AGENTS.md, and leftovers.
   - The global repository toolbar stays unchanged under the pane.

The animated GIF shows `Land it` moving T-02 into Landed, and `Pause` changing
the sentence and the pill.

## 5 · Wrap up

Shown when `farm.status === 'completed'`.

- **The seal:** an `--ok` check in a glass circle that pops in once. Next to
  it: "`<goal>` has landed", the task count, how long it took, the commits on
  the target branch, and whether the checks are green on it.
- **What landed:** one row per task with id, title, the agent's badge, merge
  commit sha, and tries.
- **The agents raised:** the open items from every hand-off (`questions`,
  `risks`, `proposedTasks`), each with one action. `Answer` adds a note.
  `Make it a task`. `Start a farm with it` opens Setup with the goal filled in.
- **The right card:**
  - Who did what: tasks built and reviews done per agent, as bars.
  - **Tidy up:** `Remove worktrees and branches` (`sweep()`). The result line
    says the commits stay.
  - `Open main in Graph`.
  - The primary `Start a new goal`.

## 6 · Task

Route `/farm?task=<id>`. `‹ Board` goes back.

- **Header:** id, title, kind chip, and the actions for the task's state on the
  right:
  - working: `Stop`, `Open the worktree`;
  - ready: `Send it back`, `Land it into main`;
  - stuck: `Give up on it`, `Edit the task`, `Retry with <agent>`.
- **The stepper card:** the same six steps as the track, at full size.
  - The card's heading is one sentence: "Claude Code is writing it", "Ready to
    land. It needs your yes.", or "Stuck. The farm stopped trying."
  - `Attempt n of maxAttempts` sits on the right.
  - Each step has a one-line detail: when it was queued and what it waited
    for, the branch, the agent and elapsed time, the check commands, the
    reviewer, and who merges.
  - The current step holds the strands loader. A failed step shows an `×`
    in `--danger`. The "your turn" landing step is a filled `--ok` circle with
    a merge icon.
- **A callout**, only when it helps. On ready: what landing does and which
  tasks it frees. On stuck: what failed, three times, in plain words.
- **Tabs:** `Output · Changes · Checks · Review · Hand-off`. The default tab
  depends on the state: working opens Output, ready opens Review, stuck opens
  Checks.
  - **Output:** the live transcript in a sunken mono card. Colour each line by
    kind: think muted, edit `--ok`, command `--lane-5`, error `--danger`. A
    blinking caret shows while it runs. Footer: "`<agent>` is working · last
    line 8 s ago" and `Full log` (`transcript`).
  - **Changes:** files with `+/−`, and `Open the branch in Graph`.
  - **Checks:** one card per command with a dot, state and duration. The
    output opens under a failed one.
  - **Review:** the reviewer badge, the verdict chip, the summary, and the
    issues (severity chip, file, message).
  - **Hand-off:** the summary, risks, questions, and proposed tasks (`Add to
    plan`).
  - Empty tabs say when they will fill: "Codex reviews it once the checks
    pass."
- **The right card:** the agent (with `Reassign`), the brief, "Done when"
  (acceptance criteria with ticks), May touch, Needs, Asked by (`origin`),
  Branch, Worktree, and `Edit the task`.

This replaces `TaskDetail.svelte`. `TaskEditor` stays.

## 7 · Crew

- A grid of agent cards. Each card has:
  - the badge (haloed while working), name, role chip, path and version, and
    an `On the crew` switch (`saveAgent` with `enabled`);
  - a **now strip** in the agent's tint: what it is doing, or "Free · takes
    T-06 when T-05 lands";
  - capability chips;
  - the record from `scoreboard`: landed, sent back, failed checks, typical
    time;
  - traits in one line ("Prompt as an argument · streams output"), and
    `Arguments` (`extraArgs`).
- The right card explains how agents are found (with `Look again`), how tasks
  are assigned, and how to add another CLI agent.

Turning an agent off while it is running finishes the run first. Say so on the
card.

## 8 · Activity

- **The timeline:** the last *n* minutes, with one row per agent plus a
  "Checks · merges" row.
  - Bars come from `runs` (`startedMs` to `endedMs`, or now).
  - Bar styles: filled in the agent colour for writing; dashed for review and
    planning (`phase`); striped `--danger` when that run's checks failed.
  - Markers on the last row: `--ok` landed, `--lane-5` checks passed,
    `--danger` checks failed.
  - Light vertical rules mark time. An `--accent` "now" line sits on the right
    edge.
  - Time labels sit under the rows.
  - Hovering a bar shows its title. Clicking it opens the task.
- **What happened:** the event log, newest first, with filters `All · Needs
  you · Landed · Failures`. Each row has the time, agent badge, coloured dot,
  sentence, and a task chip that links to the task.

This replaces `ActivityDrawer.svelte`. The transcript part of the drawer moves
into the Task screen's Output tab.

## Words

Follow TASK-007 and `describe.ts`: say what is true, in short sentences. Every
new label goes in `describe.ts`, and `describe.test.ts` asserts the copy. Agents
are named by `displayName`, never by provider id. Times are relative ("6 min",
"spoke 8 s ago") on the board and absolute (`09:58`) in Activity and the
stepper.

## Motion and performance

- Everything that moves already exists: `wave` (strands), `weave`/`travel`/
  `breathe` (orb), a halo pulse, a `glow` opacity pulse, and `rise` for
  arriving lines.
- Under reduced motion, the shell's rule stops all of it.
- WebKitGTK paints in software. The only new blurred surfaces are the farm
  pill and the Setup bottom bar. Never blur a card or a row.
- Keep the store's event-driven design. Add no new timers beyond the single
  screen clock.

## What the code has today (read on 2026-10-07)

| Need | State |
| --- | --- |
| Agents, detection, traits, capabilities, role | ✅ `AgentStatus`, `detectAgents`, `saveAgent` |
| Goal, autonomy, checks, max parallel and attempts | ✅ `create`, `configure` |
| Planner streaming, cancel | ✅ `plan`, `farmStore.planning`, `planningRun`, `cancelPlan` |
| Drafts, accept/discard | ✅ `drafts`, `readyTasks`, `discardTasks` |
| Waves | ❌ derive from `dependsOn` in `describe.ts` |
| Why a task waits | ✅ `snapshot.waiting` plus `dependsOn` |
| Quiet agents | ✅ `AgentRun.lastOutputMs` |
| Needs you | ⚠️ `farmStore.needsYou` exists; check that it includes "verified and approved, waiting for a merge" |
| Land, retry with another agent | ✅ `mergeTask`, `runTask(id, agent)`. Check that `runTask` is allowed once `attempts >= maxAttempts`. If it isn't, reset attempts on an explicit retry. |
| Verification, review, hand-off per task | ✅ `taskDetail` |
| Commit count and `+/−` per task | ❌ not in `TaskDetail`. Use the existing diff machinery against the task branch's merge base, or add `filesChanged` stats to the hand-off read. |
| Timeline | ✅ `runs` with `startedMs`, `endedMs` and `phase`; events with `atMs` |
| Scoreboard | ✅ `scoreboard` (rename the columns to the copy above) |
| Wrap-up summary | ⚠️ derive from tasks, runs and events; merge sha from the `mergeCompleted` event, or add it |
| Hand-off items still open across the farm | ❌ collect from every `taskDetail(...).handoff`, or add one command that returns them all |
| Clean-up | ✅ `stale`, `sweep` |

## Suggested slices

Each slice is one work item under Amendment 12: an item, a plan, testing
documents and a branch. Assign the next free `FEAT-###` numbers when you start
each one, not before, because the record test refuses identifiers that point
at nothing. Update `docs/screens.md` § 1Q in the same change as each slice, add
a `CHANGELOG.md` entry under `[Unreleased]` (Amendment 20), and do not add a
dependency.

1. **The shared pieces.** `FarmPhases`, the agent badge, the task track, and
   `describe.ts` additions (`track`, `waves`, sentence builders). Tests first.
   Frontend only.
2. **Building.** The now card, Needs you, the four-column board, the crew and
   Lately card, the farm pill and the Rules sheet. Retire `AgentStrip`,
   `ProgressRing` and the three-pane layout.
3. **Task.** The `?task=` route, the stepper, the tabs and the brief card.
   Retire `TaskDetail.svelte`.
4. **Setup, Planning and Plan review.** Retire `Starter` and `PlanningCard`,
   and fold the Settings pane into Setup and the Rules sheet.
5. **Crew and Activity.** The timeline and the filtered log. Retire
   `ActivityDrawer`.
6. **Wrap up**, plus the backend gaps above: per-task stats, merge sha, open
   hand-off items.

Dependencies: 1 comes first. Slices 2 to 5 can then run in parallel. Slice 6
needs slice 2.

## Acceptance criteria (whole feature)

- From any Farm screen, a person can say which phase the farm is in and what
  comes next without clicking.
- Building answers, in its first sentence, how many tasks are working, being
  checked, and waiting on the person.
- Everything waiting on a person is in Needs you, with the action on the card.
  The rail shows a dot while it is not empty.
- Every task card shows where the task is in its six steps. Its Task screen
  shows the same steps with details.
- A quiet agent is marked after three minutes. A dead run is not shown as
  working.
- Every crew row says what that agent is doing, or what it will pick up next.
- The Activity timeline matches the runs and events on record.
- Pause changes the sentence and the pill, starts nothing new, and leaves
  running agents alone.
- Light and dark both match the images. Reduced motion stops every animation
  listed above.

## Open questions for the author

- Should `Land it` merge into the farm's target only, or offer a choice? Today
  `mergeTarget` exists per task.
- Should Wrap up appear by itself when the last task lands, or only after the
  last merge is approved?
- Should Plan review allow editing a brief inline, or go through `TaskEditor`
  as today?

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Handoff: Home, starting a farm, and the notification center

## Why

Raised by the author, 2026-10-18. Three things:

1. **Starting a farm is hard.** The Farm's Setup page (FEAT-073, redone in
   `design_handoff_farm/`) is one long form: an explainer, then the crew, the
   goal half-way down, then the rules — and only then a way to start. The
   thing that changes every time (the goal) is buried under the things that
   almost never change (crew and rules). The rest of the Farm — planning, the
   board, a task's journey, wrap up — is fine and **does not change**.
2. **There is no place that says what the repository needs from you.** What
   is waiting on you is spread over Pull requests, Review, Pipelines, Issues
   and the Farm.
3. **Notifications vanish.** The pull request watcher (FEAT-114) and every
   `notice` say things once, in a toast or the OS centre, and then they are
   gone.

## The decisions

| # | Decision |
| --- | --- |
| 1 | Starting a farm is a **wizard, one step at a time**: Goal → Start from → Crew &amp; rules → Start. Only a one-line goal is required. |
| 2 | **Crew and rules are remembered per repository**, from the last farm there. Step 3 asks "Same as last time?" and step 1 offers *Skip to start*. |
| 3 | A farm can start from **any mix** of: a one-line goal (always), issues, a written brief (`.md`), a branch in progress. |
| 4 | **Home** is a new screen and the new **top of the rail**: Home, a divider, then Farm and everything else as today. |
| 5 | A **notification center**: a drawer from a bell in the title bar, keeping every notification, across repositories. |
| 6 | The parallel-agents "hub" goes the **local** way first (see `HUB.md`). |

## About the design files

| File | What it is |
| --- | --- |
| `source/*.dc.html` | Design Component sources. Open them to read exact copy, structure and behaviour. **Treat the markup as a layout spec only.** Throw away the DC runtime. |
| `source/canvas.json` | How the artboards sit on the canvas. |
| `HUB.md` | The proposal for parallel agents on one repository. |

`Main.dc.html` **is Home** (the canvas's entry file has to be called Main).
`Rail`, `Dock`, `Steps` and `SoFar` are pieces the screens import.

Build it with the repo's own components (`Btn`, `Chip`, `Icon`, `Loader`,
`Menu`, `Sheet`, `.card`, `.ornament`) and tokens. Take exact pixel values
from `metrics.ts` and `app.css`, not from the mockups. Every repository name,
pull request, issue, person, count and time in the mockups is made up,
except the commit messages on Home, which are from the real log.

**Fidelity: high** for structure, copy, and the meaning of every colour.
Dark theme (Pomodoro Notte) only is drawn; Giorno follows from the tokens.

### What is *not* a change

- **The dock** (`Dock.dc.html`) is today's `Toolbar.svelte` and
  `StatusStrip.svelte`, drawn to match: branch field, Pull ▾, Fetch ▾, Push,
  divider, Branch, Stash. Its icon paths are copied from `icons.ts`. Do not
  change the toolbar.
- **The rail** (`Rail.dc.html`): `nav.ts` is the authority, and its icons
  are sketches. The **only** rail change is Home at the top, followed by a
  divider, above Farm. Farm keeps `1Q` and `/farm`.
- **The Farm after Setup** — Planning, Plan review, Building, Task, Crew,
  Activity, Wrap up — stays as `design_handoff_farm/` describes it.

### New icons

Two, in `icons.ts`'s style (24 box, stroke 1.7, round caps and joins):

| Name | Paths |
| --- | --- |
| `bell` | `M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z`, `M10 20.5a2 2 0 0 0 4 0` |
| `comment` | `M4 5h16v11H9l-5 4z` |

Home also needs a `home` icon for the rail:
`M3.5 10.5 12 3.5l8.5 7V20a1 1 0 0 1-1 1H15v-6H9v6H4.5a1 1 0 0 1-1-1z`.

---

## 1 · Home

The new top of the rail and, as the author decided, the screen the window
opens on. It gets a new screen code; `nav.ts` decides which.

**The idea:** one list of what is waiting on you, then the state of the
repository at a glance.

### Header

Repository name (large), the branch with ahead/behind (`main ↑2 ↓0`), when it
was last fetched. Under it, the remote, the licence and the version. On the
right: `Fetch all` and the primary `New farm` (opens the wizard).

### Needs you (the heart of the screen)

A card, two-thirds wide. One row per thing that is **waiting on you now**,
most urgent first. Filter chips: All, Reviews, Failing, Farm.

| Kind | Colour | Comes from | Action |
| --- | --- | --- | --- |
| Merge ready | `--ok` | Farm: a task verified and approved, waiting for a person | `Approve merge` (primary) |
| Review requested | `--lane-5` | Watched pull requests, `reviewRequested` | `Review` (primary) |
| Task stuck | `--danger` | Farm: failed after `maxAttempts` | `Open task` |
| Checks failed | `--danger` | Pipelines, on a branch of yours | `See the run` |
| Changes requested | `--warn` | Your pull request | `Open` |
| Assigned to you | `--lane-4` | Issues | `Open` |

Each row: a tinted icon tile, the kind in its colour, the title, one line of
context, the age, one button. Once the thing is done it leaves the list; it
is not marked read. Its record stays in the notification center.

### Beside it

- **Farm** card, when a farm exists: the goal, `2 of 9 merged · 7 to go`, the
  same segmented progress the Building header uses, the crew's badges (a
  halo on the busy ones), `Open the farm →`. No farm: a quiet `Start a farm`
  that opens the wizard.
- **Working copy**: changed count, stash count, the first three paths with
  their status letter, `Commit…` (goes to Working copy) and `Stash`.

### The rest, three across (they wrap on a narrow window)

- **Branches**: the five most recently touched. The name (mono), a tag
  (its pull request, `farm · <agent>`, `checks failing`), who, when, and a
  two-sided ahead/behind bar against the default branch. `All 23` →
  Branches.
- **Pull requests**: open ones, yours and those waiting on you first. State
  in colour (Your review, Approved, Changes requested, Draft), author, size,
  and a checks dot.
- **Latest commits**: seven rows, a dot in its lane colour on a thin line,
  hash, subject, age. `Graph` → the Graph.
- **Commits, last 16 weeks**: a heatmap, tomato from `--soft` to `--accent`
  in five steps, with a legend.
- **Pipelines**: one row per workflow, the last 14 runs as bars (newest on
  the right; failed taller and `--danger`, running `--warn` with a glow).
- **Releases**: latest tag, age, files; the count of entries under
  `## [Unreleased]` in `CHANGELOG.md`; `Draft v<next>` → Releases.

Every card that reads a forge uses the existing watcher clock (FEAT-114,
`watcher.subscribe`). Home adds no timer of its own.

---

## 2 · Starting a farm — the wizard

Replaces **Setup** only (`Setup.svelte`, screen 1 in
`design_handoff_farm/`). The phase tracker in the header is replaced, while
the wizard is open, by the wizard's own four steps (`Steps.dc.html`).

### The frame (every step)

- Left: the question, max 780 px wide. A small `Step n of 4` in `--accent`,
  then the question as a 36 px headline.
- Right: **Your farm** (`SoFar.dc.html`), a card that fills in as you go:
  Goal, Start from, Crew, Rules, Tasks. Crew and Rules show the remembered
  values dimmed with a `last time` tag until step 3 is reached.
- Bottom: a glass bar. Back on the left; on the right, Skip where a step is
  optional, and the primary `Continue`. `Enter` continues.

### Step 1 · Goal

"What should be true when it's done?" One large field (the title), and
*Anything the agents should know* (the description), optional. Under them,
**Or pick up a past farm**: recent farms in this repository — a stopped one
offers `Resume`, a done one `Use as a starting point` (copies its goal,
context and rules). *How a farm runs* (today's six-chip strip) is a
`<details>` fold, closed.

`Skip to start` goes straight to step 4 with the remembered crew and rules.

### Step 2 · Start from (optional)

"Anything to start from?" Three blocks, any mix:

- **Issues**: a searchable list with checkboxes (number, title, label).
  Uses the Issues screen's (FEAT-136) providers.
- **Brief**: one or more `.md` files from the repository or dropped in.
- **Branch in progress**: pick a branch; the farm finishes it or splits it.

### Step 3 · Crew &amp; rules

"Same crew and rules as last time?", naming the farm they came from.

- **Crew**: one toggle tile per detected agent — badge, name, what it is good
  at. Broken and missing agents as today's Setup shows them.
- **How much it does alone**: the five `AUTONOMY_LEVELS` as one segmented
  control, and under it the live **You come in** line (today's copy).
- **Checks on every task** (`$ cmd` lines, `+ Add a check`), **Agents at
  once**, **Tries before it needs you**.
- **Where each task works**: *A worktree per task* (today) or *A slice per
  task* (`new`; see `HUB.md` — hide it until the hub ships).
- `AGENTS.md found` line; CodeRabbit and stale worktrees behind `More rules`.

Saved per repository when the farm starts (in `.spagitty/`, next to the
farm's own state), and read back the next time.

### Step 4 · Start

"How should the tasks be made?" Three choices; the primary button says what
it will do:

| Choice | Button | Does |
| --- | --- | --- |
| Plan it with an agent (+ planner picker) | `Plan it with Claude Code` | `create` → `configure` → `plan(agent)`, as today |
| One task per issue (only when issues were picked) | `Make 3 tasks` | One draft per issue, straight to Plan review |
| I'll write the tasks | `Open an empty plan` | Today's "I'll write the tasks myself" |

**The planner gets** — chips listing exactly what goes into the planning
prompt: goal and notes, each issue, each brief, `AGENTS.md`, a map of the
repository. "Nothing runs until you accept the tasks."

---

## 3 · Notification center

A drawer from the right, over any screen, 470 px wide, the `.ornament`
material at full height under the title bar. The rest of the window dims.

### The bell

In the title bar (`TitleBar.svelte`), left of the window controls. An
`--accent` count of unread items; pressed while the drawer is open.

### What goes in it

Everything Spagitty tells you, **kept**:

| Category | Kinds |
| --- | --- |
| Pull requests | The watcher's five `NewsKind`s, with the watcher's headlines verbatim |
| Farm | Ready to merge, stuck, stopped, finished |
| Checks | Failed, passing again |
| Git | A fetch, pull or push that failed (today's `notice` errors), a release published |

Toasts and the OS notification still happen as today; each also lands here.

### The drawer

- Header: **Notifications**, `4 new`, `Mark all read`, settings (→ Settings'
  notification toggles), close.
- Search, and **All repos / this repo**.
- Chips: All, Unread (count), Pull requests, Farm, Checks, Git.
- Grouped by day (Today, Yesterday, then `Wednesday, Oct 14`), with sticky
  group headers.
- Each row: tinted icon tile, title (bold while unread), what it is about,
  the repository (mono chip), the time, an unread dot. On hover or focus:
  `Mark read` and one action — the same action Home's Needs you uses for
  that kind.
- Opening an item marks it read. Escape closes the drawer.

### Storage

A table in the app's own store, one row per notification: id, repository,
category, kind, title, body, a target to open, created at, read at. Kept
until the person clears it (see open questions).

---

## Open questions

1. With Crew folded into the wizard, does the Building tracker become
   **Plan → Build → Wrap up**?
2. Home's **Needs you**: this repository only, or every open repository with
   this one first?
3. Should a toast still show when the drawer's count would say the same
   thing (window focused)?
4. Keep every notification forever, or offer *Clear older than 90 days*?
5. The bell on every screen's title bar, or only beside the tabs?

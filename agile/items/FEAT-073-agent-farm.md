<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-073 — The agent farm: running and shepherding agents from inside Spagitty

**Status:** Done.
**Screens:** Farm (1Q), rail, Settings (1K).
**Raised by:** the product line — "your gateway to a Git-managed agent farm" —
which had, until this, no farm behind it.

## Problem

Spagitty could *read* what agents had done. It could attribute a commit from a
`Co-authored-by` trailer, rank agents by first-pass rate, and show several
worktrees at once. It could not run one, and it could not answer the question
that actually costs a person their day: *what are these four agents doing right
now, and which of them needs me?*

Without that, supervising a handful of agents means a terminal per agent, a
worktree per terminal, and a human holding the dependency graph in their head.
The information that matters — which task is blocked, which finished but was
never verified, which two are about to touch the same file — exists nowhere but
in the operator's memory.

## Change

A control plane, `crates/spagitty-farm`, and one screen over it.

- **The crate** is orchestration and nothing else. It calls `spagitty-core` for
  every git operation and reimplements none of them; `spagitty-core` gains no
  neighbour that spawns other people's programs. Inside it:
  `model/` (goal, task, agent, run, handoff), `agent/` (one adapter per
  provider, detection, registry), `workspace/` (a branch and a worktree per
  task, and the leases between them), `execution/`, `verification/`, `review/`,
  `orchestrator/` (dependency DAG, scheduler, router, planner), `persistence/`
  and `policy.rs`.
- **Adapters, not models.** Claude Code, Codex, Cursor and Oh My Pi are found on
  `PATH`; anything else with a command line can be added by hand. Spagitty runs
  them as the user; it contains no model and ships no key.
- **A branch and a worktree per task**, named `spagitty-farm/<task>/<provider>`
  so the graph, the worktree list and the Farm screen find each other by one
  derived name. Nothing an agent does reaches the user's working copy.
- **An agent saying "done" is not done.** Verification runs the repository's own
  commands in the task's worktree, and a review is performed by a *different*
  agent than the one that wrote the change. Both are in the path to `Done` and
  neither can be skipped by an agent's own report.
- **Agents never talk to each other.** Every handoff goes through Spagitty, so
  there is one audit trail and one place that decides what happens next.
- **Five autonomy levels**, from Manual (nothing runs by itself) to Unattended.
  The setting is a sentence about *where the human is*, not a magnitude, which
  is why it is a list of rows rather than a slider.
- **The farm on disk** is JSON under `.spagitty/`, written by rename so a crash
  leaves the previous state intact, with events appended one object per line.
  The directory is added to `.git/info/exclude`; a farm is never committed.
- **Farm (1Q)**: the plan on the left, the selected task in the middle, and what
  just happened along the bottom — the three questions a supervisor has, all
  visible at once. Events drive it; nothing polls.

## The starter page

The screen's first state is the one most people will see most often, and it was
a heading, a sentence and a button that sent you elsewhere to find the field you
needed. It is now the page that answers *what is this*, *how do I start* and
*will it work here*: the loop in four steps, a goal field that starts a farm
without navigating anywhere, and three readiness rows — whether an agent was
found, whether this repository has rules for one, and whether anything checks
the work. None of the three blocks starting a farm. A farm with no agent is
still a plan, and saying so is more useful than disabling the button.

## The rail

The Farm takes the rail's top slot. Everything below it is where the farm's
output is read, and a screen somebody supervising a farm is in all day does not
belong eleven rows down among the screens they visit to look something up. The
Graph keeps `/` and follows immediately.

**The repository's own state left the rail for the status strip.** Four lines
stacked under the screens — working copy, walk and remote freshness, tags and
submodules — two of which were second copies of counts the rows above already
carry as badges. A status strip is where an application says what is true of
what it has open, and one line spanning the window fits what took the rail's
width to say.

**Open repository leaves the rail once a repository is open.** It was a filled
accent button above every screen offering to replace the repository the person
was working in — the loudest pixel in the window, spent on the one action nobody
wanted from there. The tab strip's `+`, the repository menu and All repositories
all still open one.

## Non-scope

- **A model of our own, or an API key.** Spagitty runs what is installed.
  *Half reversed in 2.0 by FEAT-115:* Spagitty still has no model of its own
  and ships no key, but it now holds the person's own key, in the keychain, and
  makes model requests for an API agent they assign in Review or Merger. The
  farm itself still runs only what is installed.
- **Cross-repository farms.** A farm belongs to one repository, and lives in it.
- **Agents talking to each other directly.** See above; this is a rule, not a
  gap.
- **SQLite.** Argued in `persistence/store.rs`: a farm is a dozen tasks and a
  few hundred events, and the rest of Spagitty's state is JSON beside it.

## Acceptance criteria

- No farm is started, and no agent is run, without the user asking.
- A task's work is confined to its own branch and worktree.
- A task cannot reach `Done` on an agent's own say-so.
- The screen tells the truth when nothing is configured: no agent found, no
  rules file, nothing verifying — each said plainly rather than hidden.
- Deleting a task keeps the commits on its branch.
- `tools/record.test.ts` passes.

## Fixed in passing

- **Every theme wore the same accent.** The eight palettes each accented with
  the brand amber — `#976317` in light, `#eeb04d` in dark — which put one hue on
  seven palettes built around a different one, and a muddy brown on every light
  background. Each family now accents with a colour of its own, and
  `themes.test.ts` fails if two families ever share one again. Four more
  families landed with it: Nord, Rosé Pine, Solarized and Everforest.
- **Settings could not load or save external tools with no repository open.**
- **A custom agent was judged by `--version`**, which many agents do not answer
  to; it is judged by whether it runs.

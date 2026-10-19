<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-141 — Plan

**Item:** [FEAT-141](../items/FEAT-141-starting-a-farm-one-step-at-a-time.md)

Written on 2026-10-18, before the code.

## The pieces

1. **What a goal starts from**, in `spagitty-farm`'s `model/goal.rs`: `sources`,
   a list of issues (number, title, body, labels, url), briefs (path and text)
   and at most one branch. Serialised with a default, so a farm written before
   reads with none.
2. **The planning prompt** in `context.rs` gains a `STARTING POINTS` section
   (each issue, each brief, the branch) and a `REPOSITORY MAP` (the top two
   levels of `git ls-files`, folded and capped). Both are tested on the words
   they produce.
3. **The branch in progress:** a farm whose goal names a branch starts its tasks
   from that branch and merges them back into it, by giving those tasks that
   merge target.
4. **Remembered setup and past farms** in `persistence/store.rs`:
   `setup.json` holds the crew, the rules and which farm they came from;
   `past/<farm>.json` keeps a farm when a new one replaces it. The service lists
   past farms newest first and resumes one by putting it back.
5. **The commands** in `src-tauri/src/farm.rs`: `farm_create` takes the sources;
   `farm_remembered`, `farm_remember`, `farm_past`, `farm_resume`, and
   `farm_brief` (reads a `.md` file, size-capped) and `farm_briefs` (the
   repository's `.md` files).
6. **The wizard's state** in `src/lib/farm/wizard.ts`: the steps, what Your farm
   says at each one, what Continue and Skip do, the start button's words and the
   planner chips. Pure, so the tests read it without a screen.
7. **The screens** under `src/lib/farm/components/wizard/`: `Wizard.svelte` (the
   frame and the bar), `WizardSteps.svelte`, `SoFar.svelte`, and one component per
   step. The crew tiles and rules reuse what Setup and `RulesEditor` do; the
   autonomy control becomes one segmented row there and in the Rules sheet.
8. **The farm page** shows the wizard where it showed Setup, and the tracker
   becomes Plan → Build → Wrap up. `Setup.svelte` goes.
9. **Home's `New farm`** opens the wizard.

## What is decided, and why

- **Remembered in `.spagitty/farm/`,** next to the farm's own state, as the
  handoff says. It is per repository, like the agent registry.
- **A past farm is kept whole.** Resuming it is putting the file back, and using
  it as a starting point copies its goal, notes and rules into the wizard.
- **The repository map is computed, not asked of the agent,** so the chip that
  says it goes in is true before the planner starts.
- **"One task per issue" uses the existing draft path,** so its tasks arrive in
  Plan review like a planner's, and nothing runs until they are accepted.

## Not in this plan

- Dropping files onto the window (see the item).
- The slice choice in step 3 is drawn only when FEAT-143 is built.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-140 — Plan

**Item:** [FEAT-140](../items/FEAT-140-home.md)

Written on 2026-10-18, before the code.

## The pieces

1. **The overview** in `crates/spagitty-core/src/home.rs`: one read of what Home
   shows that only git knows — the remote, the licence, the version (the latest
   tag), when it was last fetched (`FETCH_HEAD`'s time), the branch against its
   upstream, the default branch, the five most recently touched local branches
   with ahead and behind against the default branch, commits per day over the
   last 16 weeks, and the entries under `## [Unreleased]` in `CHANGELOG.md` by
   heading. Each part is optional and a failure costs that part only. Ahead and
   behind are counted for those five branches, never for all of them (BUG-090).
2. **The command** `home_overview` in `src-tauri/src/commands.rs`, off the main
   thread like the rest.
3. **Needs you** in `src/lib/home/needs.ts`: a pure function from the farm's
   tasks, the open pull requests, the pipeline runs of the person's branches and
   the issues assigned to them, to rows in the handoff's order — Merge ready,
   Review requested, Task stuck, Checks failed, Changes requested, Assigned to
   you — and oldest first within a kind. The kind's colour, icon and action come
   from `$lib/notifications/kinds.ts` (FEAT-142).
4. **The store** in `src/lib/home/store.svelte.ts`: the overview, the stash, the
   forge reads (pull requests through the Pull requests store, the default
   branch's pipeline runs, the latest releases, assigned issues), each with its
   own error. Forge reads are subscribed to the watcher's clock; nothing else
   ticks.
5. **The screen** at `src/routes/home/+page.svelte` → `src/lib/home/HomeScreen.svelte`,
   with a component per card under `src/lib/home/`. Shared parts: `Btn`, `Chip`,
   `Icon`, `Loader`, `.card`. The farm card reads `farmStore` and reuses
   Building's segments (`SEGMENTS`, `segment`).
6. **The rail and the launch:** `nav.ts` gains `2A` Home, in a `home` group of
   its own, and Farm gets a divider before it. `session.ts` lands a resumed tab on
   Home. New icons `home`, `bell` and `comment` in `icons.ts`.
7. **The records:** `docs/screens.md`, the changelog, and the tests.

## What is decided, and why

- **`/home`, not `/`.** The Graph is `/` in a dozen places and in every stored
  session; moving it would change what each of those means. Home is reached by
  the rail and by the launch.
- **One command for the git half.** The header, Branches, the heatmap and
  Releases' changelog count are all local reads of the same repository; one
  call is one lock and one round trip.
- **The latest commits come from the graph's walk** when it is there, so the
  lane colours are the graph's own; without a walk they are drawn in the first
  lane.

## Not in this plan

- Across repositories, and anything on the toolbar.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-140 — Home: what this repository needs from you

**Status:** Done — on `release/3.2.0`; unmerged. Covered by Rust and mounted tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.2.0`.
**Screens:** 2A (new), chrome.
**Designs:** [Home, starting a farm, and the notification center](../../design_handoff_home_farm_notifications/README.md), section 1 and `source/Main.dc.html`.

## Change

What is waiting on the person is spread over Pull requests, Review, Pipelines,
Issues and the Farm. Home is one place that lists it, most urgent first, and then
shows the state of the repository at a glance. It is the new top of the rail and
the screen a launch opens on.

## Acceptance criteria

- **The rail:** Home is the first row, then a divider, then Farm and the rest as
  before. Farm keeps `1Q` and `/farm`. Home is `2A` at `/home`.
- **A launch opens on Home.** The last session's tab and selected commit still
  come back; only the screen is Home.
- **Header:** the repository's name, the branch with ahead and behind its
  upstream, when it was last fetched; under it the remote, the licence and the
  version. `Fetch all` and the primary `New farm`, which opens the farm wizard
  (FEAT-141).
- **Needs you:** one row per thing waiting on the person now, most urgent first,
  with the filter chips All, Reviews, Failing and Farm. The six kinds, their
  colours, sources and actions are the handoff's table: Merge ready (`--ok`),
  Review requested (`--lane-5`), Task stuck (`--danger`), Checks failed
  (`--danger`), Changes requested (`--warn`), Assigned to you (`--lane-4`). A
  row leaves once the thing is done; it is not marked read.
- **Beside it:** the Farm card (goal, merged and to go, the segmented progress
  Building uses, the crew with a halo on the busy ones, `Open the farm →`; or a
  quiet `Start a farm`), and Working copy (changed and stash counts, the first
  three paths with their letter, `Commit…` and `Stash`).
- **Three across, wrapping:** Branches (five most recently touched, a tag,
  who, when, a two-sided bar against the default branch, `All n`), Pull
  requests (open ones, yours and those waiting on you first, state in colour,
  author, size, a checks dot), Latest commits (seven, a dot in its lane colour,
  `Graph`), Commits over the last 16 weeks (a heatmap in five steps of the
  accent, with a legend), Pipelines (one row per workflow, the last 14 runs as
  bars), Releases (latest tag, age, files; entries under `## [Unreleased]`;
  `Draft v<next>`).
- **No timer of its own.** Every card that reads a forge uses the watcher's
  clock (`watcher.subscribe`, FEAT-114).
- **Tests:** the needs list is a pure function with a unit test per kind and for
  the order; the overview's reads have Rust tests on fixture repositories; the
  screen mounts with each card and with nothing connected.

## Not in this item

- Needs you across every open repository. Home is about the open one; the
  notification center (FEAT-142) is the place that spans them.
- Changing the toolbar or anything else on the rail.

## Decided on the way

The handoff's open question 2 (this repository, or every one) is answered as
*this repository only*, above. The notification center already spans them, and
a list of actions that act on another repository would have to open it first.

## What was built

- **The overview** in `crates/spagitty-core/src/home.rs` and the `home_overview` command: the remote as host and path, the licence from the licence file's words, the highest version tag, the branch against its upstream (`branches::current_divergence`), the default branch, the five most recent local branches with ahead and behind against it, sixteen weeks of commit times, and `## [Unreleased]` counted by heading.
- **Needs you** in `src/lib/home/needs.ts`, the card arithmetic in `shape.ts`, the reads in `store.svelte.ts`, and the cards under `src/lib/home/`. Kinds, colours, icons and buttons come from `src/lib/notifications/kinds.ts`, shared with the notification center.
- **The rail, the palette and the launch:** `2A` in `nav.ts` with a `home` group and a divider before the Farm; *Go to Home* in the palette; Settings › Rail shows a Home card; `session.ts` lands every launch on `/home`.
- **Icons** `home`, `bell` and `comment`, with the handoff's paths.

## Differences from the handoff

- **Assigned to you → Open** opens the issue on its host. The Issues screen has no way to be opened at one issue from outside it.
- **Draft v<next>** opens Releases; it does not start the draft with that version filled in.
- **Checks failed** reads the checked-out branch and the branches of the person's own open pull requests, at most three, on the watcher's clock.
- **Latest commits** come from the graph's walk, so they appear once the walk has started, with the graph's own lane colours.

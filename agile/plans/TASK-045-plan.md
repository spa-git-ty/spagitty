<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-045 — Plan

**Item:** [`agile/items/TASK-045-a-rail-of-five.md`](../items/TASK-045-a-rail-of-five.md)

## Approach

**What is on the rail is data, not markup.** `NavItem` gains `shows` (always,
while there are conflicts, or only while open), `also` (the other routes a row
stands for) and `delight`. `isShown` and `isItemActive` decide from a
`NavContext` — the path, the conflict count, whether the layer is on — and
`navRows` filters with them. The rail passes the context and draws what comes
back; `nav.test.ts` can assert every case without mounting anything.

**The refs screens share one control.** `RefTabs` replaces the title on
Branches, Tags, Stash and Reflog. It is four buttons at the title's size, the
current one at full strength, each with the count the rail used to carry.
The routes are unchanged, so a link, the palette and the back button all work
as they did.

**`Off` is a personality, not a separate switch.** A fourth level at the bottom
of the same scale keeps one setting instead of two that can disagree. The store
gains `on`; `apply` records and persists as before and skips the sound cue and
the pulse, and `announce` returns before the notice or the queue. Settings
filters God mode out of its chips while the layer is off; `#godmode` still
resolves, so nothing that links there breaks.

**The Rust default moves with it.** `Personality::Off` is `#[default]` and
`Settings::default()` names it. `serde(default)` fills a missing key from the
default, so a file that names a personality keeps it and a new install starts
at Off.

## Alternatives considered

- **One Refs screen with four panes.** A new route and a merged store, for the
  same result on screen. Four routes and a shared header change nothing that
  already links to them.
- **A separate `delight: bool` setting.** Two settings that can say
  "Professional, but off" is one too many.
- **Removing the routes that leave the rail.** Out of scope: they are still
  used, just not every day.

## Files

| File | Change |
| --- | --- |
| `src/lib/nav.ts` | `shows`, `also`, `delight`; `NavContext`, `isShown`, `isItemActive`, `REF_SCREENS`; the new list. |
| `src/lib/chrome/NavRail.svelte` | Draws the filtered rows; active by `isItemActive`. |
| `src/lib/branches/RefTabs.svelte` | New. |
| `src/routes/{branches,tags,stash,reflog}/+page.svelte` | `RefTabs` in place of the title. |
| `src/lib/delight/store.svelte.ts` | `on`; Off records silently. |
| `src/lib/settings/PersonalitySection.svelte` | The Off level; Off is silent like Professional. |
| `src/routes/settings/+page.svelte` | God mode only while the layer is on. |
| `src/lib/types.ts`, `src/lib/settings/store.svelte.ts` | `off` in the type and the default. |
| `src-tauri/src/settings.rs` | `Personality::Off`, the default. |

## Risks and rollback

- **People who used the rail by position.** Every row below Working copy moves
  up. The order of what is left is unchanged.
- **Existing installs keep their personality** if their settings file was ever
  written, because the file names it. That is deliberate: a choice somebody
  made, or accepted, is not undone by an upgrade.
- Rollback is a revert.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-056 — Plan

**Item:** [`agile/items/TASK-056-the-rail-as-the-author-chose-it.md`](../items/TASK-056-the-rail-as-the-author-chose-it.md)

## Approach

`src/lib/nav.ts` is the one place the rail is decided. Stash (1G), Tags (1N)
and Reflog (1M) are added to `NAV_ITEMS` under `tools` after Log, with the
icons the toolbar and menus already use for them; `shows: 'open'` comes off
Rebase, Log and All repositories; and the Branches row loses `also`, so
`isItemActive` puts you on the row of the screen you are on. `REF_SCREENS`
stays, for the segmented control the four refs screens share.

The drag: `CommitRows.svelte`'s `dropOnRef` used to open a menu of the four
integrations, each disabled unless the target was checked out. It now calls
`merger.present({ a: target, b: source, into: 'a' })` and goes to `/merge`.
`present` holds the pair until the screen next primes, where it wins over the
pair remembered from the last visit and is remembered in its place.

## Risks

- A longer rail on a short window: the rail is an ornament that scrolls with
  its labels hidden; nothing about its geometry changed.

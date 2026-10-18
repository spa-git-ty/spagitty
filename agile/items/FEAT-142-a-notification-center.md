<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-142 — A notification center that keeps what it said

**Status:** Done — on `release/3.2.0`; unmerged. Covered by Rust and mounted tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.2.0`.
**Screens:** chrome, 1K.
**Designs:** [Home, starting a farm, and the notification center](../../design_handoff_home_farm_notifications/README.md), section 3 and `source/Notifications.dc.html`.

## Change

The pull request watcher (FEAT-114) and every notice say things once, in a toast
or the operating system's centre, and then they are gone. A bell in the title bar
opens a drawer that keeps every notification, across repositories.

## Acceptance criteria

- **The bell** sits in the title bar, left of the window controls, with an
  accent count of unread items. It is pressed while the drawer is open.
- **The drawer** comes from the right over any screen, 470 px wide, in the
  ornament material at full height under the title bar; the rest of the window
  dims. Escape closes it.
- **What goes in it:** pull requests (the watcher's five kinds with its
  headlines verbatim), the farm (ready to merge, stuck, stopped, finished),
  checks (failed, passing again) and git (a fetch, pull or push that failed, a
  release published). Toasts and the operating system's notification still
  happen as before; each also lands here.
- **Header:** Notifications, `n new`, Mark all read, settings (Settings ›
  Notifications), close. Search, and All repos / this repo. Chips: All, Unread
  (count), Pull requests, Farm, Checks, Git.
- **Grouped by day** — Today, Yesterday, then `Wednesday, Oct 14` — with sticky
  group headers.
- **Each row:** a tinted icon tile, the title (bold while unread), what it is
  about, the repository, the time, an unread dot. On hover or focus, Mark read
  and one action — the same action Home's Needs you uses for that kind. Opening
  an item marks it read.
- **Storage:** one row per notification in the application's own store — id,
  repository, category, kind, title, body, target, created at, read at — kept
  until the person clears it. Settings › Notifications offers *Clear older than
  90 days*.
- **Tests:** the store's file round trip and its edits in Rust; grouping,
  filtering and the farm and checks translations as pure functions; the drawer
  mounted with each filter and the keyboard.

## Decided on the way

The handoff's open questions 3, 4 and 5: a toast still shows as before (the
drawer is a record, not a replacement); notifications are kept until cleared,
with *Clear older than 90 days* offered; the bell is in the title bar, which is
on every screen.

## What was built

- **The record** in `src-tauri/src/inbox.rs`: `notifications.json` in the configuration directory, written atomically after each change, and the `inbox_*` commands.
- **The drawer** in `src/lib/notifications/NotificationCenter.svelte`, its state in `center.svelte.ts`, the slicing in `center.ts`, the buttons in `act.ts`, and the bell in `TitleBar.svelte`.
- **The feeds:** the watcher keeps each piece of news as it announces it; `feed.svelte.ts` keeps failed fetches, pulls and pushes (heard from the notice that says so, `notice.listen`), the farm's ready, stuck, stopped and finished, and the checked-out branch's checks on the watcher's clock; Releases keeps a published release.
- **Settings › Notifications** says how many are kept and offers *Clear older than 90 days*.

## Differences from the handoff

- **A pull request row's button** opens it in Review when it is in the open repository, and on its host otherwise.
- **Checks** are read for the checked-out branch, and only while pull request notifications and *Failing checks* are on, at the watcher's interval.
- **The settings button** opens Settings › Notifications; there are no per-category switches beyond the ones the watcher already had.

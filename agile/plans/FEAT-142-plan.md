<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-142 — Plan

**Item:** [FEAT-142](../items/FEAT-142-a-notification-center.md)

Written on 2026-10-18, before the code. Built first of the three, because Home's
Needs you and the drawer share one table of kinds and actions.

## The pieces

1. **The store** in `src-tauri/src/inbox.rs`: `notifications.json` in the
   application's configuration directory, beside `settings.json`. One `Entry`
   per notification (id, repository path and name, category, kind, title, body,
   target, created, read). Commands: list, add, mark read, mark all read (all or
   one repository's), clear older than a date. Held in memory behind a mutex and
   written whole, atomically, after each change. Kept until cleared.
2. **The kinds** in `src/lib/notifications/kinds.ts`: one table from a kind to
   its category, icon, colour token and action label. Home's Needs you reads the
   same table, so a kind looks and acts the same in both places.
3. **The center's state** in `src/lib/notifications/center.svelte.ts`: the
   entries, the unread count, whether the drawer is open, and the actions. The
   grouping by day and the filters are pure functions in `center.ts`.
4. **What feeds it**:
   - the watcher's `announce`, one entry per piece of news, with its headline;
   - the farm, from `farm-event`, through a pure `fromFarmEvent` in `farm.ts`
     (ready to merge, stuck, stopped, finished);
   - checks, from the current branch's pipeline runs on the watcher's clock,
     compared with the last result seen (`checks.ts`, pure);
   - git, from the network store where a fetch, pull or push fails, and from
     Releases where a release is published.
5. **The bell** in `TitleBar.svelte`, left of the window controls, and **the
   drawer** in `src/lib/notifications/NotificationCenter.svelte`, mounted once in
   the layout. The drawer uses `.ornament` for its material; the scrim is a
   token colour.
6. **Opening an item** marks it read and follows its target: a route in the
   application, or a URL in the browser.
7. **Settings › Notifications** gains *Clear older than 90 days*.

## What is decided, and why

- **JSON, not SQLite.** The application has no database and every other store
  is a JSON file; a dependency for one table is not worth it.
- **Written whole.** A few thousand entries are a few hundred kilobytes; an
  atomic rewrite is simpler than a log that needs compacting.
- **Repository by path.** The name is stored too, because a path is not what a
  person reads, and a repository can be closed or moved after the fact.
- **The toast stays.** The drawer is the record of what was said, not a new
  way of saying it.

## Not in this plan

- Notification settings per category. The existing toggles already choose what
  the watcher says.

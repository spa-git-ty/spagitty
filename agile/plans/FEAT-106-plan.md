<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-106 — Plan

**Item:** [`agile/items/FEAT-106-a-rebase-you-can-see.md`](../items/FEAT-106-a-rebase-you-can-see.md)

- `src/lib/rebase/plan.ts`: `counts`, `sentence`, `ACTION_LOOK`, `orphanSquash` — the words and numbers, tested apart from markup.
- `src/lib/rebase/RebaseScreen.svelte`: head, stage (Merger's card, connector and result classes, Merger's `BranchPicker` for the starting point), lower row. Props: branch, HEAD's short id, `onpick`.
- `src/lib/rebase/RebasePlan.svelte` replaces `TodoList.svelte`; `RebaseHistory.svelte` (an SVG drawn from the preview) replaces `PreviewPane.svelte`.
- Store: `setAction` keeps a typed message; `setMessage`, `messageOf`; `seed` for tests and previews.
- `src/routes/rebase/+page.svelte` keeps only what it reads on arrival (progress, the upstream) and plans on pick.

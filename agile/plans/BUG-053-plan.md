<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-053 — Plan

**Item:** [`agile/items/BUG-053-what-the-author-found-sweeping.md`](../items/BUG-053-what-the-author-found-sweeping.md)

`routes/requests/+page.svelte` drops the shimmer for `Loader`. `metrics.ts`: `NODE_R` 12, `MERGE_R` 5.5, `LANE_PITCH` 28, new `NODE_TURN_RADIUS`. `lanes.ts`: `turnsAt(edge, above, below)` picks top / bottom / middle; top and bottom draw one `arcTo`. `graph/fit.ts` `refsFitWidth`, called from `GraphHeader`'s divider double-click with a canvas measure in the chips' font. `spagitty-core` `avatars.rs`: `forge_account` per `Kind`, `gitlab_avatar_of`, `bitbucket_account_of`, empty login allowed in the who-cache, image hosts; `commands.rs` lets GitLab ask without a commit. `version.ts` reads `package.json`; `StatusStrip` shows `about.version`.

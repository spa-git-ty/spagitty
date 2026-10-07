<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-112 — Implementation plan

**Item:** [FEAT-112](../items/FEAT-112-farm-crew-activity.md)

Crew availability, capabilities, arguments and scoreboard; run timeline and newest-first filtered event history.

Keep state event-driven. Reuse the app's Btn, Chip, Icon, Loader, card and ornament surfaces. Replace the DC runtime with repository components. Keep the rail at 1Q and route task details through `?task=`. Validate behavior with mounted components and temporary Git repositories, then review the production build against the supplied images. The six slices share one integration branch because they replace one route and its common store.


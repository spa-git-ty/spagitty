<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-061 — Plan

**Item:** [BUG-061](../items/BUG-061-merger-shows-the-previous-pairs-forecast.md)

Guard the forecast at the store rather than in each component: the getter returns it only when `forecast.a.name` and `forecast.b.name` are the picked `a` and `b`, which is how the backend names the sides. Route `roles`, `choices` and `ask()` through the getter. Add page tests: change B while the next forecast is pending and expect the loader and no Resolve or Merge now for the old pair, then the new plan; change only the strategy and expect no loader and no new request. Correct two fixtures that answered with sides the page had not asked for.

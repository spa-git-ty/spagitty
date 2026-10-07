<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-057 — Plan

**Item:** [`agile/items/TASK-057-the-log-screen-in-the-house-style.md`](../items/TASK-057-the-log-screen-in-the-house-style.md)

`src/lib/search/LogScreen.svelte` holds the layout (head, query card, results card, side cards), reusing Merger's head, label and card vocabulary; the route keeps the walk's subscription. `QueryBar`, `ResultRows`, `ResultDetail` and `BlameStrip` keep their behaviour and class names and change markup and style; rows and the detail use `AuthorAvatar`. `search.seed` for tests and previews.

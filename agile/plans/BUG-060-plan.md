<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-060 — Plan

**Item:** [BUG-060](../items/BUG-060-merger-forecast-lost-on-first-open.md)

Do not restart a forecast that a refresh did not change. Read each branch's and tag's commit in `prime()`, remember the question `load()` asked — repository, pair, both tips — and skip `load()` when a refresh asks the same one while an answer is on its way or in hand. Test with a deferred forecast: three refreshes before it answers still let it land; a moved tip asks again and keeps only the new answer; a settled forecast survives a refresh; choosing another branch still asks. Confirm the tests fail without the change.

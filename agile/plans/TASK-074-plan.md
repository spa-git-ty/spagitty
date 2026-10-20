<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-074 — Plan

**Item:** [TASK-074](../items/TASK-074-pull-requests-no-longer-list-gates-5-and-6.md)

1. Move the scope job and gates 1 to 4 out of `gates.yml`, word for word, into `checks.yml` under `on: workflow_call`, with `ship` as an output.
2. `gates.yml`: `push` only; a `checks` job calls `checks.yml`; gates 5 and 6 need `checks` and read `needs.checks.outputs.ship`. The `event_name == 'push'` condition goes, since the file only runs on a push.
3. `pull-request.yml`: the old `pull_request` trigger with its `edited` type, the same concurrency group, and a `checks` job calling `checks.yml`.
4. `docs/ci.md` says which file runs what.

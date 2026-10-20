<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-074 — Automated test record

**Item:** [TASK-074](../items/TASK-074-pull-requests-no-longer-list-gates-5-and-6.md)

- `tools/release-macos.test.ts` and `tools/updates.test.ts` read the build lane from `gates.yml`, which still holds gate 5.
- The three workflows were parsed with `Bun.YAML.parse`: `gates.yml` runs on `push` with `checks`, `build`, `release`; `checks.yml` on `workflow_call` with gates 0 to 4; `pull-request.yml` on `pull_request` with `checks`.

## Results — 2026-10-20

- `bunx vitest run tools`: see the pull request's gate 3.
- The pull request itself is the first run of `pull-request.yml`; the merge into `main` is the first run of the new `gates.yml`.

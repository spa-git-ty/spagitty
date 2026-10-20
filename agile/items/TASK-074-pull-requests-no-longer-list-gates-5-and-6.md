<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-074 — Pull requests no longer list gates 5 and 6

**Status:** Done — on `feat/FEAT-153-blame-on-click`; unmerged. The workflows parse; the lanes prove themselves on the pull request and on the merge into `main`.
**Branch:** `feat/FEAT-153-blame-on-click`.
**Screens:** none; `.github/workflows/`.
**Raised by:** the author, 2026-10-20: "ther's 2 steps on pr to main gate that are alwys not complient in the merge … 5 · build … 6 · release … they should only work on main not on prs".

## Problem

Gates 5 and 6 were jobs in `gates.yml`, which ran on pushes and pull requests alike, and an `if:` kept them from running on a pull request. A job skipped by its `if:` is still a job: every pull request's merge box listed *5 · build* and *6 · release* as skipped, beside the gates that had passed, and they read as checks the pull request had not met.

## Change

- **`checks.yml`** holds gates 0 to 4 unchanged, as a reusable workflow whose `ship` output is gate 0's answer.
- **`gates.yml`** runs on a push to `main` or `dev`: it calls `checks.yml`, then gates 5 and 6 as before, on `main` and only for a change that ships.
- **`pull-request.yml`** runs on a pull request into `main` or `dev` and calls `checks.yml` only. Gates 5 and 6 do not exist there, so they are not listed.

The checks on a pull request are now named *checks / 1 · licenses* and so on. No ruleset requires a check by name, so nothing has to be changed on GitHub.

## Acceptance criteria

- A pull request into `main` lists gates 0 to 4 and no gate 5 or 6.
- A merge into `main` that ships still builds and releases.
- `tools/release-macos.test.ts` and `tools/updates.test.ts` still read the build lane from `gates.yml`.

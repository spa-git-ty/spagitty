<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Branch rulesets

`main.json` is the ruleset for the default branch. GitHub does not read it from
the repository: it has to be applied once by a repository admin, and this file
is the record of what was applied.

## What it enforces

- Nobody pushes to `main` directly, admins included. Changes arrive through a
  pull request.
- Only the repository **admin** role can merge a pull request into `main`.
  Contributors with write access can open and review pull requests, but the
  merge button is closed to them (`update` restricts every update, and the
  admin role bypasses it in `pull_request` mode only, so the bypass works
  through a merge and never through a push).
- `main` cannot be force-pushed or deleted.

Keep the admin role to the maintainer alone; anyone given admin can merge.

## Applying it

From a checkout, with the GitHub CLI signed in as an admin:

```sh
gh api -X POST repos/spa-git-ty/spagitty/rulesets --input .github/rulesets/main.json
```

Or in the browser: **Settings › Rules › Rulesets › New ruleset › Import a
ruleset**, then choose `main.json`.

To change it later, edit `main.json` here first, then update the live ruleset
(`gh api -X PUT repos/spa-git-ty/spagitty/rulesets/<id> --input ...`).

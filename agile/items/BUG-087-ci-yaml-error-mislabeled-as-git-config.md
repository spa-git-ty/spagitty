<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-087 — CI YAML parse error is reported as a git-configuration error

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1X (Pipelines · preview)
**Raised by:** the author, 2026-10-18, on 3.1.0: "could not read the git configuration: Duplicate YAML key …".

## What happens

A duplicate YAML key in `.gitlab-ci.yml` surfaces as "could not read the git configuration: Duplicate YAML key …", misattributing a CI workflow parse error to Git configuration.

## Cause

In `crates/spagitty-core/src/pipelines/mod.rs`, `read()` wraps `yaml::parse` with the `config` helper, which produces `Error::Config` → "could not read the git configuration: …". The message is generic; the call site is a workflow parse, not a git-config read.

## Acceptance criteria

- A YAML parse error is reported without the "git configuration" prefix, e.g. "could not read the workflow: …".

## Fix

- `crates/spagitty-core/src/pipelines/mod.rs`: replace `.map_err(config)?` at the YAML parse with a message that names the workflow.

## Not done

- `Error::Config` is still used elsewhere; no new error variant added.
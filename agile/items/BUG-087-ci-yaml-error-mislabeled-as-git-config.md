<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-087 — A repeated key empties the GitLab preview, and says it is a git-configuration error

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1Y (Pipelines · preview)
**Raised by:** the author, 2026-10-18, on 3.1.0: `.gitlab-ci.yml` shows 0 jobs and "could not read the git configuration: Duplicate YAML key package-digital-onboarding-service-docker."

## What happens

The author's `.gitlab-ci.yml` defines `package-digital-onboarding-service-docker` twice (lines 883 and 1105) and `package-ekyc-api-service-docker` twice. GitLab runs the file and keeps the later definition. Spagitty refused the whole file: no jobs, no graph, one problem worded as a git-configuration failure.

## Cause

- `pipelines/yaml.rs` returned an error on any repeated mapping key, so one repetition discarded the workflow.
- Every Pipelines error was built with `Error::Config`, whose text starts "could not read the git configuration:". The first attempt on this branch added "could not read the workflow:" inside it, which only made the sentence longer: "could not read the git configuration: could not read the workflow: Duplicate YAML key …".

## Acceptance criteria

- A repeated key keeps the later value, as GitLab does, and the file is drawn.
- Each repetition is listed as a problem on its line, naming the earlier line and what the host does with it (GitLab keeps the later one; GitHub refuses the workflow).
- No Pipelines message says "git configuration".

## Fix

- `yaml::parse_all` returns the tree and the repeated keys; `workflow_from` turns them into problems at the top of the list.
- New `Error::Workflow`, whose text is the sentence alone; the Pipelines `config()` helper and the two command refusals use it.
- Test: `a_duplicate_key_keeps_the_later_value_and_is_reported`.

## Checked

- On the author's `core-banking-services` checkout (read only): 29 jobs, 2 problems, 0.2 s.

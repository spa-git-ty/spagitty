<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Preview workflows and follow their runs

**Status:** Backlog — recorded 2026-10-14; not started.
**Branch:** `codex/FEAT-138-pipelines`.
**Screens:** 1Y, chrome.
**Raised by:** the author, 2026-10-14: “focus on the screens !! , apply them all functional end to end”.
**Proposal:** [Forge views](../../docs/proposals/forge-views.md). **Designs:** [Forge views](../../design_handoff_forge_views/).

## Change

Draw the working copy’s workflows before a push, then follow runs on watched branches and requests.

## Acceptance criteria

- Parse both workflow formats, local includes, anchors, references, inheritance and matrices as specified in proposal §7.1; keep source lines and report bundled schema problems with nearest-key hints.
- Evaluate the specified triggers and expression subset against event, branch/tag and changed files. Unsupported expressions and remote includes are decided on the host.
- Draw virtual job graphs in Needs and Stages layouts, with edges, expandable matrices, the selected job’s details and file links. Collapse over 200 jobs to stages.
- Read host runs, jobs, steps and logs; persist watched branches and include the default branch and open requests. Poll at the stated intervals only while visible; say when updated.
- Cancel and re-run failed with separate confirmations and useful permission errors. Offer the GitLab lint action explicitly.
- Match both Pipelines artboards; omit local execution as agreed.
- **Tests:** refusal and success cases in core fixture tests, command tests, store tests and mounted screen tests; types, coverage and build pass.

## Not in this item

- Other hosts, workflow editing, local workflow execution and partial-staging stashes are outside 3.1.
- The other three screens have their own items.

## The author's note of 2026-10-14

Later on 2026-10-14, the author said: “اعمل بوش اول ب اول وافتح ال PR” and “عشان اراجع معاك اول ب اول”. Push each checked feature branch and open its pull request for incremental review. This supersedes the earlier no-push decision; no merge remains authorized.

The author approved the proposed implementation choices on 2026-10-14: all listed host writes, each confirmed; existing managers remain in control unless Import is chosen; shared blocks use `.spagitty/hooks.toml`, personal blocks use application data; local pipeline execution is deferred; hidden releases are local; separate stacked branches start at `release/3.0.1`, with no merge or push.

The author then said: “اه كمل كده + في الريل الي عالشمال اعمل فاصل للشاشات الجديده زي البروبوزال ما عامل احنا الردي عندنا فاصل فوق بعد الفارم و فاصل تحت قبل الكونفجرشن حطهم فوق الكونفرجيشن وحط بينهم وبين الي فوقيهم خط فاصل”. The four screens form a separate group immediately above Settings, with a divider above them. Existing chrome is preserved. This later note overrides the proposal’s position after Recipes.

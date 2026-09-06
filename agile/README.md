<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Working record

This directory is the working record of what is being built, as required by
Amendment 12 of the amendments book (`docs/AMENDMENTS.md` points at the
canonical copy).

```
agile/
  items/       work items: features, tasks, bugs
  plans/       one implementation plan per work item
  testing/     test plans per work item — automated and manual sweep
  roadmap.md   the order the outstanding work is meant to be taken in
```

Identifiers are `FEAT-###`, `TASK-###` and `BUG-###`, assigned in sequence and
never reused. The identifier is the join key: an item's four documents are

```
agile/items/FEAT-004-<short-kebab-description>.md
agile/plans/FEAT-004-plan.md
agile/testing/FEAT-004-automated.md
agile/testing/FEAT-004-sweep.md
```

and its branch is `feature/FEAT-004-<short-kebab-description>`.

Backlog items — recorded but not started — carry an item document only. Their
plan and testing documents are written when the work begins, since a plan
written before the work is a guess rather than a record.

## Status

The first word of an item's `**Status:**` line is one of these, and it must be
the same word as the one in the index. Everything after it is prose.

| Word | Means |
| --- | --- |
| `Done` | Built and merged. |
| `Fixed` | A bug, fixed and merged. |
| `Partial` | Some of it is built and merged and some is not. The item says which. |
| `Open` | Being worked on now. |
| `Backlog` | Recorded, not started. Item document only. |

`Partial` exists because the record had no word for it and used `Backlog`
instead, which sent readers to write code that was already there (TASK-012).

## This index is checked

`tools/record.test.ts` runs in the ordinary test suite and fails if the index
and the tree disagree: a missing row, a row with no document, a status that does
not match the item, a cited identifier that resolves to nothing, or a built item
whose missing documents are not recorded below. Drift is a test failure now
rather than an audit.

## Features

| ID | Title | Screen | Status |
| --- | --- | --- | --- |
| [FEAT-001](items/FEAT-001-graph-screen.md) | Graph screen | 1A | Done |
| [FEAT-002](items/FEAT-002-diff-screen.md) | Diff screen | 1B | Done |
| [FEAT-003](items/FEAT-003-working-copy.md) | Working copy / Commit | 1C | Done |
| [FEAT-004](items/FEAT-004-branches.md) | Branches | 1F | Done |
| [FEAT-005](items/FEAT-005-stash.md) | Stash | 1G | Done |
| [FEAT-006](items/FEAT-006-all-repositories.md) | All repositories | 1J | Done |
| [FEAT-007](items/FEAT-007-log-search.md) | Log search | 1I | Done |
| [FEAT-008](items/FEAT-008-conflicts.md) | Conflicts | 1D | Done |
| [FEAT-009](items/FEAT-009-rebase.md) | Interactive rebase | 1E | Done |
| [FEAT-010](items/FEAT-010-pull-requests.md) | Pull requests | 1H | Done |
| [FEAT-011](items/FEAT-011-settings.md) | Settings | 1K | Done |
| [FEAT-012](items/FEAT-012-clone.md) | Clone | 1L | Done |
| [FEAT-013](items/FEAT-013-branch-destructive-operations.md) | Branch delete and rename | 1F | Done |
| [FEAT-014](items/FEAT-014-stash-pop-apply-drop.md) | Stash pop, apply and drop | 1G | Done |
| [FEAT-015](items/FEAT-015-rebase-execution.md) | Rebase execution | 1E | Done |
| [FEAT-016](items/FEAT-016-conflict-resolution-writes.md) | Conflict resolution writes | 1D | Done |
| [FEAT-017](items/FEAT-017-forge-integration.md) | Forge integration | 1H, 1K | Done |
| [FEAT-018](items/FEAT-018-fetch-and-push.md) | Fetch and push | chrome | Done |
| [FEAT-019](items/FEAT-019-commit-signing.md) | Commit signing | 1C, 1K | Done |
| [FEAT-020](items/FEAT-020-show-git-commands.md) | Show the git command behind each action | all, 1K | Done |
| [FEAT-021](items/FEAT-021-themes.md) | Themes, and a title bar that stops lying | chrome, 1K | Done |
| [FEAT-022](items/FEAT-022-graph-parity.md) | The graph as a launcher, not a report | 1A | Done |
| [FEAT-023](items/FEAT-023-graph-presentation.md) | Author heads, and a graph column of its own | 1A | Done |
| [FEAT-025](items/FEAT-025-resizable-message-column.md) | A table you can size, and one that scrolls sideways | 1A | Done |
| [FEAT-026](items/FEAT-026-collapsible-rail.md) | The nav rail collapses to icons | chrome | Done |
| [FEAT-027](items/FEAT-027-repository-tabs.md) | Repository tabs in the title bar | chrome | Done |
| [FEAT-028](items/FEAT-028-toolbar-groups.md) | The toolbar: centred, grouped, and without Commit | chrome | Done |
| [FEAT-029](items/FEAT-029-graph-and-type-presentation.md) | Bigger faces, and a type scale the rail follows | 1A, chrome | Done |
| [FEAT-030](items/FEAT-030-rail-open-repository.md) | The rail's open-repository slot | chrome | Done |
| [FEAT-033](items/FEAT-033-branch-divergence-on-the-chip.md) | Branch divergence on the chip | 1A | Done |
| [FEAT-034](items/FEAT-034-stash-entry-file-browsing.md) | Browse a stash entry file by file | 1G | Done |
| [FEAT-035](items/FEAT-035-lane-overflow-compression.md) | Lanes past the cap compress instead of stacking | 1A | Done |
| [FEAT-036](items/FEAT-036-one-chip-per-branch.md) | One chip per branch, local and remote as icons | 1A | Done |
| [FEAT-037](items/FEAT-037-window-depth-and-resizable-panels.md) | The window has depth, and every panel resizes | chrome | Done |
| [FEAT-038](items/FEAT-038-pull.md) | Pull | chrome | Done |
| [FEAT-039](items/FEAT-039-resizable-graph-column.md) | The graph column resizes, and the lanes compress into it | 1A | Done |
| [FEAT-040](items/FEAT-040-graph-footer.md) | The graph's footer says what is true, not what to do | 1A | Done |
| [FEAT-041](items/FEAT-041-rail-drops-the-shortcut-hint.md) | The rail stops advertising a shortcut | chrome | Done |
| [FEAT-042](items/FEAT-042-tighter-corners-and-a-round-cast.md) | Tighter corners, and a cast shadow that follows them | all | Done |
| [FEAT-043](items/FEAT-043-app-status-strip.md) | A status strip along the bottom of the window | chrome | Done |
| [FEAT-044](items/FEAT-044-repo-tabs-own-row.md) | The repository tabs get a row of their own | chrome | Done |
| [FEAT-045](items/FEAT-045-toolbar-repo-and-branch.md) | The toolbar names the repository, and picks a branch for real | chrome | Done |
| [FEAT-046](items/FEAT-046-graph-squeeze-keeps-the-portraits.md) | Squeezing the graph column must not shrink the portraits | 1A | Done |
| [FEAT-047](items/FEAT-047-branch-table-columns-and-divergence.md) | The branches table: resizable columns, and a divergence worth reading | 1F | Done |
| [FEAT-048](items/FEAT-048-discard-changes.md) | Discard changes | 1C | Done |
| [FEAT-049](items/FEAT-049-remotes-management.md) | Remotes management | settings | Done |
| [FEAT-050](items/FEAT-050-reflog-view.md) | Reflog view | 1M | Done |
| [FEAT-051](items/FEAT-051-tags-list.md) | Tags list | 1N | Done |
| [FEAT-052](items/FEAT-052-graph-reads-at-depth.md) | The graph reads at depth | 1A | Done |
| [FEAT-053](items/FEAT-053-square-lane-turns.md) | Lanes turn square | 1A, 1G | Done |
| [FEAT-054](items/FEAT-054-update-check.md) | Tell people when there is a newer Spagitty | 1K | Done |
| [FEAT-056](items/FEAT-056-hide-the-detail-panel.md) | The detail panel can be put away | 1A, 1H | Done |
| [FEAT-055](items/FEAT-055-the-window-paints-on-the-path-that-works.md) | The window paints on the path that works | — | Done |
| [FEAT-057](items/FEAT-057-liquid-glass.md) | Liquid glass: the pane that bends what is behind it | all | Done |
| [FEAT-058](items/FEAT-058-pull-request-files-and-review.md) | A pull request you can read and answer | 1H | Done |
| [FEAT-059](items/FEAT-059-pull-request-review-workspace.md) | Dedicated pull request review workspace | 1H | Done |
| [FEAT-060](items/FEAT-060-spagitty-brand.md) | Spagitty, with a face: the brand | brand | Done |
| [FEAT-061](items/FEAT-061-brand-guide-and-showcase.md) | Brand guide and interactive showcase | brand | Done |
| [FEAT-062](items/FEAT-062-worktrees-management.md) | Worktrees management | 1J, chrome | Done |
| [FEAT-063](items/FEAT-063-file-history-and-blame.md) | File history and blame view | 1I, 1O | Done |
| [FEAT-064](items/FEAT-064-diff-syntax-highlighting.md) | Diff syntax highlighting | 1B, 1C, 1G, 1H | Done |
| [FEAT-065](items/FEAT-065-image-and-binary-diffs.md) | Image and binary diffs | 1B, 1C, 1G | Done |
| [FEAT-066](items/FEAT-066-diff-content-search.md) | Diff content search | 1I | Done |
| [FEAT-067](items/FEAT-067-submodules-management.md) | Submodules management | 1F, settings | Done |
| [FEAT-068](items/FEAT-068-external-diff-merge-tools.md) | External diff and merge tool launchers | 1K | Done |
| [FEAT-069](items/FEAT-069-multi-identity-profiles.md) | Multi-identity profiles | 1K, chrome | Done |
| [FEAT-070](items/FEAT-070-extended-forge-integration.md) | Extended forge integration | 1H, 1K | Done |
| [FEAT-072](items/FEAT-072-delight-layer.md) | The delight layer: badges, titles and reward moments | 1P, 1K, chrome | Done |
| [FEAT-073](items/FEAT-073-agent-farm.md) | The agent farm: running and shepherding agents from inside Spagitty | 1Q, 1K, chrome | Done |
| [FEAT-074](items/FEAT-074-the-activity-drawer.md) | The activity drawer: a log worth reading | Farm (1Q) | Done |
| [FEAT-075](items/FEAT-075-the-queue-explains-itself.md) | The queue explains itself | Farm (1Q) | Done |
| [FEAT-076](items/FEAT-076-the-farm-takes-on-large-work.md) | The farm takes on large work | Farm (1Q) | Done |
| [FEAT-078](items/FEAT-078-who-asked-for-this-task.md) | Who asked for this task | Farm (1Q) | Done |
| [FEAT-077](items/FEAT-077-the-farm-is-worth-watching.md) | The farm is worth watching | Farm (1Q) | Done |

## Bugs

| ID | Title | Screen | Status |
| --- | --- | --- | --- |
| [BUG-001](items/BUG-001-type-errors-shipped-in-task-002.md) | Type errors shipped in TASK-002 | — | Fixed |
| [BUG-002](items/BUG-002-primary-button-invisible.md) | The primary button has no fill | all | Fixed |
| [BUG-003](items/BUG-003-lane-canvas-offset.md) | The lane canvas paints over the messages | 1A | Fixed |
| [BUG-004](items/BUG-004-linux-blank-webview.md) | The packaged Linux app opens a blank window | packaging | Fixed |
| [BUG-005](items/BUG-005-metrics-doc-and-test-drift.md) | The geometry change left its test and comments behind | 1A | Fixed |
| [BUG-006](items/BUG-006-repo-card-branch-overlap.md) | A long branch name overlaps the branch count | 1J | Fixed |
| [BUG-007](items/BUG-007-replaced-dialog-resolves-wrong-value.md) | A replaced dialog resolves the wrong caller's value | all | Fixed |
| [BUG-008](items/BUG-008-menu-arrow-up-from-nothing.md) | ArrowUp into a fresh menu lands in the middle | all | Fixed |
| [BUG-009](items/BUG-009-message-column-has-no-handle.md) | The commit message column cannot be resized | 1A | Fixed |
| [BUG-009b](items/BUG-009b-graph-divider-resizes-message.md) | The boundary people reach for carries a dead divider | 1A | Fixed |
| [BUG-010](items/BUG-010-case-insensitive-shadowing.md) | A component and its store differ only by case | — | Fixed |
| [BUG-011](items/BUG-011-tls-provider-never-selected.md) | The first HTTPS request kills the process | 1H, 1K | Fixed |
| [BUG-012](items/BUG-012-network-freezes-the-window.md) | A network request freezes the window | 1H, 1K | Fixed |
| [BUG-013](items/BUG-013-tab-strip-with-no-repository.md) | The window comes back with a tab strip and no repository | chrome | Fixed |
| [BUG-014](items/BUG-014-conflicts-footer-says-resolving-is-not-built.md) | The Conflicts screen says resolving is not built | 1D | Fixed |
| [BUG-015](items/BUG-015-backend-list-disarms-the-safe-renderer.md) | A backend preference list disarms the safe renderer | — | Fixed |
| [BUG-016](items/BUG-016-graph-columns-stop-at-the-last-row.md) | The graph's columns stop at the last commit row | 1A | Fixed |
| [BUG-017](items/BUG-017-the-lens-wipes-the-window.md) | The lens wipes the right column and the bottom of the window | all | Fixed |
| [BUG-018](items/BUG-018-a-menu-cannot-be-dismissed.md) | A menu cannot be dismissed, and the next one is drawn over it | all | Fixed |
| [BUG-019](items/BUG-019-closing-the-last-tab-leaves-the-repository-open.md) | Closing the last tab leaves the repository open | chrome | Fixed |
| [BUG-020](items/BUG-020-the-window-freezes-while-the-farm-plans.md) | The window freezes while the farm plans | Farm (1Q) | Fixed |
| [BUG-021](items/BUG-021-a-run-says-nothing-until-it-ends.md) | A run says nothing until it ends | Farm (1Q) | Fixed |
| [BUG-022](items/BUG-022-the-farm-subscribes-after-it-asks.md) | The farm subscribes after it asks, so the answer is lost | Farm (1Q) | Fixed |
| [BUG-023](items/BUG-023-a-record-test-reads-another-tests-clone.md) | A record test reads another test's clone | none | Fixed |
| [BUG-024](items/BUG-024-cancellable-runs.md) | Stop a run while its waiter owns it | Farm (1Q) | Open |
| [BUG-025](items/BUG-025-single-run-watcher.md) | One completion watcher per run | Farm (1Q) | Open |
| [BUG-026](items/BUG-026-verification-output.md) | Verification keeps draining and can be stopped | Farm (1Q) | Open |
| [BUG-027](items/BUG-027-verified-merges.md) | Require current evidence before automatic merging | Farm (1Q) | Open |
| [BUG-028](items/BUG-028-dev-styles.md) | Development styles depend on request order | All | Fixed |

`BUG-009b` carries a suffix rather than the next number because it is the same
report, reopened on a corrected diagnosis, and BUG-009's own document keeps the
first diagnosis visible. Identifiers are not reused, so the correction needed one
of its own.

## Tasks

| ID | Title | Screen | Status |
| --- | --- | --- | --- |
| [TASK-001](items/TASK-001-records-baseline.md) | Records baseline | — | Done |
| [TASK-002](items/TASK-002-test-and-ci-baseline.md) | Test and CI baseline | — | Done |
| [TASK-003](items/TASK-003-runtime-generic-tauri-layer.md) | Make the Tauri layer generic over `Runtime` | — | Done |
| [TASK-004](items/TASK-004-rename-to-spagitty.md) | Rename the project from GitLord to Spagitty | — | Done |
| [TASK-005](items/TASK-005-branch-coverage-floor.md) | Branch coverage is below the Amendment 10 floor | — | Done |
| [TASK-007](items/TASK-007-copy-sweep.md) | Copy sweep: drop the hand-holding and the Mac notation | all | Done |
| [TASK-008](items/TASK-008-branches-footer.md) | The last self-narrating footer | 1F | Done |
| [TASK-009](items/TASK-009-drop-the-work-item-ids.md) | The interface stops naming its own work items | all | Done |
| [TASK-010](items/TASK-010-security-gate-advisories.md) | Gate 4 fails on unmaintained advisories | — | Done |
| [TASK-011](items/TASK-011-secret-scanning-never-ran.md) | Secret scanning has never run | — | Done |
| [TASK-012](items/TASK-012-record-drift.md) | The working record has drifted from the tree | — | Done |
| [TASK-013](items/TASK-013-backfill-document-sets.md) | Backfill the missing plan and testing documents | — | Done |
| [TASK-014](items/TASK-014-dead-remote.md) | The repository still points at a remote that no longer exists | — | Done |
| [TASK-015](items/TASK-015-document-drift.md) | The documents describe a Spagitty that no longer exists | — | Done |
| [TASK-016](items/TASK-016-one-branch-carried-fifteen-items.md) | One branch carried twenty-eight items | — | Done |
| [TASK-017](items/TASK-017-flow-restore.md) | Seventy-one commits with no path back to `main` | — | Done |
| [TASK-018](items/TASK-018-first-ci-run.md) | Four gates that have never seen this code | — | Done |
| [TASK-019](items/TASK-019-resume-session.md) | The launch sequence lives where no test can reach it | — | Done |
| [TASK-020](items/TASK-020-the-glass-material-settled.md) | The glass material, settled at the window | all | Done |
| [TASK-021](items/TASK-021-centre-the-name-in-the-title-bar.md) | Centre the name in the title bar | chrome | Done |
| [TASK-022](items/TASK-022-the-glass-goes-back-on-the-gpu.md) | The glass goes back on the GPU | all | Done |
| [TASK-023](items/TASK-023-flat-ui-remove-gradients.md) | Flatten UI and remove gradients | all | Done |
| [TASK-024](items/TASK-024-the-glass-reads-as-glass.md) | The glass reads as glass again | all | Done |
| [TASK-025](items/TASK-025-release-lane-amendment-20.md) | The release lane obeys Amendment 20 | — | Done |
| [TASK-026](items/TASK-026-remove-every-remaining-shadow.md) | Remove every remaining shadow | all | Done |
| [TASK-027](items/TASK-027-migrate-the-js-toolchain-to-bun.md) | Migrate the JS toolchain to bun | — | Done |
| [TASK-028](items/TASK-028-reconcile-working-record-and-docs.md) | Reconcile working record and docs | — | Done |
| [TASK-029](items/TASK-029-candidate-feature-backlog.md) | Candidate feature backlog items | — | Done |
| [TASK-030](items/TASK-030-the-farm-refresh-stops-working-on-the-main-thread.md) | The farm's refresh stops working on the main thread | Farm (1Q) | Done |
| [TASK-031](items/TASK-031-long-sessions-stay-fast.md) | A long session stays fast | Farm (1Q) | Done |
| [TASK-032](items/TASK-032-release-baseline.md) | Restore the release baseline and record the reliability plan | — | Open |
| [TASK-033](items/TASK-033-coverage-platforms.md) | Restore coverage and platform tests | All | Open |
| [TASK-036](items/TASK-036-sync-amendments.md) | Use the current shared amendments book | — | Open |

## Skipped identifiers

These were assigned and never became items. They are listed so a reader can tell
a *missing* document from an identifier that was never real, and because
Amendment 12 forbids reusing any of them.

| ID | What is known |
| --- | --- |
| FEAT-024 | Cited nowhere. Skipped when the numbers were handed out. |
| FEAT-031 | Cited once, by FEAT-035, as "changes `CommitRows`". Nothing else survives; the intent is not recoverable. Retired. |
| FEAT-032 | Cited nowhere. Skipped. |
| TASK-006 | Cited nowhere. Skipped. |

## Documents outstanding

Items that are built but do not yet have all four documents. Each row is a debt,
recorded rather than hidden; `tools/record.test.ts` fails if a built item's
missing documents are not listed here, and fails again if a row here is stale.

| ID | Missing | Why, and what closes it |
| --- | --- | --- |
| FEAT-055 | plan, automated, sweep | A rendering-path decision measured on one machine and written as a policy function with a unit test per row of the table. What is missing is a second machine: the plan and sweep are worth writing when somebody with different hardware can confirm or contradict it. |
| FEAT-056 | plan, automated, sweep | A toggle and a set beside the widths, covered incidentally by the layout round-trip in `panels.test.ts`. The plan and sweep are worth writing when the Graph and Pull requests screens get their own component tests to point at. |
| BUG-001 | plan, automated, sweep | Fixed inside FEAT-003's change before it had a branch of its own; its item document says so. No separate work to plan. |
| FEAT-073 | plan, automated, sweep | The crate landed with its own Rust tests and the screen with component tests, and the item document records the decisions a plan would have argued in advance. What is missing is the record written *before* the code, which cannot be back-dated honestly; the sweep is owed the first time a farm is driven end to end on a repository that is not this one. |

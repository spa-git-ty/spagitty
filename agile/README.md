<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Working record

This directory is the working record of what is being built, and it is
authoritative.

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
| `Dropped` | Recorded and decided against. Item document only; the status line says who decided and why. |

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
| [FEAT-079](items/FEAT-079-a-node-that-says-who.md) | A node that says who | Graph | Done |
| [FEAT-080](items/FEAT-080-follow-omarchy.md) | Follow Omarchy | Settings, all | Done |
| [FEAT-081](items/FEAT-081-the-graph-resizes-like-a-hand-moves.md) | The graph resizes like a hand moves | Graph | Done |
| [FEAT-082](items/FEAT-082-a-spatial-shell.md) | A spatial shell | chrome | Done |
| [FEAT-083](items/FEAT-083-the-content-in-the-spatial-language.md) | The content in the spatial language | Graph, all | Done |
| [FEAT-084](items/FEAT-084-the-other-screens-in-the-spatial-language.md) | The other screens in the spatial language | 1F, 1K, 1M, 1N, 1O, 1Q, all | Done |
| [FEAT-085](items/FEAT-085-a-new-mark-palette-and-wordmark.md) | A new mark, palette and wordmark | brand, chrome, 1J | Done |
| [FEAT-086](items/FEAT-086-the-pomodoro-theme.md) | The Pomodoro theme | all | Done |
| [FEAT-087](items/FEAT-087-the-review-screen-and-inbox.md) | The Review screen and its inbox | 1R, chrome | Done |
| [FEAT-088](items/FEAT-088-gitlab-as-its-api-says.md) | GitLab, as its API says | 1R, 1H, Settings | Done |
| [FEAT-089](items/FEAT-089-a-pull-request-read-from-disk.md) | A pull request read from disk | 1R | Done |
| [FEAT-090](items/FEAT-090-settings-reading.md) | Settings › Reading | 1K, 1B, 1C, 1O, 1R | Done |
| [FEAT-091](items/FEAT-091-the-review-room.md) | The review room | 1R | Done |
| [FEAT-092](items/FEAT-092-conflict-fix-origin.md) | Conflict fixes, told apart | 1R | Done |
| [FEAT-093](items/FEAT-093-threads-done-properly.md) | Threads done properly | 1R | Done |
| [FEAT-094](items/FEAT-094-review-reads-like-code.md) | Review reads like code | 1R, 1H, diff | Done |
| [FEAT-095](items/FEAT-095-check-out-a-pull-request.md) | Check out a pull request | 1R | Done |
| [FEAT-096](items/FEAT-096-an-extension-host-anyone-can-build-for.md) | An extension host anyone can build for | 1K, 1C, 1Q, 1H, palette | Done |
| [FEAT-097](items/FEAT-097-coderabbit-reviews-local-changes.md) | CodeRabbit reviews local changes | 1C, 1K, palette | Dropped |
| [FEAT-098](items/FEAT-098-coderabbit-as-a-farm-review-gate.md) | CodeRabbit as a farm review gate | Farm (1Q) | Dropped |
| [FEAT-099](items/FEAT-099-coderabbit-on-github-pull-requests.md) | CodeRabbit on GitHub pull requests | 1H | Dropped |
| [FEAT-100](items/FEAT-100-a-merger-that-shows-the-result-first.md) | A merger that shows the result first | 1S, chrome | Done |
| [FEAT-101](items/FEAT-101-merge-without-conflicts.md) | Merge without conflicts | 1S | Done |
| [FEAT-102](items/FEAT-102-resolve-every-conflict-every-way.md) | Resolve every conflict, every way | 1S, 1D | Done |
| [FEAT-103](items/FEAT-103-a-rebase-that-stops-in-merger.md) | A rebase that stops, in Merger | 1S | Done |
| [FEAT-104](items/FEAT-104-a-branch-named-where-head-is.md) | A branch named where HEAD is | chrome, 1A | Done |
| [FEAT-105](items/FEAT-105-review-from-the-pull-request-screen.md) | Review from the pull request screen | 1H, 1R | Done |
| [FEAT-106](items/FEAT-106-a-rebase-you-can-see.md) | A rebase you can see | 1E | Done |
| [FEAT-107](items/FEAT-107-hooks-you-can-see-and-skip.md) | Hooks you can see and skip | 1C, Settings | Done |
| [FEAT-108](items/FEAT-108-farm-shared-journey.md) | Farm shared journey | 1Q | Done |
| [FEAT-109](items/FEAT-109-farm-building.md) | Farm building board | 1Q | Done |
| [FEAT-110](items/FEAT-110-farm-task-journey.md) | Farm task journey | 1Q | Done |
| [FEAT-111](items/FEAT-111-farm-plan-journey.md) | Farm setup and planning | 1Q | Done |
| [FEAT-112](items/FEAT-112-farm-crew-activity.md) | Farm crew and activity | 1Q | Done |
| [FEAT-113](items/FEAT-113-farm-wrap-up.md) | Farm wrap up and evidence | 1Q | Done |
| [FEAT-114](items/FEAT-114-pull-request-notifications.md) | Pull request notifications | Settings, chrome | Done |
| [FEAT-115](items/FEAT-115-agents-in-review-and-merge.md) | Agents in Review and Merger | 1K, 1R, 1S, chrome | Done |
| [FEAT-116](items/FEAT-116-pull-requests-does-not-review.md) | Pull requests lists and reads; Review reviews | 1H, 1R | Done |
| [FEAT-117](items/FEAT-117-every-merge-happens-in-merger.md) | Every merge happens in Merger | 1S, 1H, 1A, 1Q | Done |
| [FEAT-118](items/FEAT-118-an-agents-work-shown-properly.md) | What an agent was sent and what it said, shown properly | 1R, 1S | Done |
| [FEAT-119](items/FEAT-119-an-agents-progress-you-can-read.md) | An agent's progress you can read | 1R, 1S | Done |
| [FEAT-120](items/FEAT-120-the-review-file-list-hides-and-has-two-sizes.md) | The review room's file list can be hidden, and is compact or detailed | 1R | Done |
| [FEAT-121](items/FEAT-121-what-is-running-is-said-and-what-would-collide-waits.md) | What is running is said, and what would collide waits | chrome, 1A, 1C, 1F, 1G, 1S | Done |
| [FEAT-122](items/FEAT-122-one-branch-menu-that-can-be-searched.md) | One branch menu, that can be searched and says where you are | chrome, 1S, 1E | Done |
| [FEAT-123](items/FEAT-123-merger-checks-out-what-it-merged-into-and-offers-the-push.md) | Merger checks out the branch it merged into, and offers to push it | 1S | Done |
| [FEAT-124](items/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top.md) | An agent that shows it is working, and asks at the top of its card | 1R, 1S | Done |
| [FEAT-125](items/FEAT-125-a-choice-on-a-conflict-can-be-cancelled-and-pick-lines-starts-empty.md) | A choice on a conflict can be cancelled, and Pick lines starts empty | 1S, 1D | Done |
| [FEAT-126](items/FEAT-126-merger-says-a-branch-is-behind-its-remote-and-pulls-it.md) | Merger says a branch is behind its remote, and pulls it | 1S | Done |
| [FEAT-127](items/FEAT-127-the-review-room-says-the-pull-request-moved-and-pulls-the-latest.md) | The review room says the pull request moved, and pulls the latest | 1R | Done |
| [FEAT-128](items/FEAT-128-a-conflicts-columns-scroll-together-or-each-alone.md) | A conflict's columns scroll in place, together or each alone | 1D, 1S | Done |
| [FEAT-129](items/FEAT-129-review-shows-a-file-before-and-after-side-by-side.md) | Review shows a file before and after, side by side | 1R | Done |
| [FEAT-130](items/FEAT-130-a-terminal-on-the-rail.md) | A terminal on the rail, with tmux in it | 1T | Done |
| [FEAT-131](items/FEAT-131-files-any-revision-read-never-written.md) | Files: any revision, read and never written | 1U | Done |
| [FEAT-132](items/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line.md) | Investigate: the life of a file, a folder, a branch or a line | 1O | Done |
| [FEAT-133](items/FEAT-133-recipes-git-steps-in-blocks-run-with-an-agent.md) | Recipes: git steps in blocks, run with an agent | 1V | Done |
| [FEAT-134](items/FEAT-134-pin-or-hide-what-is-on-the-rail.md) | Pin or hide what is on the rail | 1K | Done |
| [FEAT-135](items/FEAT-135-farm-sessions-in-the-terminal.md) | Farm sessions in the terminal | 1T, 1A | Dropped |
| [FEAT-136](items/FEAT-136-issues.md) | Issues beside the code | 1W | Done |
| [FEAT-137](items/FEAT-137-hooks.md) | Hooks that run in steps and are built from blocks | 1X | Done |
| [FEAT-138](items/FEAT-138-pipelines.md) | Preview workflows and follow their runs | 1Y | Done |
| [FEAT-139](items/FEAT-139-releases.md) | Draft and publish releases with their files | 1Z | Done |
| [FEAT-140](items/FEAT-140-home.md) | Home: what this repository needs from you | 2A, chrome | Done |
| [FEAT-141](items/FEAT-141-starting-a-farm-one-step-at-a-time.md) | Starting a farm, one step at a time | 1Q | Done |
| [FEAT-142](items/FEAT-142-a-notification-center.md) | A notification center that keeps what it said | chrome, 1K | Done |
| [FEAT-143](items/FEAT-143-a-slice-per-task.md) | A slice per task | 1Q | Done |
| [FEAT-144](items/FEAT-144-the-landing-lane.md) | The landing lane: merging without a queue, and a log of every branch move | 1Q | Done |
| [FEAT-145](items/FEAT-145-history-as-tools-for-agents.md) | History as tools for agents | 1Q | Done |
| [FEAT-146](items/FEAT-146-a-conflict-resolver-chosen-in-rules.md) | A conflict resolver chosen in Rules | 1Q | Done |
| [FEAT-147](items/FEAT-147-own-copies-and-intake.md) | Own copies, and intake at the door | 1Q | Done |
| [FEAT-148](items/FEAT-148-arrange-home-s-cards-in-settings.md) | Arrange Home's cards in Settings | Settings, 2A | Done |
| [FEAT-149](items/FEAT-149-a-coloured-tag-for-each-kind-of-file.md) | A coloured tag for each kind of file | 1U | Done |
| [FEAT-150](items/FEAT-150-open-a-file-from-anywhere-with-a-click.md) | Open a file from anywhere with a click | every screen | Done |
| [FEAT-151](items/FEAT-151-search-the-text-of-files.md) | Search the text of files, by word or regular expression | 1U | Done |
| [FEAT-152](items/FEAT-152-the-files-tree-widens-and-copies.md) | The Files tree widens by dragging, copies from a right-click, and a long selection holds | 1U | Done |
| [FEAT-153](items/FEAT-153-blame-on-click-can-be-turned-off.md) | Blame on click can be turned off | 1U | Done |
| [FEAT-154](items/FEAT-154-usages-of-a-name.md) | Usages: where a name is defined and used | 1U | Done |
| [FEAT-155](items/FEAT-155-untrack.md) | Untrack: stop tracking files on a branch, in a commit, and push it | 2B | Done |
| [FEAT-156](items/FEAT-156-calmer-palettes-and-an-accent-of-your-own.md) | Calmer light and dark palettes, and an accent of your own | every screen | Done |
| [FEAT-157](items/FEAT-157-close-a-terminal-session-and-give-tmux-our-colours.md) | Close a terminal session, and give tmux Spagitty's colours | 1T | Done |
| [FEAT-158](items/FEAT-158-type-to-search-any-list.md) | Type to search any list | every screen with a list | Done |
| [FEAT-159](items/FEAT-159-clear-all-notifications.md) | Clear all notifications | the notification drawer | Done |
| [FEAT-160](items/FEAT-160-calls-send-a-rest-call-and-keep-the-collection-in-the-repository.md) | Calls: send a REST call and keep the collection in the repository | 2C | Open |
| [FEAT-161](items/FEAT-161-environments-secrets-and-values-kept-from-responses.md) | Environments, secrets and values kept from responses | 2C | Backlog |
| [FEAT-162](items/FEAT-162-import-calls-from-code-and-from-collection-files.md) | Import calls from code and from collection files | 2C | Backlog |
| [FEAT-163](items/FEAT-163-soap-calls-from-a-wsdl.md) | SOAP calls from a WSDL | 2C | Backlog |
| [FEAT-164](items/FEAT-164-share-a-collection-as-a-file-a-zip-or-a-commit.md) | Share a collection as a file, a zip or a commit | 2C | Backlog |
| [FEAT-165](items/FEAT-165-settings-calls.md) | Settings › Calls | 1K | Backlog |
| [FEAT-166](items/FEAT-166-copy-a-call-as-curl-and-paste-one-in.md) | Copy a call as cURL, and paste one in | 2C | Backlog |
| [FEAT-167](items/FEAT-167-signed-ws-security.md) | Signed WS-Security for SOAP calls | 2C, 1K | Backlog |

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
| [BUG-024](items/BUG-024-cancellable-runs.md) | Stop a run while its waiter owns it | Farm (1Q) | Fixed |
| [BUG-025](items/BUG-025-single-run-watcher.md) | One completion watcher per run | Farm (1Q) | Fixed |
| [BUG-026](items/BUG-026-verification-output.md) | Verification keeps draining and can be stopped | Farm (1Q) | Fixed |
| [BUG-027](items/BUG-027-verified-merges.md) | Require current evidence before automatic merging | Farm (1Q) | Fixed |
| [BUG-028](items/BUG-028-dev-styles.md) | Development styles depend on request order | All | Fixed |
| [BUG-029](items/BUG-029-the-window-is-ringed-with-blurred-desktop.md) | The packaged Linux window is ringed with blurred desktop | All | Fixed |
| [BUG-030](items/BUG-030-the-toolbar-offers-what-it-cannot-do.md) | The toolbar offers what it cannot do, and hides what it can | chrome | Fixed |
| [BUG-031](items/BUG-031-the-window-opens-in-the-wrong-theme.md) | The window opens in the wrong theme, then changes its mind | All | Fixed |
| [BUG-032](items/BUG-032-a-path-test-that-only-passes-on-unix.md) | A path test that only passes on Unix | none | Fixed |
| [BUG-033](items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md) | A CRLF file diffs as a rewrite | Working copy (1C) | Fixed |
| [BUG-034](items/BUG-034-a-transparent-band-round-the-window-on-windows.md) | A transparent band round the window on Windows | All | Fixed |
| [BUG-035](items/BUG-035-native-scrollbars-and-a-smudged-tab.md) | Native scrollbars, and a smudged tab | All | Fixed |
| [BUG-036](items/BUG-036-the-commit-list-a-size-above-the-rest.md) | The commit list, a size above the rest | Graph (1A) | Fixed |
| [BUG-037](items/BUG-037-core-tests-read-the-machines-git-config.md) | Core tests read the machine's git config | none | Fixed |
| [BUG-038](items/BUG-038-interactive-rebase-cannot-start-on-windows.md) | Interactive rebase cannot start on Windows | 1E | Fixed |
| [BUG-039](items/BUG-039-a-graph-test-that-reads-the-clock.md) | A graph test that reads the clock | none | Fixed |
| [BUG-040](items/BUG-040-the-shells-pane-class-leaks-into-five-components.md) | The shell's pane class leaks into five components | 1B, 1C, 1D, 1G, 1Q | Fixed |
| [BUG-041](items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md) | The pinned header cuts the detail card's shadow | 1A | Fixed |
| [BUG-042](items/BUG-042-a-corporate-ca-is-not-trusted.md) | A corporate CA is not trusted | Settings, Pull requests | Fixed |
| [BUG-043](items/BUG-043-a-gitlab-token-is-checked-as-github.md) | A GitLab token is checked as GitHub | Settings | Fixed |
| [BUG-044](items/BUG-044-start-review-does-nothing-on-gitlab.md) | Start review does nothing on GitLab | 1R | Fixed |
| [BUG-045](items/BUG-045-the-name-leaves-the-title-bar.md) | The name leaves the title bar | All | Fixed |
| [BUG-046](items/BUG-046-a-console-window-opens-for-git.md) | A console window opens for git | All (Windows) | Fixed |
| [BUG-047](items/BUG-047-whole-file-scrolls-back-to-the-top.md) | Whole file scrolls back to the top | 1R | Fixed |
| [BUG-048](items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md) | A wider graph column draws no more lanes | 1A | Fixed |
| [BUG-049](items/BUG-049-small-monospace-text-ignores-the-reading-font.md) | Small monospace text ignores the reading font | All | Fixed |
| [BUG-050](items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md) | The Mac check waits for a licence agreement | CI | Fixed |
| [BUG-051](items/BUG-051-what-the-author-saw-on-first-run.md) | What the author saw on first run | 1S, 1D, 1F, 1A | Fixed |
| [BUG-052](items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md) | Squeezed lanes, silent waits and striped notices | 1A, all, chrome | Fixed |
| [BUG-053](items/BUG-053-what-the-author-found-sweeping.md) | What the author found sweeping | 1H, 1A, chrome | Fixed |
| [BUG-054](items/BUG-054-review-loader-conversation-panel-and-scrollbars.md) | Review's loader, its Conversation card, and scrollbars at rest | 1R, all | Fixed |
| [BUG-055](items/BUG-055-branch-where-you-point.md) | Branch where you point, stash in place, and a working copy that is noticed | chrome, 1A, 1R, 1H, 1C | Fixed |
| [BUG-056](items/BUG-056-review-requests-and-profile-navigation.md) | Review and Pull requests align, and profiles open in Settings | chrome, 1R, 1H | Fixed |
| [BUG-057](items/BUG-057-your-own-pull-requests-in-review.md) | Review lists your own pull requests | 1R | Fixed |
| [BUG-058](items/BUG-058-conversation-reads-in-full.md) | A conversation thread can be read in full | 1R | Fixed |
| [BUG-059](items/BUG-059-windows-watcher-never-sees-refs.md) | On Windows the watcher never sees a ref move, and refreshes forever | chrome, 1A, 1S | Fixed |
| [BUG-060](items/BUG-060-merger-forecast-lost-on-first-open.md) | Merger sits on "Working out the merge…" when it is first opened | 1S | Fixed |
| [BUG-061](items/BUG-061-merger-shows-the-previous-pairs-forecast.md) | Merger shows the previous pair's forecast under a new branch name | 1S | Fixed |
| [BUG-062](items/BUG-062-after-a-merge-another-pull-request-opens-empty.md) | After a merge, Pull requests opens another pull request with nothing in it | 1H | Fixed |
| [BUG-063](items/BUG-063-a-conflicting-pull-request-is-not-marked.md) | A pull request that conflicts with its base is not marked, and Merge only fails | 1R, 1H, 1S | Fixed |
| [BUG-064](items/BUG-064-review-says-nobody-asked-you-after-you-answered.md) | Review says "nobody asked you yet" about pull requests you have already reviewed | 1R | Fixed |
| [BUG-065](items/BUG-065-agent-cli-access.md) | Command-line agents cannot start or finish reviews correctly | Settings, Review, Merger, Farm | Fixed |
| [BUG-066](items/BUG-066-extensions-refuse-spagitty-2.md) | Extensions refuse to run on Spagitty 2 | 1K | Fixed |
| [BUG-067](items/BUG-067-review-says-the-token-is-invalid.md) | Review says the token is invalid until Pull requests has been opened | 1R | Fixed |
| [BUG-068](items/BUG-068-ordinary-commits-marked-as-conflict-fixes.md) | Ordinary commits are marked as conflict fixes | 1R | Fixed |
| [BUG-069](items/BUG-069-added-lines-look-red.md) | Added lines sometimes look red | 1R, 1B | Fixed |
| [BUG-070](items/BUG-070-local-agents-do-not-get-their-instructions.md) | Local agents do not get their instructions, and some cannot start in Review or Merger | 1R, 1S, 1K | Fixed |
| [BUG-071](items/BUG-071-a-gemini-agent-gets-bad-request.md) | A Gemini agent from Google AI Studio gets "Bad request" | 1K, 1R, 1S | Fixed |
| [BUG-072](items/BUG-072-a-remote-agent-with-no-key-is-told-bad-request.md) | A remote agent with no key is told "Bad request", not that its key is missing | 1K, 1R, 1S | Fixed |
| [BUG-073](items/BUG-073-two-quick-assign-clicks-start-two-agents.md) | Two quick Assign clicks start two agents on one pull request | 1R, 1S | Fixed |
| [BUG-074](items/BUG-074-a-remote-agents-token-limit-is-read-only-between-steps.md) | A remote agent's token limit is read only between steps | 1K, 1R, 1S | Fixed |
| [BUG-075](items/BUG-075-a-reply-that-cannot-be-read-is-taken-as-empty.md) | A model's reply that cannot be read is taken as an empty one | 1K, 1R, 1S | Fixed |
| [BUG-076](items/BUG-076-merger-says-pushed-when-the-push-failed.md) | Merger says *Pushed* when the push failed, and no fetch or push works after it | 1S | Fixed |
| [BUG-077](items/BUG-077-a-farm-task-merged-in-merger-skips-the-farms-checks.md) | A Farm task merged in Merger skips the Farm's own checks | 1Q, 1S | Fixed |
| [BUG-078](items/BUG-078-the-attribution-check-refuses-a-person-named-like-an-agent.md) | The attribution check refuses a person whose name an agent shares | — | Fixed |
| [BUG-079](items/BUG-079-all-resolved-is-drawn-as-a-button.md) | *All resolved* is drawn as a button | 1D, 1S | Fixed |
| [BUG-080](items/BUG-080-the-review-pill-runs-out-of-a-narrow-diff.md) | The review pill runs out of a narrow diff, over the conversation | 1R, 1D, 1S | Fixed |
| [BUG-081](items/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why.md) | Push of a new branch fails and does not say why | chrome, 1A | Fixed |
| [BUG-082](items/BUG-082-saving-the-git-identity-clears-the-fields.md) | Saving the git identity clears the fields | 1K | Fixed |
| [BUG-083](items/BUG-083-hiding-the-blame-in-files-breaks-every-line.md) | Hiding the blame in Files breaks every line | 1U | Fixed |
| [BUG-084](items/BUG-084-pipelines-live-shows-empty-trace-panel.md) | Pipelines Live still draws the trace panel when a branch has no runs | 1Y | Fixed |
| [BUG-085](items/BUG-085-hooks-runs-step-list-collapses-when-detail-hidden.md) | Hooks Runs step list collapses when the detail panel is hidden | 1X | Fixed |
| [BUG-086](items/BUG-086-pipelines-preview-target-branch-input-no-placeholder.md) | Pipelines Preview target branch input has no placeholder | 1Y | Fixed |
| [BUG-087](items/BUG-087-ci-yaml-error-mislabeled-as-git-config.md) | A repeated key empties the GitLab preview, and says it is a git-configuration error | 1Y | Fixed |
| [BUG-088](items/BUG-088-ci-problem-file-badge-wraps-on-hyphen.md) | CI problem file badge wraps awkwardly on the hyphen | 1Y | Fixed |
| [BUG-089](items/BUG-089-releases-misleading-empty-state-on-api-error.md) | Releases on GitLab is always empty: "responded 302" | 1Z | Fixed |
| [BUG-090](items/BUG-090-pipelines-screen-does-not-match-its-artboards.md) | Pipelines does not look like its artboards, and Preview takes half a minute | 1Y | Fixed |
| [BUG-091](items/BUG-091-git-s-push-line-stays-beside-the-branch.md) | git's push line stays beside the branch, hashes and all | chrome | Fixed |
| [BUG-092](items/BUG-092-nothing-can-be-dragged-into-place-on-windows.md) | Nothing can be dragged into place on Windows | Settings, 1E, 1X, 1A | Fixed |
| [BUG-093](items/BUG-093-the-farm-wizard-s-search-box-and-step-markers-are-broken.md) | The farm wizard's search box and step markers are broken | 1Q | Fixed |
| [BUG-094](items/BUG-094-a-resumed-agent-skips-files-that-changed-since.md) | A resumed agent skips files that changed since its first run | 1R, 1S | Fixed |
| [BUG-095](items/BUG-095-the-terminal-s-plus-says-no-server-running.md) | The Terminal's `+` says *no server running on session* | 1T | Fixed |
| [BUG-096](items/BUG-096-the-working-copy-s-hunks-are-not-coloured.md) | The working copy's hunks are not coloured | Working copy (1C) | Fixed |
| [BUG-097](items/BUG-097-a-push-opens-the-credential-helpers-sign-in-window.md) | A push opens the credential helper's sign-in window instead of using the connected account | every push | Fixed |
| [BUG-098](items/BUG-098-pull-requests-says-only-500-internal-server-error.md) | Pull requests says only "500 Internal Server Error" on the author's GitLab | 1H, 1R | Fixed |
| [BUG-099](items/BUG-099-the-hooks-side-panel-cards-do-nothing.md) | The Hooks screen's side panel cards do nothing when clicked | 1X | Fixed |

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
| [TASK-032](items/TASK-032-release-baseline.md) | Restore the release baseline and record the reliability plan | — | Done |
| [TASK-033](items/TASK-033-coverage-platforms.md) | Restore coverage and platform tests | All | Done |
| [TASK-036](items/TASK-036-sync-amendments.md) | Use the current shared amendments book | — | Done |
| [TASK-037](items/TASK-037-updates-that-actually-arrive.md) | Updates that actually arrive | 1K | Done |
| [TASK-038](items/TASK-038-settings-that-stop-lecturing.md) | Settings that stop lecturing | Settings | Done |
| [TASK-039](items/TASK-039-one-colour-system-in-every-dialog.md) | One colour system in every dialog | all dialogs | Done |
| [TASK-040](items/TASK-040-the-macos-download-says-what-it-is.md) | The macOS download says what it is | — | Done |
| [TASK-041](items/TASK-041-the-frame-stops-competing-with-the-work.md) | The frame stops competing with the work | chrome, Graph | Done |
| [TASK-042](items/TASK-042-one-control-scale-and-a-platform-policy.md) | One control scale, and a window that knows its platform | chrome, all | Done |
| [TASK-043](items/TASK-043-the-linux-renderer-and-the-accessibility-bridge.md) | The Linux renderer and the accessibility bridge | — | Backlog |
| [TASK-044](items/TASK-044-settings-stops-explaining-itself.md) | Settings stops explaining itself | Settings | Done |
| [TASK-045](items/TASK-045-a-rail-of-five.md) | A rail of five | chrome | Done |
| [TASK-046](items/TASK-046-lists-you-can-read.md) | Lists you can read | Working copy, Stash, Diff | Done |
| [TASK-047](items/TASK-047-a-commit-bar.md) | A commit bar | Working copy (1C) | Done |
| [TASK-048](items/TASK-048-commands-off-the-main-thread.md) | Commands off the main thread | all | Done |
| [TASK-049](items/TASK-049-the-record-catches-up-with-what-merged.md) | The record catches up with what merged | — | Done |
| [TASK-050](items/TASK-050-the-chatter-goes.md) | The chatter goes | 1K, all | Done |
| [TASK-051](items/TASK-051-one-theme.md) | One theme | 1K, all | Done |
| [TASK-052](items/TASK-052-nothing-waits-in-line.md) | Nothing waits in line | all | Open |
| [TASK-053](items/TASK-053-the-review-room-after-first-use.md) | The review room after first use | 1R | Done |
| [TASK-054](items/TASK-054-build-macos-on-a-mac.md) | Build macOS on a Mac | — | Done |
| [TASK-055](items/TASK-055-extensions-ship-in-the-release.md) | Extensions ship in the release | — | Dropped |
| [TASK-056](items/TASK-056-the-rail-as-the-author-chose-it.md) | The rail as the author chose it | chrome, 1A | Done |
| [TASK-057](items/TASK-057-the-log-screen-in-the-house-style.md) | The Log screen in the house style | 1I | Done |
| [TASK-058](items/TASK-058-restore-the-patch-release.md) | Restore the patch release after the bug-fix merges | CI | Done |
| [TASK-059](items/TASK-059-reading-font-and-ruler-off-by-default.md) | The reading font and the focus ruler are off by default, and off the review bar | 1R, 1K | Done |
| [TASK-060](items/TASK-060-agent-permissions-with-standard-names.md) | Agent permissions with standard names | 1K, 1R, 1S | Done |
| [TASK-061](items/TASK-061-plain-words-for-merging-and-approval.md) | Plain words for merging and stopping for approval | all | Done |
| [TASK-062](items/TASK-062-finish-the-open-pull-requests.md) | Finish the three open pull requests | — | Done |
| [TASK-063](items/TASK-063-audit-the-merged-pull-requests.md) | Audit the pull requests merged since 2.0.0 | — | Done |
| [TASK-064](items/TASK-064-refuse-agent-attribution-in-ci.md) | Refuse agent attribution in commits and descriptions, in CI | — | Done |
| [TASK-065](items/TASK-065-run-the-agents-sweep-where-it-was-skipped.md) | Run the agents sweep that FEAT-115 was closed without | 1K, 1R, 1S | Backlog |
| [TASK-066](items/TASK-066-style-notes-from-the-2-0-audit.md) | Style notes from the 2.0 audit | — | Done |
| [TASK-067](items/TASK-067-the-attribution-check-does-not-read-who-wrote-a-commit.md) | The attribution check does not read who wrote a commit | — | Done |
| [TASK-068](items/TASK-068-what-coderabbit-found-on-63-and-64.md) | What CodeRabbit found on pull requests #63 and #64 | 1S, 1R, 1K, 1Q | Done |
| [TASK-069](items/TASK-069-the-two-modals-that-choose-a-branch-use-the-branch-menu.md) | The two modals that choose a branch use the branch menu | 1H, 1J | Done |
| [TASK-070](items/TASK-070-the-plans-file-list-scrolls-in-a-card-the-height-of-its-neighbours.md) | The plan's file list scrolls, in a card the height of its neighbours | 1S | Done |
| [TASK-071](items/TASK-071-remove-agy.md) | agy is removed | 1K, 1Q, 1R, 1S | Done |
| [TASK-072](items/TASK-072-the-declared-minimum-rust-is-older-than-a-dependency-needs.md) | The declared minimum Rust is older than a dependency needs | — | Done |
| [TASK-073](items/TASK-073-the-sweeps-refreshed-and-run-from-a-portal.md) | The sweeps, refreshed and run from a portal | — | Done |
| [TASK-074](items/TASK-074-pull-requests-no-longer-list-gates-5-and-6.md) | Pull requests no longer list gates 5 and 6 | — | Done |
| [TASK-075](items/TASK-075-remove-coderabbit.md) | Remove CodeRabbit | 1Q, 1A, Settings | Done |

## Skipped identifiers

These were assigned and never became items. They are listed so a reader can tell
a *missing* document from an identifier that was never real, and because
none of them may be reused.

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
| BUG-043 | plan, automated, sweep | Inherited from main: the merged fix has no separate verification records. A retrospective verification record is owed. |
| BUG-001 | plan, automated, sweep | Fixed inside FEAT-003's change before it had a branch of its own; its item document says so. No separate work to plan. |
| FEAT-073 | plan, automated, sweep | The crate landed with its own Rust tests and the screen with component tests, and the item document records the decisions a plan would have argued in advance. What is missing is the record written *before* the code, which cannot be back-dated honestly; the sweep is owed the first time a farm is driven end to end on a repository that is not this one. |
| FEAT-114 | plan, automated, sweep | Built in one session against a request to ship it in 1.3.0. The change detection has unit tests in `changes.test.ts` and the host parsing in `watch.rs`, but there is no plan written ahead and no automated record; the sweep is owed on Windows, macOS and Linux, including whether an ad-hoc signed macOS build can post notifications. |
| BUG-057 | plan, automated, sweep | Fixed in one session from a report, so there is no plan or automated record. Covered by `inbox.test.ts` and the Review page tests; the sweep against a repository whose open pull requests are all the reader's own is owed. |
| BUG-058 | plan, automated, sweep | Fixed in one session from a report, so there is no plan or automated record. Covered by a room test with a long whole-PR comment and a reply; how the clipped text is measured in a real window is owed to the sweep. |
| BUG-084 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-085 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-086 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-087 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-088 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-089 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-090 | plan, automated, sweep | Fixed in one session from a report; covered by the change. Verification records owed. |
| BUG-091 | plan, automated, sweep | Fixed in one session from a report; covered by `network/plain.test.ts` and the store tests. Verification records owed. |
| BUG-092 | plan, automated, sweep | Fixed in one session from a report; covered by the Rail and rebase plan drag tests. The sweep on Windows, with a real pointer, is owed. |
| BUG-093 | plan, automated, sweep | Fixed in one session from a screenshot; a CSS precedence fix the test DOM cannot see. The sweep is owed: the author is checking it. |
| FEAT-148 | plan, automated, sweep | Built in one session from a request, with no plan written ahead. Covered by `settings/home-section.test.ts`, the layout and screen cases in `home/home.test.ts` and `settings.rs`. The sweep is owed: the author is checking it. |
| FEAT-149 | plan, automated, sweep | Built in one session from a request, with no plan written ahead. Covered by `files/kind.test.ts`. The sweep, in light and dark, is owed: the author is checking it. |
| FEAT-150 | plan, automated, sweep | Built in one session from a request, with no plan written ahead. Covered by the click and double-click cases in `files/open.test.ts`. The sweep across the screens is owed: the author is checking it. |

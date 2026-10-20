<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Manual sweeps

Every manual sweep, one row each, with how far it has got. **Generated**: the
sweep portal writes it after every result, and `bun run sweep:index` writes it
by hand. Do not edit it; edit the sweep documents.

`bun run sweep` opens the portal: each ticket with its steps, a result to set
(passed, failed, blocked, partly passed, obsolete), a note, and screenshots or a
screen recording to attach. Results are written into the sweep documents
themselves. Attached files are kept under `agile/testing/evidence/`, which is
not committed.

**1899 tickets in 291 sweeps:** 194 passed, 0 failed, 0 blocked, 0 partly passed, 42 obsolete, 1661 not run, 2 with a note only.

| Item | Title | Status | Tickets | Passed | Failed | Blocked | Obsolete | Not run | Sweep |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FEAT-001 | Graph screen | Done | 15 | 0 | 0 | 0 | 0 | 15 | [FEAT-001-sweep.md](FEAT-001-sweep.md) |
| FEAT-002 | Diff screen | Done | 17 | 0 | 0 | 0 | 0 | 17 | [FEAT-002-sweep.md](FEAT-002-sweep.md) |
| FEAT-003 | Working copy / Commit | Done | 20 | 0 | 0 | 0 | 0 | 20 | [FEAT-003-sweep.md](FEAT-003-sweep.md) |
| FEAT-004 | Branches | Done | 16 | 0 | 0 | 0 | 1 | 15 | [FEAT-004-sweep.md](FEAT-004-sweep.md) |
| FEAT-005 | Stash | Done | 13 | 0 | 0 | 0 | 1 | 12 | [FEAT-005-sweep.md](FEAT-005-sweep.md) |
| FEAT-006 | All repositories | Done | 15 | 0 | 0 | 0 | 0 | 15 | [FEAT-006-sweep.md](FEAT-006-sweep.md) |
| FEAT-007 | Log search | Done | 17 | 0 | 0 | 0 | 0 | 17 | [FEAT-007-sweep.md](FEAT-007-sweep.md) |
| FEAT-008 | Conflicts | Done | 15 | 0 | 0 | 0 | 2 | 13 | [FEAT-008-sweep.md](FEAT-008-sweep.md) |
| FEAT-009 | Interactive rebase | Done | 16 | 0 | 0 | 0 | 2 | 14 | [FEAT-009-sweep.md](FEAT-009-sweep.md) |
| FEAT-010 | Pull requests | Done | 9 | 0 | 0 | 0 | 7 | 2 | [FEAT-010-sweep.md](FEAT-010-sweep.md) |
| FEAT-011 | Settings | Done | 18 | 0 | 0 | 0 | 1 | 17 | [FEAT-011-sweep.md](FEAT-011-sweep.md) |
| FEAT-012 | Clone | Done | 16 | 0 | 0 | 0 | 0 | 16 | [FEAT-012-sweep.md](FEAT-012-sweep.md) |
| FEAT-013 | Branch delete and rename | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-013-sweep.md](FEAT-013-sweep.md) |
| FEAT-014 | Stash pop, apply and drop | Done | 7 | 0 | 0 | 0 | 0 | 7 | [FEAT-014-sweep.md](FEAT-014-sweep.md) |
| FEAT-015 | Rebase execution | Done | 11 | 0 | 0 | 0 | 0 | 11 | [FEAT-015-sweep.md](FEAT-015-sweep.md) |
| FEAT-016 | Conflict resolution writes | Done | 12 | 0 | 0 | 0 | 0 | 12 | [FEAT-016-sweep.md](FEAT-016-sweep.md) |
| FEAT-017 | Forge integration | Done | 11 | 0 | 0 | 0 | 0 | 11 | [FEAT-017-sweep.md](FEAT-017-sweep.md) |
| FEAT-018 | Fetch and push | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-018-sweep.md](FEAT-018-sweep.md) |
| FEAT-019 | Commit signing | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-019-sweep.md](FEAT-019-sweep.md) |
| FEAT-020 | Show the git command behind each action | Done | 16 | 0 | 0 | 0 | 0 | 16 | [FEAT-020-sweep.md](FEAT-020-sweep.md) |
| FEAT-021 | Themes, and a title bar that stops lying | Done | 11 | 0 | 0 | 0 | 4 | 7 | [FEAT-021-sweep.md](FEAT-021-sweep.md) |
| FEAT-022 | The graph as a launcher, not a report | Done | 34 | 0 | 0 | 0 | 1 | 33 | [FEAT-022-sweep.md](FEAT-022-sweep.md) |
| FEAT-023 | Author heads, and a graph column of its own | Done | 14 | 0 | 0 | 0 | 0 | 14 | [FEAT-023-sweep.md](FEAT-023-sweep.md) |
| FEAT-025 | A table you can size, and one that scrolls sideways | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-025-sweep.md](FEAT-025-sweep.md) |
| FEAT-026 | The nav rail collapses to icons | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-026-sweep.md](FEAT-026-sweep.md) |
| FEAT-027 | Repository tabs in the title bar | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-027-sweep.md](FEAT-027-sweep.md) |
| FEAT-028 | The toolbar: centred, grouped, and without Commit | Done | 4 | 0 | 0 | 0 | 1 | 3 | [FEAT-028-sweep.md](FEAT-028-sweep.md) |
| FEAT-029 | Bigger faces, and a type scale the rail follows | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-029-sweep.md](FEAT-029-sweep.md) |
| FEAT-030 | The rail's open-repository slot | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-030-sweep.md](FEAT-030-sweep.md) |
| FEAT-033 | Branch divergence on the chip | Done | 7 | 0 | 0 | 0 | 0 | 7 | [FEAT-033-sweep.md](FEAT-033-sweep.md) |
| FEAT-034 | Browse a stash entry file by file | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-034-sweep.md](FEAT-034-sweep.md) |
| FEAT-035 | Lanes past the cap compress instead of stacking | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-035-sweep.md](FEAT-035-sweep.md) |
| FEAT-036 | One chip per branch, local and remote as icons | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-036-sweep.md](FEAT-036-sweep.md) |
| FEAT-037 | The window has depth, and every panel resizes | Done | 7 | 0 | 0 | 0 | 0 | 7 | [FEAT-037-sweep.md](FEAT-037-sweep.md) |
| FEAT-038 | Pull | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-038-sweep.md](FEAT-038-sweep.md) |
| FEAT-039 | The graph column resizes, and the lanes compress into it | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-039-sweep.md](FEAT-039-sweep.md) |
| FEAT-040 | The graph's footer says what is true, not what to do | Done | 7 | 0 | 0 | 0 | 0 | 7 | [FEAT-040-sweep.md](FEAT-040-sweep.md) |
| FEAT-041 | The rail stops advertising a shortcut | Done | 3 | 0 | 0 | 0 | 0 | 3 | [FEAT-041-sweep.md](FEAT-041-sweep.md) |
| FEAT-042 | Tighter corners, and a cast shadow that follows them | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-042-sweep.md](FEAT-042-sweep.md) |
| FEAT-043 | A status strip along the bottom of the window | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-043-sweep.md](FEAT-043-sweep.md) |
| FEAT-044 | The repository tabs get a row of their own | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-044-sweep.md](FEAT-044-sweep.md) |
| FEAT-045 | The toolbar names the repository, and picks a branch for real | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-045-sweep.md](FEAT-045-sweep.md) |
| FEAT-046 | Squeezing the graph column must not shrink the portraits | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-046-sweep.md](FEAT-046-sweep.md) |
| FEAT-047 | The branches table: resizable columns, and a divergence worth reading | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-047-sweep.md](FEAT-047-sweep.md) |
| FEAT-048 | Discard changes | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-048-sweep.md](FEAT-048-sweep.md) |
| FEAT-049 | Remotes management | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-049-sweep.md](FEAT-049-sweep.md) |
| FEAT-050 | Reflog view | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-050-sweep.md](FEAT-050-sweep.md) |
| FEAT-051 | Tags list | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-051-sweep.md](FEAT-051-sweep.md) |
| FEAT-052 | The graph reads at depth | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-052-sweep.md](FEAT-052-sweep.md) |
| FEAT-053 | Lanes turn square | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-053-sweep.md](FEAT-053-sweep.md) |
| FEAT-054 | Tell people when there is a newer Spagitty | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-054-sweep.md](FEAT-054-sweep.md) |
| FEAT-057 | Liquid glass: the pane that bends what is behind it | Done | 7 | 0 | 0 | 0 | 7 | 0 | [FEAT-057-sweep.md](FEAT-057-sweep.md) |
| FEAT-058 | A pull request you can read and answer | Done | 11 | 0 | 0 | 0 | 3 | 8 | [FEAT-058-sweep.md](FEAT-058-sweep.md) |
| FEAT-059 | Dedicated pull request review workspace | Done | 1 | 0 | 0 | 0 | 1 | 0 | [FEAT-059-sweep.md](FEAT-059-sweep.md) |
| FEAT-060 | Spagitty, with a face: the brand | Done | 1 | 0 | 0 | 0 | 1 | 0 | [FEAT-060-sweep.md](FEAT-060-sweep.md) |
| FEAT-061 | Brand guide and interactive showcase | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-061-sweep.md](FEAT-061-sweep.md) |
| FEAT-062 | Worktrees management | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-062-sweep.md](FEAT-062-sweep.md) |
| FEAT-063 | File history and blame view | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-063-sweep.md](FEAT-063-sweep.md) |
| FEAT-064 | Diff syntax highlighting | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-064-sweep.md](FEAT-064-sweep.md) |
| FEAT-065 | Image and binary diffs | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-065-sweep.md](FEAT-065-sweep.md) |
| FEAT-066 | Diff content search | Done | 4 | 4 | 0 | 0 | 0 | 0 | [FEAT-066-sweep.md](FEAT-066-sweep.md) |
| FEAT-067 | Submodules management | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-067-sweep.md](FEAT-067-sweep.md) |
| FEAT-068 | External diff and merge tool launchers | Done | 4 | 4 | 0 | 0 | 0 | 0 | [FEAT-068-sweep.md](FEAT-068-sweep.md) |
| FEAT-069 | Multi-identity profiles | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-069-sweep.md](FEAT-069-sweep.md) |
| FEAT-070 | Extended forge integration | Done | 5 | 5 | 0 | 0 | 0 | 0 | [FEAT-070-sweep.md](FEAT-070-sweep.md) |
| FEAT-072 | The delight layer: badges, titles and reward moments | Done | 35 | 35 | 0 | 0 | 0 | 0 | [FEAT-072-sweep.md](FEAT-072-sweep.md) |
| FEAT-074 | The activity drawer: a log worth reading | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-074-sweep.md](FEAT-074-sweep.md) |
| FEAT-075 | The queue explains itself | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-075-sweep.md](FEAT-075-sweep.md) |
| FEAT-076 | The farm takes on large work | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-076-sweep.md](FEAT-076-sweep.md) |
| FEAT-077 | The farm is worth watching | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-077-sweep.md](FEAT-077-sweep.md) |
| FEAT-078 | Who asked for this task | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-078-sweep.md](FEAT-078-sweep.md) |
| FEAT-079 | A node that says who | Done | 14 | 2 | 0 | 0 | 0 | 12 | [FEAT-079-sweep.md](FEAT-079-sweep.md) |
| FEAT-080 | Follow Omarchy | Done | 14 | 0 | 0 | 0 | 0 | 14 | [FEAT-080-sweep.md](FEAT-080-sweep.md) |
| FEAT-081 | The graph resizes like a hand moves | Done | 15 | 4 | 0 | 0 | 0 | 11 | [FEAT-081-sweep.md](FEAT-081-sweep.md) |
| FEAT-082 | A spatial shell | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-082-sweep.md](FEAT-082-sweep.md) |
| FEAT-083 | The content in the spatial language | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-083-sweep.md](FEAT-083-sweep.md) |
| FEAT-084 | The other screens in the spatial language | Done | 10 | 7 | 0 | 0 | 0 | 3 | [FEAT-084-sweep.md](FEAT-084-sweep.md) |
| FEAT-085 | A new mark, palette and wordmark | Done | 7 | 2 | 0 | 0 | 0 | 5 | [FEAT-085-sweep.md](FEAT-085-sweep.md) |
| FEAT-086 | The Pomodoro theme | Done | 3 | 3 | 0 | 0 | 0 | 0 | [FEAT-086-sweep.md](FEAT-086-sweep.md) |
| FEAT-087 | The Review screen and its inbox | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-087-sweep.md](FEAT-087-sweep.md) |
| FEAT-088 | GitLab, as its API says | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-088-sweep.md](FEAT-088-sweep.md) |
| FEAT-089 | A pull request read from disk | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-089-sweep.md](FEAT-089-sweep.md) |
| FEAT-090 | Settings › Reading | Done | 6 | 1 | 0 | 0 | 0 | 5 | [FEAT-090-sweep.md](FEAT-090-sweep.md) |
| FEAT-091 | The review room | Done | 8 | 1 | 0 | 0 | 0 | 7 | [FEAT-091-sweep.md](FEAT-091-sweep.md) |
| FEAT-092 | Conflict fixes, told apart | Done | 6 | 1 | 0 | 0 | 0 | 5 | [FEAT-092-sweep.md](FEAT-092-sweep.md) |
| FEAT-093 | Threads done properly | Done | 7 | 1 | 0 | 0 | 0 | 6 | [FEAT-093-sweep.md](FEAT-093-sweep.md) |
| FEAT-094 | Review reads like code | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-094-sweep.md](FEAT-094-sweep.md) |
| FEAT-095 | Check out a pull request | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-095-sweep.md](FEAT-095-sweep.md) |
| FEAT-096 | An extension host anyone can build for | Done | 12 | 0 | 0 | 0 | 0 | 12 | [FEAT-096-sweep.md](FEAT-096-sweep.md) |
| FEAT-097 | CodeRabbit reviews local changes | Done | 12 | 0 | 0 | 0 | 0 | 12 | [FEAT-097-sweep.md](FEAT-097-sweep.md) |
| FEAT-098 | CodeRabbit as a farm review gate | Done | 1 | 0 | 0 | 0 | 0 | 1 | [FEAT-098-sweep.md](FEAT-098-sweep.md) |
| FEAT-099 | CodeRabbit on GitHub pull requests | Done | 1 | 0 | 0 | 0 | 0 | 1 | [FEAT-099-sweep.md](FEAT-099-sweep.md) |
| FEAT-100 | A merger that shows the result first | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-100-sweep.md](FEAT-100-sweep.md) |
| FEAT-101 | Merge without conflicts | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-101-sweep.md](FEAT-101-sweep.md) |
| FEAT-102 | Resolve every conflict, every way | Done | 13 | 0 | 0 | 0 | 0 | 13 | [FEAT-102-sweep.md](FEAT-102-sweep.md) |
| FEAT-103 | A rebase that stops, in Merger | Done | 7 | 0 | 0 | 0 | 0 | 7 | [FEAT-103-sweep.md](FEAT-103-sweep.md) |
| FEAT-104 | A branch named where HEAD is | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-104-sweep.md](FEAT-104-sweep.md) |
| FEAT-105 | Review from the pull request screen | Done | 2 | 0 | 0 | 0 | 0 | 2 | [FEAT-105-sweep.md](FEAT-105-sweep.md) |
| FEAT-106 | A rebase you can see | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-106-sweep.md](FEAT-106-sweep.md) |
| FEAT-107 | Hooks you can see and skip | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-107-sweep.md](FEAT-107-sweep.md) |
| FEAT-108 | Farm shared journey | Done | 3 | 2 | 0 | 0 | 0 | 1 | [FEAT-108-sweep.md](FEAT-108-sweep.md) |
| FEAT-109 | Farm building board | Done | 4 | 3 | 0 | 0 | 0 | 1 | [FEAT-109-sweep.md](FEAT-109-sweep.md) |
| FEAT-110 | Farm task journey | Done | 4 | 3 | 0 | 0 | 0 | 1 | [FEAT-110-sweep.md](FEAT-110-sweep.md) |
| FEAT-111 | Farm setup and planning | Done | 4 | 3 | 0 | 0 | 0 | 1 | [FEAT-111-sweep.md](FEAT-111-sweep.md) |
| FEAT-112 | Farm crew and activity | Done | 3 | 2 | 0 | 0 | 0 | 1 | [FEAT-112-sweep.md](FEAT-112-sweep.md) |
| FEAT-113 | Farm wrap up and evidence | Done | 2 | 1 | 0 | 0 | 0 | 1 | [FEAT-113-sweep.md](FEAT-113-sweep.md) |
| FEAT-115 | Agents in Review and Merger | Done | 8 | 1 | 0 | 0 | 0 | 7 | [FEAT-115-sweep.md](FEAT-115-sweep.md) |
| FEAT-116 | Pull requests lists and reads; Review reviews | Done | 2 | 0 | 0 | 0 | 0 | 2 | [FEAT-116-sweep.md](FEAT-116-sweep.md) |
| FEAT-117 | Every merge happens in Merger | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-117-sweep.md](FEAT-117-sweep.md) |
| FEAT-118 | What an agent was sent and what it said, shown properly | Done | 3 | 0 | 0 | 0 | 0 | 3 | [FEAT-118-sweep.md](FEAT-118-sweep.md) |
| FEAT-119 | An agent's progress you can read | Done | 2 | 0 | 0 | 0 | 0 | 2 | [FEAT-119-sweep.md](FEAT-119-sweep.md) |
| FEAT-120 | The review room's file list can be hidden, and is compact or detailed | Done | 2 | 0 | 0 | 0 | 0 | 2 | [FEAT-120-sweep.md](FEAT-120-sweep.md) |
| FEAT-121 | What is running is said, and what would collide waits | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-121-sweep.md](FEAT-121-sweep.md) |
| FEAT-122 | One branch menu, that can be searched and says where you are | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-122-sweep.md](FEAT-122-sweep.md) |
| FEAT-123 | Merger checks out the branch it merged into, and offers to push it | Done | 12 | 0 | 0 | 0 | 0 | 12 | [FEAT-123-sweep.md](FEAT-123-sweep.md) |
| FEAT-124 | An agent that shows it is working, and asks at the top of its card | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-124-sweep.md](FEAT-124-sweep.md) |
| FEAT-125 | A choice on a conflict can be cancelled, and Pick lines starts empty | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-125-sweep.md](FEAT-125-sweep.md) |
| FEAT-126 | Merger says a branch is behind its remote, and pulls it | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-126-sweep.md](FEAT-126-sweep.md) |
| FEAT-127 | The review room says the pull request moved, and pulls the latest | Done | 8 | 0 | 0 | 0 | 0 | 8 | [FEAT-127-sweep.md](FEAT-127-sweep.md) |
| FEAT-128 | A conflict's columns scroll in place, together or each alone | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-128-sweep.md](FEAT-128-sweep.md) |
| FEAT-129 | Review shows a file before and after, side by side | Done | 11 | 0 | 0 | 0 | 0 | 11 | [FEAT-129-sweep.md](FEAT-129-sweep.md) |
| FEAT-130 | A terminal on the rail, with tmux in it | Done | 27 | 10 | 0 | 0 | 0 | 17 | [FEAT-130-sweep.md](FEAT-130-sweep.md) |
| FEAT-131 | Files: any revision, read and never written | Done | 23 | 12 | 0 | 0 | 0 | 11 | [FEAT-131-sweep.md](FEAT-131-sweep.md) |
| FEAT-132 | Investigate: the life of a file, a folder, a branch or a line | Done | 20 | 10 | 0 | 0 | 0 | 10 | [FEAT-132-sweep.md](FEAT-132-sweep.md) |
| FEAT-133 | Recipes: git steps in blocks, run with an agent | Done | 31 | 9 | 0 | 0 | 0 | 22 | [FEAT-133-sweep.md](FEAT-133-sweep.md) |
| FEAT-134 | Pin or hide what is on the rail | Done | 14 | 8 | 0 | 0 | 0 | 6 | [FEAT-134-sweep.md](FEAT-134-sweep.md) |
| FEAT-136 | Issues beside the code | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-136-sweep.md](FEAT-136-sweep.md) |
| FEAT-137 | Hooks that run in steps and are built from blocks | Done | 13 | 0 | 0 | 0 | 0 | 13 | [FEAT-137-sweep.md](FEAT-137-sweep.md) |
| FEAT-138 | Preview workflows and follow their runs | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-138-sweep.md](FEAT-138-sweep.md) |
| FEAT-139 | Draft and publish releases with their files | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-139-sweep.md](FEAT-139-sweep.md) |
| FEAT-140 | Home: what this repository needs from you | Done | 11 | 0 | 0 | 0 | 0 | 11 | [FEAT-140-sweep.md](FEAT-140-sweep.md) |
| FEAT-141 | Starting a farm, one step at a time | Done | 10 | 0 | 0 | 0 | 0 | 10 | [FEAT-141-sweep.md](FEAT-141-sweep.md) |
| FEAT-142 | A notification center that keeps what it said | Done | 11 | 0 | 0 | 0 | 0 | 11 | [FEAT-142-sweep.md](FEAT-142-sweep.md) |
| FEAT-143 | A slice per task | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-143-sweep.md](FEAT-143-sweep.md) |
| FEAT-144 | The landing lane: merging without a queue, and a log of every branch move | Done | 6 | 0 | 0 | 0 | 0 | 6 | [FEAT-144-sweep.md](FEAT-144-sweep.md) |
| FEAT-145 | History as tools for agents | Done | 4 | 0 | 0 | 0 | 0 | 4 | [FEAT-145-sweep.md](FEAT-145-sweep.md) |
| FEAT-146 | A conflict resolver chosen in Rules | Done | 3 | 0 | 0 | 0 | 0 | 3 | [FEAT-146-sweep.md](FEAT-146-sweep.md) |
| FEAT-147 | Own copies, and intake at the door | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-147-sweep.md](FEAT-147-sweep.md) |
| FEAT-151 | Search the text of files, by word or regular expression | Done | 9 | 0 | 0 | 0 | 0 | 9 | [FEAT-151-sweep.md](FEAT-151-sweep.md) |
| FEAT-152 | The Files tree widens by dragging, copies from a right-click, and a long selection holds | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-152-sweep.md](FEAT-152-sweep.md) |
| FEAT-153 | Blame on click can be turned off | Done | 5 | 0 | 0 | 0 | 0 | 5 | [FEAT-153-sweep.md](FEAT-153-sweep.md) |
| TASK-001 | Records baseline | Done | 8 | 0 | 0 | 0 | 0 | 8 | [TASK-001-sweep.md](TASK-001-sweep.md) |
| TASK-002 | Test and CI baseline | Done | 10 | 0 | 0 | 0 | 0 | 10 | [TASK-002-sweep.md](TASK-002-sweep.md) |
| TASK-003 | Make the Tauri layer generic over `Runtime` | Done | 6 | 0 | 0 | 0 | 0 | 6 | [TASK-003-sweep.md](TASK-003-sweep.md) |
| TASK-004 | Rename the project from GitLord to Spagitty | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-004-sweep.md](TASK-004-sweep.md) |
| TASK-005 | Branch coverage is below the Amendment 10 floor | Done | 8 | 0 | 0 | 0 | 1 | 7 | [TASK-005-sweep.md](TASK-005-sweep.md) |
| TASK-007 | Copy sweep: drop the hand-holding and the Mac notation | Done | 10 | 0 | 0 | 0 | 0 | 10 | [TASK-007-sweep.md](TASK-007-sweep.md) |
| TASK-008 | The last self-narrating footer | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-008-sweep.md](TASK-008-sweep.md) |
| TASK-009 | The interface stops naming its own work items | Done | 4 | 0 | 0 | 0 | 0 | 4 | [TASK-009-sweep.md](TASK-009-sweep.md) |
| TASK-010 | Gate 4 fails on unmaintained advisories | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-010-sweep.md](TASK-010-sweep.md) |
| TASK-011 | Secret scanning has never run | Done | 4 | 0 | 0 | 0 | 0 | 4 | [TASK-011-sweep.md](TASK-011-sweep.md) |
| TASK-012 | The working record has drifted from the tree | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-012-sweep.md](TASK-012-sweep.md) |
| TASK-013 | Backfill the missing plan and testing documents | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-013-sweep.md](TASK-013-sweep.md) |
| TASK-014 | The repository still points at a remote that no longer exists | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-014-sweep.md](TASK-014-sweep.md) |
| TASK-015 | The documents describe a Spagitty that no longer exists | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-015-sweep.md](TASK-015-sweep.md) |
| TASK-016 | One branch carried twenty-eight items | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-016-sweep.md](TASK-016-sweep.md) |
| TASK-017 | Seventy-one commits with no path back to `main` | Done | 7 | 0 | 0 | 0 | 0 | 7 | [TASK-017-sweep.md](TASK-017-sweep.md) |
| TASK-018 | Four gates that have never seen this code | Done | 8 | 0 | 0 | 0 | 0 | 8 | [TASK-018-sweep.md](TASK-018-sweep.md) |
| TASK-019 | The launch sequence lives where no test can reach it | Done | 7 | 0 | 0 | 0 | 0 | 7 | [TASK-019-sweep.md](TASK-019-sweep.md) |
| TASK-020 | The glass material, settled at the window | Done | 4 | 0 | 0 | 0 | 0 | 4 | [TASK-020-sweep.md](TASK-020-sweep.md) |
| TASK-021 | Centre the name in the title bar | Done | 4 | 0 | 0 | 0 | 0 | 4 | [TASK-021-sweep.md](TASK-021-sweep.md) |
| TASK-022 | The glass goes back on the GPU | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-022-sweep.md](TASK-022-sweep.md) |
| TASK-023 | Flatten UI and remove gradients | Done | 6 | 0 | 0 | 0 | 0 | 6 | [TASK-023-sweep.md](TASK-023-sweep.md) |
| TASK-024 | The glass reads as glass again | Done | 8 | 0 | 0 | 0 | 0 | 8 | [TASK-024-sweep.md](TASK-024-sweep.md) |
| TASK-025 | The release lane obeys Amendment 20 | Done | 10 | 0 | 0 | 0 | 0 | 10 | [TASK-025-sweep.md](TASK-025-sweep.md) |
| TASK-026 | Remove every remaining shadow | Done | 9 | 0 | 0 | 0 | 0 | 9 | [TASK-026-sweep.md](TASK-026-sweep.md) |
| TASK-027 | Migrate the JS toolchain to bun | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-027-sweep.md](TASK-027-sweep.md) |
| TASK-028 | Reconcile working record and docs | Done | 2 | 2 | 0 | 0 | 0 | 0 | [TASK-028-sweep.md](TASK-028-sweep.md) |
| TASK-029 | Candidate feature backlog items | Done | 2 | 2 | 0 | 0 | 0 | 0 | [TASK-029-sweep.md](TASK-029-sweep.md) |
| TASK-030 | The farm's refresh stops working on the main thread | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-030-sweep.md](TASK-030-sweep.md) |
| TASK-031 | A long session stays fast | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-031-sweep.md](TASK-031-sweep.md) |
| TASK-032 | Restore the release baseline and record the reliability plan | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-032-sweep.md](TASK-032-sweep.md) |
| TASK-033 | Restore coverage and platform tests | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-033-sweep.md](TASK-033-sweep.md) |
| TASK-036 | Use the current shared amendments book | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-036-sweep.md](TASK-036-sweep.md) |
| TASK-037 | Updates that actually arrive | Done | 9 | 0 | 0 | 0 | 0 | 9 | [TASK-037-sweep.md](TASK-037-sweep.md) |
| TASK-038 | Settings that stop lecturing | Done | 10 | 0 | 0 | 0 | 1 | 9 | [TASK-038-sweep.md](TASK-038-sweep.md) |
| TASK-039 | One colour system in every dialog | Done | 10 | 0 | 0 | 0 | 1 | 9 | [TASK-039-sweep.md](TASK-039-sweep.md) |
| TASK-040 | The macOS download says what it is | Done | 11 | 0 | 0 | 0 | 0 | 11 | [TASK-040-sweep.md](TASK-040-sweep.md) |
| TASK-041 | The frame stops competing with the work | Done | 13 | 0 | 0 | 0 | 0 | 13 | [TASK-041-sweep.md](TASK-041-sweep.md) |
| TASK-042 | One control scale, and a window that knows its platform | Done | 12 | 0 | 0 | 0 | 0 | 12 | [TASK-042-sweep.md](TASK-042-sweep.md) |
| TASK-044 | Settings stops explaining itself | Done | 6 | 0 | 0 | 0 | 3 | 3 | [TASK-044-sweep.md](TASK-044-sweep.md) |
| TASK-045 | A rail of five | Done | 6 | 0 | 0 | 0 | 3 | 3 | [TASK-045-sweep.md](TASK-045-sweep.md) |
| TASK-046 | Lists you can read | Done | 6 | 0 | 0 | 0 | 0 | 6 | [TASK-046-sweep.md](TASK-046-sweep.md) |
| TASK-047 | A commit bar | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-047-sweep.md](TASK-047-sweep.md) |
| TASK-048 | Commands off the main thread | Done | 6 | 0 | 0 | 0 | 0 | 6 | [TASK-048-sweep.md](TASK-048-sweep.md) |
| TASK-049 | The record catches up with what merged | Done | 2 | 2 | 0 | 0 | 0 | 0 | [TASK-049-sweep.md](TASK-049-sweep.md) |
| TASK-050 | The chatter goes | Done | 4 | 1 | 0 | 0 | 0 | 3 | [TASK-050-sweep.md](TASK-050-sweep.md) |
| TASK-051 | One theme | Done | 3 | 2 | 0 | 0 | 0 | 1 | [TASK-051-sweep.md](TASK-051-sweep.md) |
| TASK-053 | The review room after first use | Done | 7 | 0 | 0 | 0 | 0 | 7 | [TASK-053-sweep.md](TASK-053-sweep.md) |
| TASK-054 | Build macOS on a Mac | Done | 4 | 0 | 0 | 0 | 0 | 4 | [TASK-054-sweep.md](TASK-054-sweep.md) |
| TASK-055 | Extensions ship in the release | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-055-sweep.md](TASK-055-sweep.md) |
| TASK-056 | The rail as the author chose it | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-056-sweep.md](TASK-056-sweep.md) |
| TASK-057 | The Log screen in the house style | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-057-sweep.md](TASK-057-sweep.md) |
| TASK-058 | Restore the patch release after the bug-fix merges | Done | 1 | 0 | 0 | 0 | 0 | 1 | [TASK-058-sweep.md](TASK-058-sweep.md) |
| TASK-059 | The reading font and the focus ruler are off by default, and off the review bar | Done | 2 | 0 | 0 | 0 | 0 | 2 | [TASK-059-sweep.md](TASK-059-sweep.md) |
| TASK-060 | Agent permissions with standard names | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-060-sweep.md](TASK-060-sweep.md) |
| TASK-061 | Plain words for merging and stopping for approval | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-061-sweep.md](TASK-061-sweep.md) |
| TASK-062 | Finish the three open pull requests | Done | 2 | 0 | 0 | 0 | 0 | 2 | [TASK-062-sweep.md](TASK-062-sweep.md) |
| TASK-063 | Audit the pull requests merged since 2.0.0 | Done | 2 | 1 | 0 | 0 | 0 | 1 | [TASK-063-sweep.md](TASK-063-sweep.md) |
| TASK-064 | Refuse agent attribution in commits and descriptions, in CI | Done | 3 | 1 | 0 | 0 | 0 | 2 | [TASK-064-sweep.md](TASK-064-sweep.md) |
| TASK-066 | Style notes from the 2.0 audit | Done | 2 | 1 | 0 | 0 | 0 | 1 | [TASK-066-sweep.md](TASK-066-sweep.md) |
| TASK-067 | The attribution check does not read who wrote a commit | Done | 2 | 0 | 0 | 0 | 0 | 2 | [TASK-067-sweep.md](TASK-067-sweep.md) |
| TASK-068 | What CodeRabbit found on pull requests #63 and #64 | Done | 3 | 0 | 0 | 0 | 0 | 3 | [TASK-068-sweep.md](TASK-068-sweep.md) |
| TASK-069 | The two modals that choose a branch use the branch menu | Done | 6 | 0 | 0 | 0 | 0 | 6 | [TASK-069-sweep.md](TASK-069-sweep.md) |
| TASK-070 | The plan's file list scrolls, in a card the height of its neighbours | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-070-sweep.md](TASK-070-sweep.md) |
| TASK-071 | agy is removed | Done | 5 | 0 | 0 | 0 | 0 | 5 | [TASK-071-sweep.md](TASK-071-sweep.md) |
| TASK-072 | The declared minimum Rust is older than a dependency needs | Done | 2 | 0 | 0 | 0 | 0 | 2 | [TASK-072-sweep.md](TASK-072-sweep.md) |
| TASK-073 | The sweeps, refreshed and run from a portal | Done | 8 | 0 | 0 | 0 | 0 | 8 | [TASK-073-sweep.md](TASK-073-sweep.md) |
| TASK-074 | Pull requests no longer list gates 5 and 6 | Done | 2 | 0 | 0 | 0 | 0 | 2 | [TASK-074-sweep.md](TASK-074-sweep.md) |
| BUG-002 | The primary button has no fill | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-002-sweep.md](BUG-002-sweep.md) |
| BUG-003 | The lane canvas paints over the messages | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-003-sweep.md](BUG-003-sweep.md) |
| BUG-004 | The packaged Linux app opens a blank window | Fixed | 7 | 0 | 0 | 0 | 0 | 7 | [BUG-004-sweep.md](BUG-004-sweep.md) |
| BUG-005 | The geometry change left its test and comments behind | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-005-sweep.md](BUG-005-sweep.md) |
| BUG-006 | A long branch name overlaps the branch count | Fixed | 8 | 0 | 0 | 0 | 0 | 8 | [BUG-006-sweep.md](BUG-006-sweep.md) |
| BUG-007 | A replaced dialog resolves the wrong caller's value | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-007-sweep.md](BUG-007-sweep.md) |
| BUG-008 | ArrowUp into a fresh menu lands in the middle | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-008-sweep.md](BUG-008-sweep.md) |
| BUG-009 | The commit message column cannot be resized | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-009-sweep.md](BUG-009-sweep.md) |
| BUG-009b | The boundary people reach for carries a dead divider | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-009b-sweep.md](BUG-009b-sweep.md) |
| BUG-010 | A component and its store differ only by case | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-010-sweep.md](BUG-010-sweep.md) |
| BUG-011 | The first HTTPS request kills the process | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-011-sweep.md](BUG-011-sweep.md) |
| BUG-012 | A network request freezes the window | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-012-sweep.md](BUG-012-sweep.md) |
| BUG-013 | The window comes back with a tab strip and no repository | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-013-sweep.md](BUG-013-sweep.md) |
| BUG-014 | The Conflicts screen says resolving is not built | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-014-sweep.md](BUG-014-sweep.md) |
| BUG-015 | A backend preference list disarms the safe renderer | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-015-sweep.md](BUG-015-sweep.md) |
| BUG-016 | The graph's columns stop at the last commit row | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-016-sweep.md](BUG-016-sweep.md) |
| BUG-017 | The lens wipes the right column and the bottom of the window | Fixed | 6 | 0 | 0 | 0 | 1 | 5 | [BUG-017-sweep.md](BUG-017-sweep.md) |
| BUG-018 | A menu cannot be dismissed, and the next one is drawn over it | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-018-sweep.md](BUG-018-sweep.md) |
| BUG-019 | Closing the last tab leaves the repository open | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-019-sweep.md](BUG-019-sweep.md) |
| BUG-020 | The window freezes while the farm plans | Fixed | 7 | 0 | 0 | 0 | 0 | 7 | [BUG-020-sweep.md](BUG-020-sweep.md) |
| BUG-021 | A run says nothing until it ends | Fixed | 8 | 0 | 0 | 0 | 0 | 8 | [BUG-021-sweep.md](BUG-021-sweep.md) |
| BUG-022 | The farm subscribes after it asks, so the answer is lost | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-022-sweep.md](BUG-022-sweep.md) |
| BUG-023 | A record test reads another test's clone | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-023-sweep.md](BUG-023-sweep.md) |
| BUG-024 | Stop a run while its waiter owns it | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-024-sweep.md](BUG-024-sweep.md) |
| BUG-025 | One completion watcher per run | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-025-sweep.md](BUG-025-sweep.md) |
| BUG-026 | Verification keeps draining and can be stopped | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-026-sweep.md](BUG-026-sweep.md) |
| BUG-027 | Require current evidence before automatic merging | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-027-sweep.md](BUG-027-sweep.md) |
| BUG-028 | Development styles depend on request order | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-028-sweep.md](BUG-028-sweep.md) |
| BUG-029 | The packaged Linux window is ringed with blurred desktop | Fixed | 8 | 0 | 0 | 0 | 0 | 8 | [BUG-029-sweep.md](BUG-029-sweep.md) |
| BUG-030 | The toolbar offers what it cannot do, and hides what it can | Fixed | 11 | 0 | 0 | 0 | 0 | 11 | [BUG-030-sweep.md](BUG-030-sweep.md) |
| BUG-031 | The window opens in the wrong theme, then changes its mind | Fixed | 11 | 0 | 0 | 0 | 0 | 11 | [BUG-031-sweep.md](BUG-031-sweep.md) |
| BUG-032 | A path test that only passes on Unix | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-032-sweep.md](BUG-032-sweep.md) |
| BUG-033 | A CRLF file diffs as a rewrite | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-033-sweep.md](BUG-033-sweep.md) |
| BUG-034 | A transparent band round the window on Windows | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-034-sweep.md](BUG-034-sweep.md) |
| BUG-035 | Native scrollbars, and a smudged tab | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-035-sweep.md](BUG-035-sweep.md) |
| BUG-036 | The commit list, a size above the rest | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-036-sweep.md](BUG-036-sweep.md) |
| BUG-037 | Core tests read the machine's git config | Fixed | 2 | 2 | 0 | 0 | 0 | 0 | [BUG-037-sweep.md](BUG-037-sweep.md) |
| BUG-038 | Interactive rebase cannot start on Windows | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-038-sweep.md](BUG-038-sweep.md) |
| BUG-039 | A graph test that reads the clock | Fixed | 2 | 2 | 0 | 0 | 0 | 0 | [BUG-039-sweep.md](BUG-039-sweep.md) |
| BUG-040 | The shell's pane class leaks into five components | Fixed | 5 | 4 | 0 | 0 | 0 | 1 | [BUG-040-sweep.md](BUG-040-sweep.md) |
| BUG-041 | The pinned header cuts the detail card's shadow | Fixed | 2 | 1 | 0 | 0 | 0 | 1 | [BUG-041-sweep.md](BUG-041-sweep.md) |
| BUG-042 | A corporate CA is not trusted | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-042-sweep.md](BUG-042-sweep.md) |
| BUG-044 | Start review does nothing on GitLab | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-044-sweep.md](BUG-044-sweep.md) |
| BUG-045 | The name leaves the title bar | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-045-sweep.md](BUG-045-sweep.md) |
| BUG-046 | A console window opens for git | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-046-sweep.md](BUG-046-sweep.md) |
| BUG-047 | Whole file scrolls back to the top | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-047-sweep.md](BUG-047-sweep.md) |
| BUG-048 | A wider graph column draws no more lanes | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-048-sweep.md](BUG-048-sweep.md) |
| BUG-049 | Small monospace text ignores the reading font | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-049-sweep.md](BUG-049-sweep.md) |
| BUG-050 | The Mac check waits for a licence agreement | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-050-sweep.md](BUG-050-sweep.md) |
| BUG-051 | What the author saw on first run | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-051-sweep.md](BUG-051-sweep.md) |
| BUG-052 | Squeezed lanes, silent waits and striped notices | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-052-sweep.md](BUG-052-sweep.md) |
| BUG-053 | What the author found sweeping | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-053-sweep.md](BUG-053-sweep.md) |
| BUG-054 | Review's loader, its Conversation card, and scrollbars at rest | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-054-sweep.md](BUG-054-sweep.md) |
| BUG-055 | Branch where you point, stash in place, and a working copy that is noticed | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-055-sweep.md](BUG-055-sweep.md) |
| BUG-056 | Review and Pull requests align, and profiles open in Settings | Fixed | 4 | 4 | 0 | 0 | 0 | 0 | [BUG-056-sweep.md](BUG-056-sweep.md) |
| BUG-059 | On Windows the watcher never sees a ref move, and refreshes forever | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-059-sweep.md](BUG-059-sweep.md) |
| BUG-060 | Merger sits on "Working out the merge…" when it is first opened | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-060-sweep.md](BUG-060-sweep.md) |
| BUG-061 | Merger shows the previous pair's forecast under a new branch name | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-061-sweep.md](BUG-061-sweep.md) |
| BUG-062 | After a merge, Pull requests opens another pull request with nothing in it | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-062-sweep.md](BUG-062-sweep.md) |
| BUG-063 | A pull request that conflicts with its base is not marked, and Merge only fails | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-063-sweep.md](BUG-063-sweep.md) |
| BUG-064 | Review says "nobody asked you yet" about pull requests you have already reviewed | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-064-sweep.md](BUG-064-sweep.md) |
| BUG-065 | Command-line agents cannot start or finish reviews correctly | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-065-sweep.md](BUG-065-sweep.md) |
| BUG-066 | Extensions refuse to run on Spagitty 2 | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-066-sweep.md](BUG-066-sweep.md) |
| BUG-067 | Review says the token is invalid until Pull requests has been opened | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-067-sweep.md](BUG-067-sweep.md) |
| BUG-068 | Ordinary commits are marked as conflict fixes | Fixed | 2 | 0 | 0 | 0 | 0 | 2 | [BUG-068-sweep.md](BUG-068-sweep.md) |
| BUG-069 | Added lines sometimes look red | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-069-sweep.md](BUG-069-sweep.md) |
| BUG-070 | Local agents do not get their instructions, and some cannot start in Review or Merger | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-070-sweep.md](BUG-070-sweep.md) |
| BUG-071 | A Gemini agent from Google AI Studio gets "Bad request" | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-071-sweep.md](BUG-071-sweep.md) |
| BUG-072 | A remote agent with no key is told "Bad request", not that its key is missing | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-072-sweep.md](BUG-072-sweep.md) |
| BUG-073 | Two quick Assign clicks start two agents on one pull request | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-073-sweep.md](BUG-073-sweep.md) |
| BUG-074 | A remote agent's token limit is read only between steps | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-074-sweep.md](BUG-074-sweep.md) |
| BUG-075 | A model's reply that cannot be read is taken as an empty one | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-075-sweep.md](BUG-075-sweep.md) |
| BUG-076 | Merger says *Pushed* when the push failed, and no fetch or push works after it | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-076-sweep.md](BUG-076-sweep.md) |
| BUG-077 | A Farm task merged in Merger skips the Farm's own checks | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-077-sweep.md](BUG-077-sweep.md) |
| BUG-078 | The attribution check refuses a person whose name an agent shares | Fixed | 1 | 0 | 0 | 0 | 0 | 1 | [BUG-078-sweep.md](BUG-078-sweep.md) |
| BUG-079 | *All resolved* is drawn as a button | Fixed | 4 | 0 | 0 | 0 | 0 | 4 | [BUG-079-sweep.md](BUG-079-sweep.md) |
| BUG-080 | The review pill runs out of a narrow diff, over the conversation | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-080-sweep.md](BUG-080-sweep.md) |
| BUG-081 | Push of a new branch fails and does not say why | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-081-sweep.md](BUG-081-sweep.md) |
| BUG-082 | Saving the git identity clears the fields | Fixed | 5 | 0 | 0 | 0 | 0 | 5 | [BUG-082-sweep.md](BUG-082-sweep.md) |
| BUG-083 | Hiding the blame in Files breaks every line | Fixed | 6 | 0 | 0 | 0 | 0 | 6 | [BUG-083-sweep.md](BUG-083-sweep.md) |
| BUG-094 | A resumed agent skips files that changed since its first run | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-094-sweep.md](BUG-094-sweep.md) |
| BUG-095 | The Terminal's `+` says *no server running on session* | Fixed | 3 | 0 | 0 | 0 | 0 | 3 | [BUG-095-sweep.md](BUG-095-sweep.md) |

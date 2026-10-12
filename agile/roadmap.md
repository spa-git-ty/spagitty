<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Roadmap

The order the outstanding work is meant to be taken in, and why each piece sits
where it does. This is a reading of the index in [README.md](README.md) against
the gap analysis in [../docs/analysis/gitkraken-gap.md](../docs/analysis/gitkraken-gap.md);
the index stays the authority on what exists and what state it is in.

Every entry below now has an identifier. The four that came out of the gap
analysis rather than out of a request were named as they were started —
FEAT-048, FEAT-049, FEAT-050 and FEAT-051 — because the record test refuses a
cited identifier that resolves to nothing, so naming them earlier would have
broken the suite for no gain.

Sizes are the author's working estimate for someone who already knows the tree,
and they assume the existing test conventions are followed.

**Every entry under Now and Next is done**, and the Later section is being
worked through in the order it is written. They were worked one commit each,
between 2026-08-24 and 2026-08-25.

## Now

### 1. FEAT-047 — the branches table: resizable columns, and a divergence worth reading

**Done**, on `feature/FEAT-047-branch-table`. Finishing it clears the only open branch and
leaves the Branches screen ready for the delete and rename work that follows.

**Closed by:** a per-repository column store with its own storage prefix, and a
two-sided divergence bar scaled against the widest drift on screen.

### 2. FEAT-048 — discard changes

**Done**, on `feature/FEAT-048-discard-changes`.

The largest hole in daily use, and the cheapest to fill. Working copy can
stage, unstage and commit; nothing there can throw a change away, so the most
common mistake in a working day sends the user to the terminal. The gap
analysis names it as the one deliberate omission that should not have been one.

**Closed by:** discard file, discard hunk and discard all on the unstaged side,
each behind a confirmation whose wording says whether the file is reverted or
deleted. The staged side is deliberately untouched.

### 3. FEAT-013 — branch delete and rename

**Done**, on `feature/FEAT-013-branch-delete-rename`.

**Closed by:** row-level delete and rename, a bulk merged cleanup that shows
every name before it runs, and a confirmation that carries the actual command
to bring an unmerged branch back rather than pointing vaguely at the reflog.

### 4. FEAT-015 — rebase execution

**Done**, on `feature/FEAT-015-rebase-execution`.

**Closed by:** a worker that runs the rebase without holding the session lock,
progress read from git's own state counters rather than parsed from its output,
and a stopped state that hands off to Conflicts with continue, skip and abort
beside it.

### 5. FEAT-016 — conflict resolution writes

**Done**, on `feature/FEAT-016-conflict-writes`.

**Closed by:** three ways out of a file — a whole side, one marker region, or
text typed into the merged pane — each followed by an explicit `git add`, plus
Continue and an Abort that says what comes back for the operation it is aborting.
The conflicted stash apply recovery flow is still parked; it was not in scope.

## Next

### 6. FEAT-049 — remotes management

**Done**, on `feature/FEAT-049-remotes`.

**Closed by:** a Settings → Remotes section that adds, renames, removes and
retargets, each through `git remote` rather than a config edit. The `-u`
question was answered: push had no `--set-upstream`, so every new branch was
left unmapped and FEAT-047's divergence bar had nothing to read. Fixed.

### 7. FEAT-050 — reflog view

**Done**, on `feature/FEAT-050-reflog`.

**Closed by:** a Reflog screen at rail entry 1L, reading HEAD or any local
branch, with three ways out of an entry — branch here, check out here, reset
here — offered in that order because only the first cannot cost anything.

### 8. FEAT-051 — tags list

**Done**, on `feature/FEAT-051-tags`.

**Closed by:** a Tags screen at rail entry 1N with create, delete, message
rewriting and checkout, and annotated told apart from lightweight throughout —
the difference decides whether a message can exist at all.

### 8b. FEAT-056 — the detail panel can be put away

**Done**, on `feature/FEAT-019-commit-signing`.

**Closed by:** a hidden set beside the panel widths, and a toggle on the two
screens with a right-hand panel. A hidden panel keeps the width it was dragged
to, so bringing it back is a toggle rather than a resize.

### 8c. FEAT-055 — the window paints on the path that works

**Done**, on `feature/FEAT-019-commit-signing`.

**Closed by:** the Linux rendering path measured on a real window — twice per
run, because the accelerated one paints first and loses its buffer afterwards —
and written as a pure policy function with a test per row. The software path
stays the default because it is the one that keeps painting; the interface, now
knowing it rasterizes on the CPU, stopped asking for fifty-four blurred layers.

### 9. FEAT-018 — finish fetch and push

**Done**, on `feature/FEAT-018-finish-fetch-push`.

**Closed by:** pruning as a setting rather than something that always happened,
progress streamed from a worker, per-remote fetch behind a right-click, and a
Branches header that says how old its drift numbers are. The upstream on first
push had already been fixed in FEAT-049.

### 10. FEAT-033 — branch divergence on the chip

**Done**, on `feature/FEAT-033-divergence-on-the-chip`.

**Closed by:** `branches::divergences`, one read that both the Branches rows and
the graph's chips look up — the item made not having two of them a criterion —
and `↓3 ↑2` on the chip in the same order and colours as the bar. A level branch
draws nothing and still answers in its tooltip.

## Later

### 11. FEAT-017 — forge integration

**Done**, on `feature/FEAT-019-commit-signing`, continuing the stack.

**Closed by:** `spagitty-core::forge` — `ureq` with native TLS reached from one
file, one GraphQL request per refresh rather than 1 + 3N REST calls, a personal
access token in the OS keychain with the login read back from the host, and four
failures told apart rather than one.

Both things the roadmap said to decide first were decided. The HTTP client is
owned by the Rust core, so the webview keeps its promise of having no way to
make a request and never holds a token. The test that forbade an HTTP client
became a narrower one worth keeping: exactly one client, only in the core, only
reachable from `forge/http.rs`.

Read-only, and GitHub only. Issues, CI status on the graph, clone-from-host and
GitLab were all outside the item and remain unwritten.

### 12. FEAT-019 — commit signing

**Done**, on `feature/FEAT-019-commit-signing`.

**Closed by:** `spagitty-core::signing`, and an answer to the item's open
question — `commit.gpgsign` is the authority, so the toggle left Spagitty's
preferences file instead of being wired up, and Settings gained a Signing
section beside the identity. `--gpg-sign` on the commit, a signing failure
reported as one rather than as "commit failed", the two conditions that can be
known before the commit is attempted, and `signed` on every graph row.

Finished by the notice on the Working copy screen — said before the button, not
after a failure — and `S` on the graph row with the full sentence in the detail
panel. Nothing anywhere says *verified*, and a test holds that.

### 13. FEAT-034 — browse a stash entry file by file

**Done**, on `feature/FEAT-034-stash-file-browsing`.

**Closed by:** the Diff screen's own `FileList` and `DiffPane`, made to take
what they render so both screens use one renderer, and a Stash screen of four
columns — entries, files, diff, detail. No new backend read: a stash is a
commit, and `file_diff` on its id was already there.

### 14. TASK-003 — make the Tauri layer generic over `Runtime`

**Done**, on `task/TASK-003-runtime-generic`. The only entry that buys nothing a
user can see; what it buys is tests against the command layer without a window.

**Closed by:** `AppHandle<R>` throughout `src-tauri`, `spagitty-core`'s fixtures
published behind a `fixture` feature so the Tauri layer walks a real repository,
and seventeen tests over the graph worker's windowing, the watcher's debounce
and the session. It found no bugs.

### 15. The candidate feature backlog

Formally specified and indexed as Backlog items under TASK-029:

- **FEAT-062 — Worktrees management:** List, add, switch, and prune git worktrees via `git worktree` with workspace tabs integration.
- **FEAT-063 — File history and blame view:** Dedicated file evolution timeline with line-by-line interactive blame gutter.
- **FEAT-064 — Diff syntax highlighting:** Grammar-based syntax highlighting in diff view and merge editor across all themes.
- **FEAT-065 — Image and binary diffs:** 2-up, swipe slider, and onion skin visual diffs for images plus binary delta metadata.
- **FEAT-066 — Diff content search:** Regex and literal search inside commit patches using `git log -G` / `git log -S`.
- **FEAT-067 — Submodules management:** Submodule tree status list, recursive init/update, and branch alignment indicators.
- **FEAT-068 — External diff and merge tool launchers:** Tool discovery and launch triggers for external CLI/GUI diff/merge apps.
- **FEAT-069 — Multi-identity profiles:** Identity profile switching for author names, emails, and signing keys per repository scope.
- **FEAT-070 — Extended forge integration:** GitLab and Bitbucket API support, in-app PR creation, and issue tracking links.

## After 2.0.1 — the author's report of 2026-10-11

The author reported fifteen problems with 2.0.1 on 2026-10-11, and asked for
the open and the merged pull requests to be checked. They are recorded as
Backlog items and are meant to be taken in this order: what is already half
fixed in an open pull request first, then the small fixes whose cause is known,
then the words and which screen does what, then the agent card and the review
room's layout. The audit is read-only and runs beside the rest.

### 16. TASK-062 — finish the three open pull requests

Pull requests #59 and #60 hold the first halves of BUG-070 and BUG-071; #57
(BUG-063) needs reworking for FEAT-117 before it can be merged.

### 17. BUG-070 and BUG-071 — agents that work

Every local agent gets its instructions and can start in Review and Merger, and
a Gemini agent from Google AI Studio can be used.

### 18. BUG-066, BUG-067, BUG-068, BUG-069 and TASK-059 — small fixes

Extensions run on 2.x, Review asks with the right account, conflict fixes are
only conflict fixes, additions are green, and the reading font and ruler are
off by default.

### 19. TASK-060 then TASK-061 — permission names, then plain words

The permission names need the author's confirmation first. The wording sweep
follows so the agents' sentences are rewritten once.

### 20. FEAT-116 then FEAT-117 — one screen for each job

Review reviews and Pull requests only shows; then every merge moves to Merger,
which gains a pull request mode.

### 21. FEAT-118 and FEAT-119 — the agent card

What was sent and what came back, shown properly and hidden by default, and a
progress you can read.

### 22. FEAT-120 — the review room's file list

Hideable, and compact or detailed.

### Beside the rest: TASK-063 — audit the merged pull requests

Starting with #52 (FEAT-115). Its findings become items of their own.

## After 2.1.0 — the author's three requests of 2026-10-12

Asked for together and built together, in this order because each uses the one before: Merger's checkout and push are named by FEAT-121, and its cards choose a branch with FEAT-122.

### 23. FEAT-121 — what is running is said, and what would collide waits

A pull, a stash and a checkout are named while they run, the button that started one shows it, and what would collide is dimmed with the reason. No layer over the window: the item records why.

### 24. FEAT-122 — one branch menu, that can be searched

The toolbar, Merger and Rebase choose a branch from one list with a field to type in, and the current branch is marked at full strength. TASK-069 is the two modals left with a native list.

### 25. FEAT-123 — Merger checks out the branch it merged into, and offers to push it

## After 2.2.0

### 26. FEAT-124 — an agent that shows it is working, and asks at the top of its card

The agent's sentence moves while it works and is heard from. When it stops for the person, what it asks and the buttons that answer it are at the top of the card, not in the timeline, and what exactly is asked opens under them.

### 27. FEAT-125 — a choice on a conflict can be cancelled, and Pick lines starts empty

*Cancel* beside the choices and *Cancel all* for a file, in the resolver Merger and Conflicts share. Pick lines opens with nothing ticked and resolves nothing until a line is.

### 28. TASK-070 — the plan's file list scrolls

Merger's *What changes* is the height of the two cards beside it and scrolls inside.

### 29. FEAT-126 — Merger says a branch is behind its remote, and pulls it

A branch card says how many commits its upstream has that it lacks, with *Pull now*, which moves the branch forward whichever branch is checked out. Merger does not fetch on its own: the item says why, and what would change that.

### 30. FEAT-127 — the review room says the pull request moved, and pulls the latest

The host is asked while a pull request is open; when the author has pushed the room says so and *Pull latest* reads it again.

### 31. TASK-071 — agy is removed

A review given to agy stopped at its first step, on a permission its headless mode cannot ask for. The author chose to remove it rather than work round it. A saved Farm agent list that names it still opens.

## After 2.2.1 — the author's four notes of 2026-10-12

### 32. BUG-079 — *All resolved* is not a button

The resolver's pill says it and offers nothing, once there is no conflict left to go to.

### 33. FEAT-128 — a conflict's columns scroll in place

A conflict is never taller than the window: its three columns scroll, together by the same share of their lengths or each alone, and its header and choices stay put.

### 34. FEAT-129 — a file before and after, side by side

A third way to read a file in Review: the whole of it as it was beside the whole of it as it is, in one column that scrolls as one.

### 35. BUG-080 — the review pill stays in its column

The pill has the whole diff column to be laid out in and wraps when it must, and the diff keeps a least width while the panels beside it give way.

## Deliberately not on this list

Recorded so they do not resurface as gaps: dragging commits onto branches,
inline commit-message editing on the graph, manual lane layout and independent
graph zoom, all in `docs/reference/gitkraken-commit-graph.md`; and
workspace/cloud sync, which is out of character for an offline-first client.

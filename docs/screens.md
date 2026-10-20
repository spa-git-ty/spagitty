<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Screens

One section per screen. Each screen carries a short code — `1A`, `1B`, … — so it
can be named in one token in a commit message or a conversation. The codes and
their order come from the design handoff and are declared in `src/lib/nav.ts`.

**This document is updated by each screen's own work item**, in the same change
as the code. A section describing something that no longer exists is a
defect.

| Code | Screen | Route | Rail | State | Item |
| --- | --- | --- | --- | --- | --- |
| 2A | Home | `/home` | yes, top | Built | FEAT-140 |
| 1A | Graph | `/` | yes | Built | FEAT-001 |
| 1B | Diff | `/diff` | no | Built | FEAT-002 |
| 1C | Working copy | `/changes` | yes | Built | FEAT-003 |
| 1D | Conflicts | `/conflicts` | yes | Built | FEAT-008 |
| 1E | Interactive rebase | `/rebase` | yes | Built | FEAT-009 |
| 1F | Branches | `/branches` | yes | Built | FEAT-004 |
| 1G | Stash | `/stash` | yes | Built | FEAT-005 |
| 1H | Pull requests | `/requests` | yes | Built | FEAT-010, FEAT-017, FEAT-058 |
| 1I | Log search | `/search` | yes | Built | FEAT-007 |
| 1J | All repositories | `/repos` | yes | Built | FEAT-006 |
| 1K | Settings | `/settings` | yes | Built | FEAT-011 |
| 1L | Clone | modal | no | Built | FEAT-012 |
| 1M | Reflog | `/reflog` | yes | Built | FEAT-050 |
| 1N | Tags | `/tags` | yes | Built | FEAT-051 |
| 1O | Investigate | `/investigate` | yes | Built | FEAT-132 (was File history, FEAT-063) |
| 1P | Badges | `/badges` | yes | Built | FEAT-072 |
| 1Q | Farm | `/farm` | yes | Built | FEAT-073 |
| 1R | Review | `/review` | yes | Built | FEAT-087 |
| 1S | Merger | `/merge` | yes | Built | FEAT-100 |
| 1T | Terminal | `/terminal` | yes | Built | FEAT-130 |
| 1U | Files | `/files` | yes | Built | FEAT-131 |
| 1V | Recipes | `/recipes` | yes | Built | FEAT-133 |
| 1W | Issues | `/issues` | yes | Built | FEAT-136 |
| 1X | Hooks | `/hooks/runs`, `/hooks/edit` | yes | Built | FEAT-137 |
| 1Y | Pipelines | `/pipelines/preview`, `/pipelines/live` | yes | Built | FEAT-138 |
| 1Z | Releases | `/releases` | yes | Built | FEAT-139 |

**Every screen in the handoff is built**, and 1A–1L is the whole of it. 1M and
1N were not in the handoff at all: the Reflog and Tags came out of the GitKraken
gap analysis rather than the design, and they are numbered after the handoff's
run rather than inserted into it, so that a code still says where a screen came
from.

1P came from neither. Every other screen here answers a question about the state
of a repository; Badges answers what has been *done* in one, and by whom.

1Q is the first screen about work that has not happened yet — a goal, the tasks
it was cut into, and the agents working them. It takes the rail's top slot: it
is the product's own subject, and everything below it in the rail is where the
farm's output is read.

1R, Review, came from the author's own reviewing: pull requests read closely,
by a dyslexic reviewer, without the host's clutter. It sits beside 1H, which is
still where pull requests are browsed, created and merged.

What remains deferred is named on the screen that defers it, in place rather
than being absent — a conflicted stash apply, and the forges Spagitty has no
client for. The two that used to be listed here, the host Pull requests could
not reach and the accounts Settings had no client for, were closed by FEAT-017.

`src/lib/ui/ScreenStub.svelte` is no longer rendered by any route. It stays
because it is how the next unbuilt screen says what it will be rather than
pretending to be it: a half-built screen that looks real is harder to read than
an honest empty one.

## 2A — Home

**Item:** [FEAT-140](../agile/items/FEAT-140-home.md). **Route:** `/home`.

The top of the rail, above a divider and the Farm, and the screen a launch opens on: the last session's tab and selected commit come back, the screen is Home. The header names the repository, the branch against its upstream, when it was last fetched, and the remote, licence and version; `Fetch all` and `New farm`.

**Needs you** lists what waits on the person now, most urgent first — merge ready, review requested, task stuck, checks failed, changes requested, assigned to you — with All, Reviews, Failing and Farm chips. A row leaves when its thing is done. Each kind's colour, icon and button come from `src/lib/notifications/kinds.ts`, which the notification center reads too. Beside it, the Farm card and Working copy; below, Branches, Pull requests and Latest commits, then sixteen weeks of commits, Pipelines and Releases. The git half is one read (`home_overview`); the forge half rides the watcher's clock and Home adds no timer.

## 2B — Untrack

**Item:** [FEAT-155](../agile/items/FEAT-155-untrack.md). **Route:** `/untrack`.

A tool, after the Reflog. Two panes. On the left, a local branch — the one checked out, at first — and its tracked files read from git at that branch, with a field to find one by name; each file and folder has a box to tick. A folder ticked takes in everything in it. On the right, what will happen before anything does: the paths, the `.gitignore` line for each (on by default, since otherwise the next `git add .` tracks them again), the commit's message, suggested from the paths until it is typed over, and *Push after committing*. One button does it: `git rm --cached`, the `.gitignore` lines, a commit, and the push through the network worker, which creates the branch on the remote if it is not there yet.

On the branch checked out, the commit is made in place with the hooks, and is refused while anything else is staged, or while `.gitignore` has changes of its own it would sweep in. On another branch it is made in a worktree with nothing checked out, so the working copy does not move; hooks do not run there, and the screen says so.

## 1Z — Releases

**Item:** [FEAT-139](../agile/items/FEAT-139-releases.md). **Route:** `/releases`.

The list filters all releases, drafts and pre-releases. Show hidden reveals releases hidden on this machine; Hide sends nothing to the host. Each row offers a host link, copy, hide and confirmed deletion. GitHub download counts are shown; GitLab's absent counts remain absent.

The editor accepts the tag, branch or commit target, previous tag and title. Write and Preview share the safe Markdown renderer. Generate from commits uses the Hooks message parser; GitHub notes is a separately confirmed host request. GitLab drafts stay in application data until publication. The explicit Latest and Pre-release options are GitHub features; GitLab's Latest badge follows its latest-release endpoint.

The files column supports the native picker and drop, links, removal, per-file progress and retry. From pipeline reads finished runs on the resolved target, downloads a selected artifact and queues it for confirmed upload. GitLab first uploads a package, then separately confirms linking it to a published release.

Write to CHANGELOG.md requires a clean checked-out target branch and enabled hooks. Commit, push and publish each ask for confirmation. A moved target stops the operation; existing tags are never moved. Deleting the tag is unchecked by default and asks separately after release deletion.

## 1W — Issues

**Item:** [FEAT-136](../agile/items/FEAT-136-issues.md). **Route:** `/issues`.

Issues follows Recipes in the Repository group. Its header selects a configured hosted remote and accepts state, assignee, label, milestone, sort and text filters. The virtual list shows state, title, labels, author, update date and reply count. Saved pages paint before host revalidation; read, cache and write errors remain separate.

The middle column combines the description, comments and events in date order. The details column shows assignees, labels, milestone progress and reported development links. Markdown uses the application's existing safe renderer. Create, comment and close each ask for confirmation. Known read-only credentials hide writes. Replies sorting is available on GitHub; GitLab does not provide it.

Start branch offers an editable suggestion and uses the tracked default branch. Hand to agent reads every comment page and opens an unready Farm task draft. The person reviews it and picks the agent through the existing Farm flow; the button does not start an agent.

## 1X — Hooks

**Item:** [FEAT-137](../agile/items/FEAT-137-hooks.md). **Routes:** `/hooks` and `/hooks/runs` show Runs; `/hooks/edit` shows the builder.

Runs reads append-only records under the repository's common git directory. A step has a state, time and its own output. The screen tails only the selected log, through the existing repository watcher. A terminal commit writes the same records. Earlier runs, staged files and formatting's restage are shown beside the log. Undo restage restores only saved index entries and refuses a later staging decision.

The builder has the seven catalogue blocks, ordered inside pre-commit, commit-msg and pre-push. Each has its command or rule, timeout and failure choice. Path conditions use git's pathspec engine. Trying a message runs the generated commit-msg script. Dry run runs the generated stage without committing; formatting may change files and restage them when chosen.

Shared definitions use `.spagitty/hooks.toml`. Personal definitions live in application data. Installing is explicit, including after a clone. A manager retains control by default; Import reads its commands without changing its files. Installing imported blocks switches git to a managed directory, preserving the manager's originals.

The new screens are rows of the Repository group, after Recipes. The existing shell components keep their appearance.

## The chrome

Persistent across every screen, built with FEAT-001.

- **Title bar** — repository name, current branch, build identity, window
  buttons. The window is undecorated, so the title bar is also the drag handle
  and `src/lib/chrome/ResizeEdges.svelte` provides the resize edges.
  It carries no theme control — Settings → Appearance is the one place the
  theme is set — and no shortcut hint: it used to show `⌘K` for Log search,
  when the shortcut is `Ctrl+F` and the notation was macOS's on every platform.
  Key names are written in their `Ctrl` / `Alt` form throughout the interface.
  The one exception is the command palette, which picks its own notation from
  the platform at runtime (`src/lib/palette/commands.ts`) — macOS is a build
  target, and printing `Ctrl+` on a machine with no such key would be wrong in
  the other direction.
- **Toolbar** — repository and branch pickers, Undo/Redo, Clone, Fetch, Push,
  Branch, Stash, Rebase, and the primary Commit button. Actions that are not
  built yet say so on hover rather than failing silently when clicked.
- **Nav rail** — the only answer to "where am I": the active item and the route
  are the same fact. Counts are right-aligned; `·` means "not computed yet", and
  screens that do not exist report `·` rather than a number that would be wrong.

  The top slot is **Open repository…**, painted as the primary action (FEAT-030).
  It is the first thing a new user needs and it used to sit below a spacer at the
  bottom, which is the least discoverable place in the rail. The slot previously
  held a "filter commits / ⌘F" field that only duplicated the Log screen's own
  query bar and the `Ctrl+F` shortcut; it is gone. The foot keeps the "Tags N ·
  Submodules N" line, now alone.

  Rail order is the screens roughly as they are worked through — Farm, then
  Graph, Working copy, Conflicts (while there is something to resolve), Files, Merger,
  Branches, Pull requests, Review — then the tools, always shown since TASK-056:
  Rebase, Investigate, Log, Stash, Tags, Reflog, Badges (with the delight layer on) and All
  repositories; then Settings. Log follows Rebase because it is where you go to
  look something up rather than a step in that sequence. Stash, Tags and Reflog
  are rows of their own again, as the author asked; Branches still names all
  four in its segmented control.

  Since FEAT-134 a person can take a screen off the rail, in Settings → Rail,
  and reorder the rows of a group. What they hid is under a **More** button at
  the end of Tools, or in the palette only. The screen that is open is always
  on the rail while it is open, hidden or not.

## The window itself

The window is undecorated and **transparent**, so everything a person reads as
"the window" is drawn by the application: a 12px corner, a 0.2px outline, an
inset highlight along the top edge, and two shadows — a tight dark one holding
the card down and a wide soft one giving it height (FEAT-037). `body` carries a
margin for the shadow to fall into; without it a `box-shadow` on an element flush
against the window edge is clipped away entirely.

Maximizing drops all of it, because a floating card with a gap around it is a
window that does not fit its own screen. CSS cannot ask Tauri its state, so
`appWindow.watchMaximized()` publishes it as `data-window` on the root element.

**The chrome is glass, and thick glass bends light** (FEAT-057). Every bar,
rail, menu and dialog is a frosted pane; the panes that float — menus and
dialogs — also refract, so the application behind them is pushed outward toward
each rim and the colour splits a little where the bend is sharpest.

It is not done the way the web does it. `backdrop-filter: url(#filter)` is what
every published recreation of this effect uses, and **WebKitGTK renders nothing
for it**: the declaration parses, `CSS.supports` answers yes, and the pane comes
out identical to one with no filter. So the filter goes on the other side of the
glass — an ordinary `filter` on the application underneath, displacing it in a
ring the exact shape of each pane. What bends around a menu is the actual commit
list, actually displaced.

Two things follow from that, and they are visible in the markup. The filtered
element is `.lens`, a wrapper inside `.app`: `.app` carries the window's outline
and cast shadow, both drawn outside its border box, and a filter clips to that
box — filtering `.app` would cut the window's own edge off for as long as a menu
was open. And a pane cannot be inside what it bends, so a menu raised deep in a
screen is moved to a window-sized stage of its own. The arithmetic is in
`src/lib/ui/liquidGlassMaps.ts`, tested without a window; the measuring and the
registry are in `src/lib/ui/liquidGlass.ts`, which does nothing at all when
there is no `.lens` to filter — which is every component test.

**The region is a fraction of the box, never a measurement of it.** The filter
covers `0 0 1 1` in `objectBoundingBox` units, and the map sources carry no
subregion so each one defaults to that. It used to be the element's measured
width and height in `userSpaceOnUse` units — CSS pixels, which WebKitGTK
consumes as device pixels, so on any display whose ratio is not 1 the region
covered `1 / devicePixelRatio` of the window and the rest of it simply stopped
painting for as long as a menu was open (BUG-017). A fraction cannot be read in
the wrong unit, and with one set of numbers describing the geometry instead of two there
is nothing left for the ratio to be wrong about — the maps are authored in CSS
pixels and stretched onto the region, and nothing in either file refers to a
device pixel.

Worth knowing before diagnosing anything here: the compositor's monitor scale is
not the webview's ratio. Hyprland reports `scale: 1` on the display where
WebKitGTK reports 1.3636.

**Every panel resizes.** `PANELS` in `src/lib/panels.svelte.ts` is the registry —
each panel names its CSS variable, the edge it is anchored to, and its range —
and `Splitter` takes any key. The anchored edge is what decides the drag
direction, and the splitter measures **the panel beside it** rather than the
window: the window's edge is the right reference only for the rail, and wrong for
anything nested inside a screen where the rail's own width sits in between.

## 1A — Graph

**Built.** `src/routes/+page.svelte`, `src/lib/graph/`.

The centre of gravity, and the application's primary navigation surface rather
than a read-only report: almost every operation Spagitty can perform is reachable
from a right-click here.

A streamed, virtualised commit list with a lane canvas, a configurable column
table, and a detail panel. Rows arrive in batches from a worker thread and never
move once drawn. Clicking selects; double-clicking opens the diff.

**Operations** (FEAT-022, `src/lib/graph/actions.ts`). Right-click a commit for
create-branch/tag-here, reset (soft/mixed/hard, named by effect rather than by
flag), revert, cherry-pick, rebase-onto, detached checkout and copy-SHA.
Right-click a branch label for merge, rebase, fast-forward, rename, delete, pin,
hide and solo. Dragging one label onto another opens Merger's plan for the
pair (TASK-056), the dragged branch coming into the one it was dropped on, so
the merge is seen before it is written. Shift and Ctrl/Cmd build a
second selection — separate from the detail panel's — that cherry-picks a group
or rebases a range.

Every destructive operation confirms through `Dialog` and reports through
`Notice`, both mounted by the shell.

**Noise control** (`visibility.svelte.ts`). Hide, solo, smart branch visibility
and pin-to-left, all per repository, all resolved to a root set for a fresh walk
rather than to a filter over drawn rows. The header chip always names the
current scope and the gear lists what is hidden, soloed or pinned with a way
back — a filter you cannot see is a filter you forget is on. The author filter
is the exception: it **dims** rather than removes, because on the graph the
shape is the thing being looked at.

**The table** (`columns.svelte.ts`, `GraphHeader.svelte`). Branch/Tag, Graph,
Commit Message, Author, Date/Time, SHA. Right-click the header to toggle, drag
to reorder, drag a divider to resize; choice, order and widths are saved per
repository. Author avatars are initials on a lane colour computed locally —
never fetched, for the reasons in `src/lib/graph/avatar.ts`.

The lane column stops widening at twelve columns — the reasoning and the
measurements are in the doc comment on `LANE_COLUMNS_MAX` in
`src/lib/metrics.ts`.

**Past that cap the pitch gives, not the column** (FEAT-035). A thirteenth lane
is drawn closer to its neighbour rather than on top of it: the lanes share out
`LANE_SPAN` between them, down to a floor of `LANE_PITCH_MIN`, so the column's
width never depends on how busy the history is and the graph can never reach the
message column. The node radius follows the pitch down — the node is what set
the pitch in the first place, and a full-size portrait on a compressed column
would paint straight back over the room the compression made.

Two limits remain, both deliberate. Up to 32 lanes a node still fits inside its
own pitch; past that it is held at `MERGE_R` and begins to overlap its
neighbour, because a node that kept shrinking would stop being visible at the
depth where it is the only thing locating a commit. And at 48 lanes the pitch
reaches its floor, after which the deepest lanes do share a column — the old
behaviour, now reached four times deeper. `git/git` peaks at 382 lanes; some
histories defeat any width.

The lane pitch, node radius and elbow control points were
retuned in FEAT-022 against `docs/reference/gitkraken-commit-graph.md`, which
also records what this graph deliberately will **not** do: no dragging commits,
no inline message editing, no manual lane layout, no independent graph zoom.

### Nodes, lanes and the column (FEAT-023)

A node is the **author's portrait**, generated from their email — Boring
Avatars' marble construction rebuilt over the theme's own lane palette in
`src/lib/graph/portrait.ts`. Nothing is fetched: a picture per author would be a
request per author on the app's most performance-sensitive screen, would hand
the repository's committer list to whichever service was asked, and would make
an offline repository look different from an online one. The face is a function
of the address, so it is the same on every machine and every launch. One
description, two renderers — canvas for the node, CSS gradients for the Author
column — so a person has one face on the screen.

A **merge is a plain dot**, not a face: it is the moment two lines join rather
than one person's work, and putting the merge author's portrait on it would
claim they wrote the branch it swallowed.

The lane column is a **surface of its own** — `--graph-bg`, a mix of `--panel`
and `--bg` so every palette gets one — bounded by a hairline. It is painted
twice, from one declaration: by each row, so it scrolls with the rows, and by a
**bed** laid out once at the full height of the table underneath them.

The bed is what a repository shorter than the window needs. With the paint
living only on the rows, the column ended at the last commit and left a blank
slab down to the status strip, under a header that still showed three columns
(BUG-016). The bed iterates `columns.shown`, exactly as the row and the lane
canvas do, so all three move together when a column is resized, reordered or
hidden — the arrangement BUG-003 established and for the same reason. Their
order is set explicitly: bed, then rows, then canvas, then the scroll edges.

Geometry moved with the faces: node radius 11, lane pitch 26, stroke 2.5, elbow
control points 0.55/0.45. A five-lane column is 149px where it was 96px. FEAT-022
had taken it down from 150px because the graph crowded the messages; FEAT-023 put
129px back and FEAT-029 the rest, deliberately, because a face needs room — and
a face at 8.5px radius was still being read as a coloured dot.

**Hovering dims nothing.** Hovering a branch label used to grey out every commit
outside it and hovering a row drew a dashed ghost line to its nearest reference.
Both fire on a pointer that is only passing through, so the screen flickered as
the mouse crossed it. The author filter still dims, because it is a standing
question the user typed rather than a side effect of where the pointer is.

## 1B — Diff

**Built.** `src/routes/diff/+page.svelte`, `src/lib/diff/`.

One commit's changes, file by file and hunk by hunk. A full-window takeover
rather than a rail screen: opened from a commit, answering one question, with
`Esc` returning to the graph.

Loaded in two steps — the file list and totals in one call, a file's hunks as it
is selected — and hunks are cached by path for the open commit. Unified and
split views are the same data; `src/lib/diff/split.ts` pairs a run of removals
with the additions that follow it.

## 1C — Working copy

**Built.** `src/routes/changes/+page.svelte`, `src/lib/changes/`.

Stage what you mean to commit, write the message, commit. A 250px column holds
Staged above Unstaged — solid rows against dashed ones — and a path appears in
both when it is staged in part. Beside them: the message box, then the hunks of
the selected file with one action each, `stage hunk` or `unstage hunk`
depending on which side is open.

Its status walk is what made the rail's Working copy and Conflicts counts real.
The toolbar's Commit button counts `staged` rather than `working`: a working
copy with ten changed files and one staged must not offer to commit ten.

**Discarding is built, and only on the unstaged side** (FEAT-048): file, hunk,
or everything, each behind a confirmation whose wording says whether the file is
reverted or deleted. The staged side is deliberately untouched — unstage first,
which costs nothing, and then discard. Stage, unstage and commit still only move
changes forward, so the one action that can lose work is the one that asks.

## 1D — Conflicts

**Built.** `src/routes/conflicts/+page.svelte`, `src/lib/conflicts/`, and the
shared resolver in `src/lib/resolver/` (FEAT-102).
It resolves as well as reads, since FEAT-016.

What git stopped on by itself — a pull, a cherry-pick, a revert, a rebase from
the graph. A merge started in Merger (1S) is resolved there, with the same
three-column resolver. Ours (A) | the result | theirs (B), one card per marker
region. The sides come from the index: when git cannot merge two
versions of a file it keeps all three — stage 1 the base, stage 2 ours, stage 3
theirs — and leaves the working-tree file with markers in it. That is the whole
data model, and `crates/spagitty-core/src/conflicts.rs` is a reader for it.

Which stages exist *is* the kind of conflict. No stage 1 means both sides added
the path; a missing stage 2 or 3 means that side deleted it, and the pane says
so rather than rendering empty — an empty pane reads as "they emptied the file",
which is a different thing and one that loses work if acted on.

The operation in progress is read from the repository's own state, never
inferred from the presence of conflicts. Merge, rebase, cherry-pick and revert
all leave conflicts behind, and naming the wrong one sends someone to the wrong
command to get out.

**Reading still never writes**, and that is held by a test: `conflicts.rs`
takes the index's modification time either side of visiting every conflicted
file, and fails if a status walk rewrote it or left a lock behind. Resolving is
the only thing that writes, and it is always something the user asked for.

Every region is a choice on screen — take ours or theirs, both in either order,
pick single lines, or edit by hand — and each result line is badged with where
it came from. Nothing is written until *Mark resolved*, which writes what was
chosen and runs `git add` in one go, so the index says what the screen says; a
file whose markers are already gone from disk can be marked resolved as it is. Two ways out of the operation, both in the header:
Continue, live only once nothing is conflicted, and Abort, whose confirmation
names what comes back for the operation being abandoned rather than pointing
vaguely at the reflog.

## 1E — Interactive rebase

**Built.** `src/routes/rebase/+page.svelte`, `src/lib/rebase/`.
It runs the rebase as well as planning it, since FEAT-015.

Plan a history rewrite and see the result before anything runs. Interactive
rebase is feared because the todo list is edited blind — you choose squash and
reword against a list of short ids and find out what you did afterwards — so
this screen is the preview, which is the half that carries the value and none
of the risk.

The todo list is **generated, not parsed**. Running `git rebase -i` to read the
file it opens would start a rebase, which is the thing this screen exists to
avoid; the list is `upstream..HEAD` walked oldest first with merges excluded,
which is what git itself lists, and there is a test comparing it against
`git rev-list --reverse --no-merges`.

The plan is the complete list and its order *is* the reordering. The preview is
a fold of it, recomputed after every edit, so the plan and the picture of the
plan cannot disagree. A squash folds upward, which is the direction git folds;
a plan whose first row is a squash has nothing above it and is refused with
that reason.

Rows move by drag **and** from the keyboard (`Alt+↑` / `Alt+↓`). Drag alone is
untestable headlessly and unusable for some people, and the store owns the
ordering so the component only reports intent.

"May conflict" is a heuristic and the screen uses that word: two commits in the
plan touching one path mark the later one. Knowing for certain means performing
the merges, which is execution. Claiming a clean result Spagitty cannot prove
would be the worse lie.

Nothing runs. `shell::rebase_interactive` is still `unimplemented!()`, there is
no command that could reach it, and Apply renders disabled saying so. A test
asserts the repository is untouched after any amount of editing — no rebase in
progress, HEAD where it was, working copy clean.

## 1F — Branches

**Built.** `src/routes/branches/+page.svelte`, `src/lib/branches/`.
Delete and rename landed with FEAT-013, resizable columns and the divergence bar
with FEAT-047.

Every branch, how far it has drifted, and what is safe to forget: branch,
ahead/behind, last change, actions. Merged branches render dashed — nothing on
them is only there — though the current branch never does, since saying
"merged" about the branch you are on reads as "safe to delete".

Ahead and behind are counted against the remote-tracking ref on disk, so they
are as old as the last fetch. The header says how old (FEAT-018), and nothing on
this screen reaches a network to make them fresher — fetching is the toolbar's
job, and the numbers change when it finishes.

Checking out goes through `git switch`, which only ever changes branch — unlike
`git checkout`, which guesses between a branch, a revision and a path. A
checkout that would overwrite uncommitted work is refused by git, with git's own
message. Branch names are validated by git for the same reason: a second
implementation of `check-ref-format` could only disagree with it.

The branches command re-opens the repository rather than reusing the session
handle, because `gix` reads config once at open time and a branch's upstream
lives in config.

## 1G — Stash

**Built.** `src/routes/stash/+page.svelte`, `src/lib/stash/`.

Stash entries drawn hanging off the commit each was made on, then the files in
the selected entry, then the selected file's diff, then a detail panel for
everything about the entry that is not a file.

The middle two columns are the **Diff screen's own components** (FEAT-034), given
their files and their file rather than reading a store — a stash is a commit, so
the two lists are the same list and there is no second diff renderer to keep in
step. `↑` and `↓` walk the files; `j` and `k` jump between hunks, as on 1B. The
unified/split choice is shared with 1B on purpose: it is a preference about
reading diffs, not about a screen.

Pop, apply and drop are wired (FEAT-014). Each goes through
`stash.restore(action)`, which hands the confirmation and the write to
`graph/actions.ts` and then re-reads the list — the confirmation is written once
there, so this screen and the graph's own stash menu cannot describe the same
operation two different ways. `actions.stash` answers whether anything changed,
so a cancelled dialog does not cost a re-read. Pop and drop release the
selection before re-reading; apply keeps the entry open.

A **conflicted apply** is not yet handled as its own state: `git stash pop` onto
a conflict leaves the entry in place and the working copy conflicted, and today
that surfaces as git's own message in a notice. Honest, but not the designed
recovery FEAT-014's notes asked for — it needs a conflict write path and belongs
with FEAT-016.

There is no stash-diff code, and there does not need to be: a stash *is* a
commit whose first parent is the commit the work was made on, so the screen asks
`commit_diff` about the entry's id like any other commit and `file_diff` for one
of its files, and `refs/stash`'s reflog is the list — `stash@{n}` is literally
the nth entry.

The open file survives a re-read of the same entry — after an apply, or after
the watcher reports a change — rather than snapping back to the first file every
time the screen refreshes. It is dropped when the *entry* changes, along with
the hunks cached for it: those belong to one entry, and the same path in another
is a different file.

The lane is drawn with the graph's metrics but not its canvas. The canvas exists
to keep scrolling flat across a hundred thousand rows; a stash list is a dozen,
and a handful of SVG paths is the smaller thing that reads the same.

Stashing is the only write. `git stash push` succeeds quietly with nothing to
save, which from a button reads as a stash that happened and then vanished, so
the core refuses that case with a reason instead.

### Reading at depth (FEAT-052)

The lane pitch compresses past twelve columns and stops at **14px** — half the
design pitch. It used to stop at 6, which gave forty-eight lanes a distinct x
and gave none of them a visible gap: on `git/git`, at a mean lane depth of 187,
the column was a picket fence. Twenty-one lanes that can be told apart beat
forty-eight that cannot, and a history deep enough to need a twenty-second was
never going to have it read.

A consequence worth knowing: the node radius shrinks with the pitch, and at the
old floor nodes overlapped their neighbours past 32 lanes. At 14 they no longer
can, at any depth.

Ref chips start at the column's left edge rather than tucking against the graph,
so every row's first chip is at the same x.

The table shows a gradient at whichever edge has content hidden under it — the
columns can be dragged wider than the window, and an overlay scrollbar that only
appears once you are scrolling is no answer to "is there anything over there".

## 1H — Pull requests

**Built.** `src/routes/requests/+page.svelte`, `src/lib/requests/`, and
`crates/spagitty-core/src/forge/` behind it (FEAT-017).

What is waiting on you above what is waiting on everyone else: solid rows over
dashed ones, with the detail panel beside them — the same two-group device All
repositories uses.

**This is the only screen whose data comes off the network.** Everything else
in Spagitty reads the disk, and the promise on the All repositories screen —
repositories are read from disk and none is uploaded — still holds: what leaves
the machine is a request to a host the user connected themselves, carrying a
token they issued, asking for pull requests they can already see in a browser.
No repository contents, no paths, no commit messages, no telemetry.

The old promise was that no HTTP client was linked in either language, with a
test to keep it that way. That could not survive reading pull requests, so the
test became a narrower one that is still worth having: **exactly one** HTTP
client, `ureq`, declared **only** in `spagitty-core`, and reachable from exactly
one file — `forge/http.rs`. The webview still links none, still makes no
request, and never holds a token. `requests.test.ts` asserts all of it.

One request per refresh, through GraphQL. REST would need a list call plus three
per pull request for the line counts, the review decision and the checks —
ninety-one requests for thirty open ones, against a budget shared with
everything else the token does.

**The files, commits, and review workspace** (FEAT-058, FEAT-059). Opening a
pull request transitions to a dedicated full-window PR workspace view. The top
header carries the PR title (with compact sizing and smooth, slow hover auto-scroll
when long), author, relative update time, status chip, checks rollup, and commit
count, alongside a return button.

The left pane features a leading **CHANGELOG** entry that renders the PR's formatted
Markdown description and changelog in the main view, followed by collapsible
accordion sections for **All Changed Files** and **List Of Commits**. Commits display
fixed-width summaries that smoothly scroll (marquee on hover) to reveal long messages
without truncation, and the entire commit row is clickable to expand and list
per-commit changed files.

The center diff pane supports both unified and split diffs. Each diff line offers
an inline review comment trigger on hover. Review comments and local drafts render
directly beneath their associated diff lines. Draft comments automatically persist
in local browser storage (`localStorage`) so no review work is lost on network
hiccups or application restarts.

**Reviewer vs Developer modes.** Reviewers can compose local draft inline comments,
inspect all threads, and publish reviews (Approve, Request Changes, Comment) with
draft comments batched together in a single submission. Authors are prevented from
approving their own PRs. Developers can review feedback, reply directly to inline
comment threads, and mark change requests as resolved.

The PR list screen renders a smooth skeleton shimmer while credentials decrypt and
forges load, with the previous side panel removed for an uncluttered workspace.
The list and comments are immediately refreshed after a review is submitted.

Merging is still not built, and the button still says so.

Failures are four different sentences, because they are four different
decisions for the reader: could not reach the host, the host is rate limiting
and when it will stop, the token was refused, and no account is connected for
this host. "Could not load" is useless to somebody deciding whether to wait or
to go and fix something.

Signing in is a personal access token rather than OAuth. It is issued, scoped
and revoked by the person without touching anything else they own, and it needs
no redirect listener and no client secret shipped inside a GPL binary anybody
can read. The login is read back from the host rather than typed. The token goes
to the OS keychain and never to a configuration file; `accounts.json` holds a
host and a login and nothing else.

The empty state is the screen rather than a placeholder. "No account is
connected", with a way to Settings → Accounts, tells the user the screen works
and the account does not — which is the difference between this and the
`ScreenStub` it replaces.

`PullRequest` in `src/lib/types.ts` is the contract the screen was designed
against before any host could be reached, and FEAT-017 filled it in rather than
redesigning the screen around what one host's API happens to return — which is
what it was written for. The vocabulary stays host-agnostic, and a test asserts
no host's name appears anywhere in the screen: the kind of thing that rots the
moment somebody adds "Open on <host>" without thinking. GitHub is the only host
implemented, and it is implemented behind that contract.

## 1I — Log search

**Built.** `src/routes/search/+page.svelte`, `src/lib/search/`.
Reached from the rail and by `Ctrl+F` from any screen, which lands here with the
first field focused — the focus travels in the URL (`?focus=1`) rather than
through a store, so the shortcut and a bookmark behave identically.

Find commits by author, path, message or date. The filters compose as AND and
each is a chip saying exactly what is applied; the chips are derived from the
fields rather than stored beside them, so the two cannot disagree.

A search is the graph's revision walk with a predicate and without lanes.
Lanes are absent on purpose: drawing them over a filtered subset would draw
edges between commits that are not parent and child. The path filter uses git's
own simplification rule — a commit TREESAME to *any* parent is skipped — which
is what stops a merge being listed for a change it only carried across.

Results stream as the walk finds them. Each query carries a token and starting
one cancels the one before, so rows from an older query are dropped rather than
rendered; that is what makes it safe to search on a keystroke.

`↵` opens the commit in the side column — message, people, files — and `Alt+Enter`
opens its hunks on the Diff screen, which is a different question.

**Blame goes through the `git` binary**, and it is the one read in the
application that does. `gix::blame` 0.16, the newest published version, panics
on an ordinary history shape: a file blamed at a merge commit whose history
contains an intervening commit that left the file alone. Every diff algorithm
and both rename settings do it. The exception is recorded on `shell::blame`
with its end condition — blame moves back in-process when that is fixed
upstream. A binary file, a missing path and a directory each say which rather
than rendering an empty list, because an empty list reads as a file nobody has
ever touched.

## 1J — All repositories

**Built.** `src/routes/repos/+page.svelte`, `src/lib/repos/`.
Reached from the toolbar's repository picker.

Every repository you work in and which ones need attention: "Needs you" above
"Nothing in progress", the second rendered dashed. A card carries the branch,
the path, what the repository was last doing, and a chip for each thing going on
— conflicts first, since those are what stop work.

Spagitty never goes looking for repositories. Opening one is the only way it
joins the list, which lives in Spagitty's own config directory as a plain JSON
file of paths.

Each card is read where the repository sits, without opening it as the current
one, and without writing to it — there is a test that compares the index's
modification time either side of the read. A path that has gone comes back as a
card that says so rather than being dropped: a repository that moved is
something to see, not something to forget quietly. Forgetting removes the row
and never the directory.

## 1K — Settings

**Built.** `src/routes/settings/+page.svelte`, `src/lib/settings/`.

Sections behind a chip index — You, Remotes, External Tools, Behaviour,
Personality, God mode, Appearance, Rail, License — because these are read rarely and
changed rarely, and one route that says which part of itself is showing is
easier to link to than eight rail entries. The section is in the URL fragment,
so `/settings#accounts` lands where the Pull requests screen points.

**You holds everything about who you are**: the identity, the signing key, the
saved profiles and the connected hosting accounts. Accounts was its own chip
until it turned out to be a chip with no branch behind it — it fell through the
screen's closing `{:else}` and drew the License section, while `AccountsSection`
was being rendered under You the whole time. The chip is gone, `#accounts` now
resolves to You so the two links the Pull requests screen carries still arrive
at the accounts, and the screen has no catch-all left to hide the next one:
every section is an explicit branch, and `sections.test.ts` reads the route and
fails if a chip is ever added without one.

The last section was called **Advanced** until TASK-007. It has only ever held
the version, the build, the project's licence and its dependencies' licences, so
the name described nothing it contained. `#advanced` is still accepted as a
fragment and selects the renamed section, because a link written before the
rename doing nothing at all is worse than one that is merely out of date.

**Rail** (FEAT-134) chooses what is on the nav rail. Every screen has a row, in
its group, and is pinned or hidden; hidden screens go under the rail's More
button or to the palette only, and the rows of a group can be reordered. Farm
and Settings read *Always*, and Conflicts and Badges show the rule they keep.
The section's preview is `NavRail` itself, drawn from the choice. It is stored
as `rail: { hidden, order, where }` in Spagitty's settings, never in the
repository, and the default is everything pinned (TASK-056).

**Nothing here needs an open repository.** With none, the identity falls back to
the global scope alone and every other section is unaffected. **God mode** is
the one exception and says so: previews and sounds work without a repository,
and everything that writes a badge record is disabled because there is nowhere
to write it.

**God mode** (FEAT-072) drives the delight layer by hand. Every other badge in
Spagitty takes real work to see, which is the point of them and also the
problem: nobody can check that Git Lord looks right without earning Git Lord,
and nobody is going to resolve ten conflicts to find out whether a sound is too
loud. Four groups, in order of what they cost — preview a card (writes
nothing), fire an event the application really produces (through the real
rules), grant or revoke straight into the record (the only writes in the
application that bypass the engine), and whole-record operations. The
grant/revoke writes live in `delight` rather than in the section that draws
them, so they can be read against the code that awards the badges people
earned.

**Appearance is the only place the theme is set.** There is one theme,
Pomodoro, built from the brand (FEAT-086), with a light variant, Giorno, and a
dark one, Notte; Appearance chooses between them, follows the system, or — under
Omarchy — follows the desktop's own palette. The eight published families that
sat beside it were removed (TASK-051): none was drawn for the spatial shell, and
together they read as nine applications. The title bar's toggle is gone; one
preference with two controls is two things to keep in step.

The palettes are **data**, in `src/lib/themes.ts`, applied to `<html>` as custom
properties by `src/lib/theme.svelte.ts`. As CSS they would be the same sixteen
tokens written twice with nothing able to check them; as data they are tested, and what is tested is the thing that matters about a colour —
whether it can be read. `src/lib/themes.test.ts` computes WCAG contrast for
both, compositing the translucent tokens over what shows through them, and
holds ordinary text to 4.5:1 and secondary text, the accent and every lane
colour to 3:1.

`src/app.css` carries the default family's two palettes. They are the boot
values — what paints before any JavaScript runs — and nothing else; editing a
colour there changes the first frame and not the theme.

**The identity is read with `gix` and written with `git`.** That is the
`shell.rs` rule applied without an exception: `.git/config` and `~/.gitconfig`
are state the whole ecosystem reads, so writing them goes through `git config`;
reading them does not. `crates/spagitty-core/src/identity.rs` is both halves.

**The scope is a parameter, never inferred.** Writing to the wrong one is the
quiet mistake this screen is shaped around — a repository-local identity that
silently became global is found months later on somebody else's commits. So both
values report which file they came from, the fields say which one they are
editing, and changing scope refills them rather than carrying a typed value
across. A value coming from the system configuration or the environment is named
as such rather than as a scope Spagitty writes, because editing the global field
would not change it.

**Clearing unsets the key.** `git config --unset`, not an empty string. An empty
`user.email` is a *configured* empty email, which git will happily commit with;
an unset one falls back to the next scope, which is what "clear" means.

**A toggle that does nothing yet says so.** The three behaviour toggles persist
in Spagitty's own config directory beside the repository list, with the same
lenient-parse-or-default treatment for the same reason. A toggle that is not
honoured yet names the item that will honour it — signing is still waiting on
FEAT-019 — because narrowing the claim to the truth is better than a switch that
silently does nothing.

**"Show the git command behind each action" is honoured** (FEAT-020). It adds a
Commands button to the toolbar and a palette command; both open a drawer listing
what Spagitty actually ran, newest first, with the exit code and git's own stderr
under anything that failed. The lines come from `record.rs`, written by the
module that spawns the process, so the panel shows the flags the shell layer
added rather than what a screen believed it asked for. Reads are absent and the
drawer says why: history, refs, diffs and status are answered in-process and
have no command line at all.

The toggles are read by the shell on start, not only by this screen. Everything
that consults them — the confirmation before a history rewrite, the command
log — is reachable without ever opening Settings, and before that read landed
they answered from the defaults instead of from what the user chose.

**About carries the GPL-3 obligations**, and they were never deferred: the
version, the license, the commit stamped in at build time, and the trademark
notice were in the stub's footer from the first commit. They moved into this
section rather than disappearing while it was rebuilt.

The dependency license list is **generated at build time**, not typed — a
hand-written list is wrong by the next update. `src-tauri/licenses.rs` reads
`cargo metadata` for the Rust half and the installed frontend tree for the JS
half, walking the production dependencies of the root `package.json` through
`node_modules` (what `bun.lock` pins), and lists only what is *linked*: build
and development dependencies are not distributed, so describing them as part of
the binary would be wrong. `cargo-about` was the plan and was dropped —
requiring a build tool on every machine and every CI runner is a cost this
avoids, since cargo already reads the lockfile.

A list that cannot be generated **degrades rather than failing the build**. A
checkout with no `node_modules`, or an environment where `cargo metadata` cannot
run offline, produces a shorter list and a note saying what is missing and why.
The degradation is itself tested, because an untested fallback is a fallback that
does not work. A package declaring no license is listed as "not declared" rather
than omitted: an incomplete list that looks complete is the worse failure.

## 1L — Clone

**Built.** `src/lib/clone/`, mounted by `src/routes/+layout.svelte`.
Reached from the toolbar and from All repositories.

Bring a repository in: an address, a folder, and the exact path it will land at
shown before anything runs.

**A modal owned by the layout, not by a screen.** A clone survives navigation —
it takes minutes on a large repository, and pressing something in the nav rail
while it runs must not cancel it. A modal owned by a screen would go with the
screen.

**The clone goes through the `git` binary**, which is the point of the item: it
is the first operation that needs credentials, and credential helpers are
external programs resolved through config — the place OS keychain integration
already lives. `GIT_TERMINAL_PROMPT=0` still holds, so a repository whose
credentials no helper can supply fails with git's own message instead of hanging
on a prompt there is no terminal for. **Spagitty never asks for a password
itself.**

**Everything that can be refused is refused before the process starts.** An
unusable address, a folder that is not there, a destination that already has
something in it: each is computed by `clone::plan` as the user types, and each
is knowable without the network. Telling somebody after a round trip what they
could have been told while typing is the failure this avoids. An existing
*empty* destination is allowed, because `git clone` allows it — matching git's
rule rather than inventing a stricter one is what makes a Spagitty clone the same
as a command-line clone.

**Progress is git's, parsed rather than invented.** `git clone --progress`
writes each phase to stderr terminated by a carriage return rather than a
newline, and `clone::progress` reads one line at a time. A line it does not
recognise is still shown, so a change to a format git does not promise degrades
to "no percentage" rather than "no progress". The same reading is where a
failure's message comes from: stderr is both channels, and it is read here.

**Cancelling removes only what the clone created.** Whether the destination
existed is decided before the process starts and remembered; a directory the
user already had is left exactly as it was found, partial contents and all,
because that is not Spagitty's to delete. The removal happens after the child is
reaped, never after the kill signal, or the two race and files reappear behind
it.

**A failed clone leaves no entry in the repository list**, and that falls out of
the existing design rather than needing a rule: the list is written by opening a
repository, and the clone offers to open only what succeeded.

## 1O — Investigate

**Built.** `src/routes/investigate/+page.svelte`, `src/lib/investigate/`,
`crates/spagitty-core/src/investigate.rs` (FEAT-132). It was File history
(FEAT-063), off the rail and about one file; `/history` now comes here with
the path it was given.

The life of a file, a folder, a branch or a line: when it was born and by
whom, how it grew, when it was renamed, moved or rewritten, and who owns it
today. The target is in the address — `?kind=line&path=…&line=…&rev=…` — so
Files' *Investigate line* and a bookmark arrive the same way.

**A life is a list of entries, newest first, read a page at a time.** The
first page is on screen before the second is asked for. At 5,000 entries the
reading stops and *Load more* reads on. A life is kept by its target and by
where HEAD is, so going back to one costs nothing and a new commit reads it
again.

**Where the entries come from.** A file's are `git log --follow --raw
--numstat`; a folder's are the same log on a pathspec; a line's are `git log
-L`; a branch's are the commits since it left the default branch, with where
it landed and the rebases and resets its reflog records. A file's size after
each commit is counted from the object, not summed.

**A rewrite** is a commit that removed at least half the file's lines as they
were before it: `REWRITE_SHARE`, a named constant. A new name in the same
folder is *renamed*, and the same file in another folder is *moved*.

**A branch's fork is not its merge base.** A branch that has had the default
branch merged into it, or has landed, shares a late commit with it. The fork
is the commit the branch's oldest own commit was made on, where "own" is on
its first-parent line and not in the other branch.

**The life card** is a line of facts, the size over time with a marker on
each born, renamed, moved and rewrite, and a row of ticks for each author,
agents in `--agent`. `chart.ts` works out where everything goes, as
arithmetic, and the component draws it.

**The selected event** says what share of the file it changed, how many of
the lines it wrote today's blame still gives it, and who owns the file today
— one `blame::compact` for the whole life, not one for each event — and
shows its largest hunk. *Open file at <sha>* reads it in Files as that commit
left it, *Compare with today* shows that version against the file now, in
place, and *Show in Graph* selects the commit.

Like Files, it is given a revision and answers from git's objects.

## 1P — Badges

**Built.** `src/routes/badges/+page.svelte`, `src/lib/delight/` (FEAT-072).

What has been earned in this repository, and by whom. It is the one screen that
is not about the state of a repository, which is also why it survives having
none open with a sentence rather than a blank — "none of this has happened yet"
is a real answer.

**Per repository, and per actor.** A badge earned in one codebase says nothing
about another, and an aggregate across all of them would flatter whoever has the
most repositories. A human is keyed on their git email, so switching an identity
profile (FEAT-069) switches record; an agent is keyed on its own slug and is
credited from the `Co-authored-by` trailer its commits already carry.

**A secret badge gives nothing away.** It is drawn as a slot with `???` in it —
shown rather than omitted, because a list that simply ended would say the
collection was complete. The header reads `n / m+?` for the same reason: a total
that let the secrets be worked out by arithmetic would remove the point of
having any.

**The Hall of Shame is a section, not a verdict.** Committing straight to `main`
is acknowledged, never celebrated: a shame badge gets no reward moment, cannot
be equipped as a title, and is left out of the markdown that leaves the
application. It is hidden entirely at the Professional personality.

**The agent table has no human on it.** Ranking the person at the keyboard
against the models they are supervising turns a useful comparison — which of
these does well in *this* repository — into a productivity leaderboard, which is
the thing the feature exists not to build. Agents are ordered by first-pass rate
rather than by volume, because volume ranks whoever was given the most work.

## 1Q — Farm

**Built.** `src/routes/farm/+page.svelte`, `src/lib/farm/` and
`crates/spagitty-farm` (FEAT-073, redesigned in FEAT-108–113).

A farm starts with a wizard, one step at a time (FEAT-141): **Goal → Start
from → Crew & rules → Start**. Only a one-line goal is required; the crew and
rules come back as the last farm in the repository had them, and earlier farms
can be resumed or started from. The farm can start from issues, briefs and a
branch in progress, all of which go into the planning prompt with a map of the
repository. After it, the farm follows three phases: **Plan → Build → Wrap up**.
The tracker stays in the header; while the wizard is open it shows the
wizard's steps. Farm remains at 1Q on the rail; its
dot means a task needs the person, including checked, approved work waiting
to merge, in the repository the farm belongs to. So the dot works before the
screen is visited, the shell opens a repository's farm when it already has one
on disk (`farm_exists`) — never a repository without one, because opening
writes the agent registry, and never in place of a farm with a run in flight,
because the backend holds one farm and replacing it would stop a planner.
Reads stay event-driven, with one five-second screen clock while a run is live.

**Setup** puts crew, goal and rules in one scrolling column. Agent detection,
custom CLIs, capability chips, autonomy, checks, slots, tries, permissions,
CodeRabbit, AGENTS.md and leftovers use existing backend commands. Plan it
creates the farm, configures it, then asks the first eligible planner.
**Planning** shows the planner's live output and can stop just that run. A
finished plan is grouped into dependency waves; missing or cyclic
prerequisites remain visibly unordered. The kept tasks are accepted, the
omitted tasks discarded, then building starts. A failed step stops that sequence.

**Building** begins with the goal and a sentence counting work, checks and
tasks needing the person. Needs you has the merge or retry action on each
card. The four board columns are Up next, Working, Checking and Landed.
Dependency waits stay Up next, cancelled tasks have a toggle, and containers
expand their children. Every card has the same six steps: Queued, Worktree,
Working, Checks, Review and Landed. The right card shows crew and recent
events. The floating pill pauses/resumes, changes autonomy and slots, adds a
task and opens Rules. A quiet run is marked after three minutes; a finished
run is never shown as working. Pause leaves existing agents to finish.

**Task** is a deep link at `/farm?task=<id>`. Its full stepper, state-specific
actions and Output, Changes, Checks, Review and Hand-off tabs sit beside the
brief. Checks and review come from task evidence; unverified work never
gets a green pass. File counts come from an in-process Git range comparison.
Task editing, deleting, decomposition and extension actions remain available.

**Crew** and **Activity** use `?pane=crew` and `?pane=activity`. Crew shows
availability, live work, capability chips, records, traits and arguments.
Activity plots the recorded runs, check and merge events, then lists events
newest first with All, Needs you, Landed and Failures filters. Transcript
output belongs in Task, rather than flooding that history.

**Wrap up** appears when the backend says the farm is completed. It lists
landed tasks, recorded merge hashes, tries, contributions and the questions,
risks and proposed work from every task's hand-off. A question's answer is
saved with its task's brief; follow-ups can become tasks or prefill a new goal.
Cleanup only removes clean worktrees and merged branches in the farm
namespace, and leaves unmerged work, uncommitted changes and live tasks alone.
Checks are described as evidence from task branches, without claiming an
unobserved check on the destination branch.

Dark and light use the shell's tokens. Only the floating pill and the Setup
bar add blur;
the shell's reduced-motion rule stops loaders, halos and entrance animations.

## 1R — Review

**Built.** `src/routes/review/+page.svelte`, `src/lib/review/`, and the forge
and `review_state` modules behind it (FEAT-087).

A place to read a pull request, beside 1H. Pull requests is still where they
are browsed, created and merged; Review is where one is read closely enough to
answer it.

**The inbox** groups the open repository's pull requests by what each needs
from you: *Needs you* (you are a requested reviewer), *Back with you* (a thread
you started has an answer), and *Open on this repo*. Your own are left out.
Each card shows its size in one to three bars and how far you got; the preview
card beside them says what is worth knowing before you start. *All my repos*
lists the open pull requests anywhere on the host that involve you; opening one
opens the clone Spagitty knows of it.

**The list is the one 1H reads**, read when a repository opens — so the rail's
dot is right before the screen is visited — and on Refresh, never on a timer.

**What you have done on a pull request is kept** as one JSON file per pull
request under Spagitty's application data, not in the repository: it is the
reviewer's working state, and the same pull request from another clone is the
same review.

**The room** (FEAT-091) reads one pull request from its fetched head: the
touched files with their viewed ticks, the diff, and the Conversation as an
inset card. A tick is kept against the file's blob, so a file changed after it
was ticked comes back unticked. The diff is changed parts with folds between,
or whole files; one file at a time or all of them in one column, of which only
the rows in view are drawn. A second pill floats at the diff's foot with the
review's own controls — Changes | Whole file, One | All, the focus ruler, `Aa`
and *Viewed, next* — and the toolbar under the pane is unchanged. When the
head cannot be fetched, the room reads the host's patch and says so.

**Conflict fixes are told apart** (FEAT-092). Each merge in the pull request is
re-done by `git show --remerge-diff`, and the lines its resolution wrote are
followed to the head. A changed part holding them is a sky card (`--resolve`,
the fifth lane colour) naming the merge, with each side of the conflict a
click away; the files list marks such files and filters by Author and
Conflict fixes.

**Writing is held until Finish review** (FEAT-093). A `+` on each line, and a
shift-click on a line number for a range, open a box; what is written is kept
in the pull request's record against the head it was written on, drawn
warm-tinted under its lines, and sent only by *Finish review* with a verdict
and the words for the whole pull request. Threads take replies at once and are
resolved or reopened on the host; comments on the pull request as a whole are
listed as *whole PR*.

## 1S — Merger

**Built.** `src/routes/merge/+page.svelte`, `src/lib/merger/`, and
`crates/spagitty-core/src/merger.rs` behind it (FEAT-100).

Any two branches, and what merging them would do, before anything is written.
The graph's right-click menu still merges into the branch that is checked out;
dragging one branch label onto another opens Merger with that pair (TASK-056).
Conflicts (1D) stays for operations git stopped on by itself.

**The plan is a dry run, and says so.** Branch A on the left, B on the right,
the raised Result card between them. The receiving card is outlined in its
colour — A in `--side-a`, B in `--side-b` amber — and the arrow from the side
whose commits come in is solid with the count. *The result lands* Into A, Into
B or Into a new branch (`merge/<a>-<b>`, from A), with Swap; only a branch here
can receive one. *How it lands* offers Merge commit, Squash, Rebase then
fast-forward and Fast-forward only, disabled with its reason when both sides
have their own commits; *History after* redraws for each. The Result card says
what will happen in one sentence, three numbers, and the conflicts: how many,
in how many files, and where they were measured. *What changes* lists every
file either branch touched, conflicts first.

**Nothing is written.** The dry run is `git merge-tree --write-tree` (git
2.38), or for an older git a `merge --no-commit` in a scratch worktree under
the git directory that is removed afterwards. It is one dry run per pair, A
merged with B: direction and strategy are worked out from it on screen, and it
is asked for again when a branch is picked or the refs move.

**Merge now** (FEAT-101) opens the commit dialog when nothing conflicts: what
will be written, the message, and a button named for the strategy. The result
is committed from the dry run's tree with `commit-tree`, or replayed in a
scratch worktree for a rebase, and only then does the receiving branch move —
with `update-ref` against the tip the plan read, or, where it is checked out,
`merge --ff-only` in that worktree, which refuses rather than overwrite
uncommitted work. A branch that is not checked out is merged into without
checking anything out. Then the done state: *<target> now includes <source>*,
Open in Graph, Merge another.

**Resolving** (FEAT-102) is the shared three-column resolver: the conflicted
files with a dot per conflict, the files that merge on their own, and for the
chosen file one card per region — A | Result | B, always in that order, each
side by its own line numbers with its context dimmed, why it conflicts, and
the commit on each side that made the change. Take A, Take B, Both in either
order, Pick lines, Edit by hand, Reset, and All from A or B per file. Every
result line carries its A, B or ✎ badge; lines that will not land fade. The
result's line numbers follow the choices. Base shows what the merge base had
here. A pill at the foot steps through every conflict and jumps to the next
unresolved. The choices are kept in application data per merge, and are only
applied again to a region whose sides have not changed. Nothing is written to
either branch until the commit dialog's button.

**A rebase that stops** (FEAT-103). Rebase, then fast-forward replays the
source's commits in a worktree of Spagitty's own under the git directory,
named after the merge, so coming back finds it where it stopped. Each stop is
resolved with the same columns — *Rebasing <source> onto <target> · commit n
of N* — with Continue, Skip this commit and Abort; git's ours and theirs are
put back as A and B. Once every commit is replayed, Finish the rebase in the
commit dialog moves the receiving branch and removes the worktree. Neither
branch moves before then.

## 1T — Terminal

**Built.** `src/routes/terminal/+page.svelte`, `src/lib/terminal/`,
`src-tauri/src/terminal.rs`, `crates/spagitty-process/src/pty.rs` and
`mux.rs` (FEAT-130).

A real terminal on the rail, after Conflicts, so nobody leaves Spagitty for
one. ``Ctrl+` `` (``⌘` `` on macOS) opens it from any screen and goes back to the
one you were on.

**The pieces.** A PTY from `portable-pty` (ConPTY on Windows, openpty
elsewhere) runs the program. Its output crosses to the screen on a Tauri
channel as raw bytes, is gathered for a frame (`batch.ts`) and written to
xterm.js once, which is what lets a million lines go by without the window
stopping. xterm.js draws with WebGL and falls back to its own renderer where
that is refused. Scrollback is 10,000 lines.

**A multiplexer when one can run.** tmux, or psmux on Windows, on a socket of
Spagitty's own (`-L spagitty`), beside whatever tmux the person runs and never
touching it. One session for each repository, named for its folder with six letters made
from its whole path, so two repositories with one folder name never share a
session, attached
with `new-session -A -s <name> -c <path>`. What is in the PTY is the *client*:
*Detach*, closing the screen's terminal and closing Spagitty all end the
client, and the session carries on. Nothing the screen can ask kills a
session. `mux::choose` is the rule for which binary, a pure function: on macOS
and Linux the system tmux when it is 3.2 or later and otherwise the one
Spagitty ships; on Windows the psmux that shipped, and otherwise one on the
`PATH`. The pill in the header names it and says *bundled* when it is
Spagitty's; its hover gives the path and version.

**None, and nothing said.** Where no multiplexer can run, or it is switched
off in Settings › Behaviour, each tab is a plain shell, the Sessions panel and
the pill are not drawn, and no text on the screen mentions one. A terminal
that works is not missing anything.

**The shell** is the one in Settings › Behaviour, and otherwise the
platform's own: `$SHELL` on macOS and Linux; `pwsh`, then Windows PowerShell,
then `cmd` on Windows.

**The Sessions panel** lists the sessions on the socket with their windows and
whether anyone is attached, the windows of the one shown, and *Follow
repository*. Followed, opening another repository switches to its session and
makes it if it is not there. The lists are read again every two seconds while
the screen is shown, because a `tmux new-window` typed in the terminal changes
them.

**A terminal outlives the screen.** The emulators and their elements are kept
in the store, not in the component. Leaving takes the element out of the page;
the program runs on, its output is still written, and coming back puts the
same element back.

**Links are checked, never guessed.** A hex word of 7 to 40 characters and a
`path:line[:col]` are found on a line (`links.ts`), and the repository is
asked which are real (`files::known`): a word that resolves to a commit, a
path that is a file at `HEAD`. Only those are underlined. A commit opens on
the Graph and a place in a file opens in Files. What the repository said is
kept, so a screen of `git log` is asked about once.

**Keys** are in `keys.ts`, one table read by the emulator and by the layout.
With the focus in the terminal every key is the shell's, except
`Ctrl+Shift+P` for the palette, ``Ctrl+Shift+` `` to leave, and `Ctrl+Shift+C`
and `Ctrl+Shift+V` to copy and paste (`⌘C` and `⌘V` on macOS).

**What ships.** `tools/multiplexer.ts` runs before Tauri reads its resources.
It downloads one pinned asset for the target, from the project that makes it,
and refuses a file whose SHA-256 is not the one written beside it: psmux on
Windows, and the tmux project's own static build on macOS and Linux, with
their licence texts. A release build sets `SPAGITTY_MULTIPLEXER_REQUIRED`, so
a download that failed stops the build. `NOTICE` lists both.

**Not here.** Farm sessions in the panel are FEAT-135.

## 1U — Files

**Built.** `src/routes/files/+page.svelte`, `src/lib/files/`,
`crates/spagitty-core/src/files.rs`, `highlight.rs`, `blame.rs` (FEAT-131).

Any file at any branch, tag or commit: a tree, the files opened from it as
tabs, and one of them read, with blame beside every line.

**It cannot write, and that is structural.** `files.rs` takes a revision and
reads trees and blobs from the object database. It never asks the repository
where its working copy is, so there is no flag to forget and nothing to
disable. A test reads the module's source for a way to the disk, and another
reads a file after its working copy has been changed and then deleted. A bare
repository reads the same. *View only* in the header and `VIEW` in the status
line say so; neither is a mode there is another of. There is no edit button
and no *Open in editor*.

**What arrives when.** The text is read and drawn at once. Colour is asked
for a page of lines at a time, when rows are about to be drawn (`VirtualRows`'
`onrange`), and fills in after. Blame is asked for after the first paint. A
file of twenty thousand lines costs its text to open and what is in view to
colour.

**Colour** is tree-sitter, in Rust: `tree-sitter-highlight` with nineteen
grammars compiled in. A blob is coloured whole, once, and kept by its id — a
blob never changes, so nothing is ever invalidated. What crosses to the screen
is, for each line, pairs of token class and length; the classes are the twelve
`tok-*` the Diff screen already has, so the two screens read the same. A
language with no grammar here (Kotlin, Dockerfile, and anything else
`lib/diff/highlight.ts` knows) is read by that highlighter instead, and the
status line then names the language without *tree-sitter*. Over 5 MB a file is
shown without colour; over 50 MB it is not opened.

**Blame** is `blame::compact`: each commit once, and for each line an index
into them. Each run of lines from one commit has one head — portrait, name,
summary, age — and every line has a bar, stronger the newer the line is among
the file's own commits. A commit with a `Co-authored-by` trailer naming an
agent, by the rule gate 1 applies (`attribution.rs`, kept in step with
`tools/attribution.mjs` by a test), is drawn in `--agent` with the hexagon.
Pressing a block lights every line from its commit.

**The peek** opens under a pressed line, as a row of the list, never over the
code: who and when, the pull request the summary names, the summary, the sha,
and what the line was before — the line that commit's change removed in that
place. *Commit* selects it on the Graph, *Blame before this* opens the file at
the commit it was changed from, under the name it had then, and *Investigate
line* goes to 1O.

**Keys** are in `keys.ts`, a table tested as one: `j` `k`, Ctrl+d Ctrl+u,
`gg` `G`, `/` `?` `n` `N`, `]b` `[b`, `b`, Enter, Esc, and `y`, which
copies `path:line@sha`. None types into the file.

**The revision** is chosen from the branch menu (FEAT-122) with its tags and
*A commit…*, which takes whatever is typed in the menu's field as a revision.
A tab keeps the revision it was opened at and takes the screen there when it
is shown. Another screen opens a file here with
`/files?path=…&rev=…&line=…`.

The revision, the tabs, the open folders and the two switches are kept per
repository in `localStorage`.

## 1V — Recipes

**Built.** `src/routes/recipes/+page.svelte`, `src/lib/recipes/`,
`src-tauri/src/recipes.rs`, `crates/spagitty-core/src/recipes/` (FEAT-133).

Git steps in blocks: an ordered stack from a closed catalogue of eighteen,
each with a few parameters chosen from lists, run by Spagitty a step at a
time.

**The catalogue is closed, and that is the safety.** A step is an enum
(`recipes::Step`). There is no variant for a force push, `branch -D` or
`reset --hard`, so no recipe can hold one, whether it was built by hand,
proposed by an agent or imported from a file. A command is a list of
arguments and never a line for a shell. The screen's list of blocks is
`catalogue.ts`, and a Rust test fails when it and the core's come apart.

**The validator** is `validate.rs`: one pure function of the recipe and a
`World`, which is what is true of the repository. It walks the steps keeping
what they would leave behind (the branch checked out, the branches made, the
stashes pushed, whether pushed commits were rewritten) and its findings are
shown under the block they are about. It runs after every edit and again at
*Run*, against the repository as it is then. An error stops a run; a warning
does not.

**The agent's help is a switch**, off by default (the author, 2026-10-13).
On, the person writes a sentence and the agent proposes the blocks
(`propose.rs`), which arrive in the builder to be read and edited: its answer
is parsed into the same enum, so a block it invents is refused. During a run
it writes a message a step leaves to it and explains a failure. It runs no
command, and a command-line agent is started in an empty directory of its
own, never in the working copy. Off, the sentence, the note and *agent
writes* are disabled and nothing asks an agent for anything.

**A run** (`run.rs`) is stepped: `tick` runs one step or stops, `answer`
takes the person's choice. The level stops it before a step: every step at
*Ask every step*, a push and a branch being deleted at *Ask before pushing*,
nothing at *Full access*; *Read-only* runs nothing. These stop it at any
level: *Stop and ask me*, a conflict, a non-zero exit, a failed check, a
rejected push, and a tree that is not clean at *Stop unless clean*. A merge's
conflict can be taken to Merger and any other to Conflicts. The run is on a
thread in the backend and sends the whole run when it changes and each line
as it is printed. While a step runs it holds the activity lanes.

**The restore point** (`restore.rs`) is written before the first step under
`refs/spagitty/restore/<id>/`: where `HEAD` was, the tip of every branch the
recipe names, each stash, the index as a tree, and the working tree as a
tree with its untracked files, made through an index of its own. *Undo the
whole run* ends an operation left in progress and puts them back. It never
deletes a file git did not know about, and it cannot take back a push. The
newest twenty are kept, none longer than seven days, and Reflog lists them.

**The brief** (`brief.rs`) is `recipes.md`, compiled in with its version, and
then this recipe in the spec's form. The person sees it as it is sent. With
the help off there is none, and the panel shows the commands as a script.

**Kept** in the data directory, per repository: the recipes, and each run's
record. Nothing is written under `.spagitty/` in the repository.

**Keys** are in `keys.ts`: a block's letter adds it, `j` `k` walk the stack,
Alt with the arrows reorders, Delete removes, Ctrl+Enter runs, Ctrl+Z takes
an edit back. None fires while a field has the focus.

## 1Y — Pipelines

**Built.** `src/routes/pipelines/`, `src/lib/pipelines/`, `src-tauri/src/pipelines.rs`, `crates/spagitty-core/src/pipelines/` and `forge/pipelines.rs` (FEAT-138).

Preview reads the working copy's GitHub Actions and GitLab CI files. It resolves concrete local includes, anchors, references, inheritance and matrices, then draws dependencies in Needs or Stages columns. The chosen event, ref and changed files determine runs, approval, skipped or decided-on-host states. Bundled offline schemas and nearest-key hints report problems without executing workflow commands or fetching remote includes.

Jobs retain their source file and line. Open source shows those exact working-copy contents in a read-only panel. Matrices expand; workflows above 200 jobs start as stage summaries. The graph draws only cards and edges around the viewport.

Live reads runs on the default branch, personal watches and own open request branches. The shared forge clock reads active branches every five seconds and quiet branches every sixty, while visible. The run graph reads configuration at the run's actual commit. GitHub step states are available while running; its log follows completion. GitLab partial traces can be read while running. Cancel, re-run failed and GitLab lint each ask for confirmation and send one attempt. Nothing runs locally.

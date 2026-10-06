<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Screens

One section per screen. Each screen carries a short code — `1A`, `1B`, … — so it
can be named in one token in a commit message or a conversation. The codes and
their order come from the design handoff and are declared in `src/lib/nav.ts`.

**This document is updated by each screen's own work item**, in the same change
as the code. A section describing something that no longer exists is a defect
under Amendment 11.

| Code | Screen | Route | Rail | State | Item |
| --- | --- | --- | --- | --- | --- |
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
| 1O | File history | `/history` | no | Built | FEAT-063 |
| 1P | Badges | `/badges` | yes | Built | FEAT-072 |
| 1Q | Farm | `/farm` | yes | Built | FEAT-073 |
| 1R | Review | `/review` | yes | Built | FEAT-087 |
| 1S | Merger | `/merge` | yes | Built | FEAT-100 |

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

  Rail order is the screens roughly as they are worked through — Graph, Working
  copy, Conflicts, Branches, Stash, Pull requests, Rebase, Log — then a divider,
  then All repositories and Settings. Log follows Rebase because it is where you
  go to look something up rather than a step in that sequence.

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
hide and solo. Dragging one label onto another offers the three integration
verbs, with the gesture carrying the direction. Shift and Ctrl/Cmd build a
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
Personality, God mode, Appearance, License — because these are read rarely and
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

## 1O — File history

A dedicated view for inspecting a single file's commit evolution and line attribution (FEAT-063).

**The commit timeline follows renames.** `history::file_commits` walks commits touching the selected path via `git log --follow`, displaying author name, time, summary, and short hash.

**Interactive line attribution.** The right pane renders line-by-line blame metadata (author name, commit SHA, timestamp) alongside file content. Hovering a commit in the timeline or blame gutter highlights every line introduced by that commit.

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

**Built.** `src/routes/farm/+page.svelte`, `src/lib/farm/`, `crates/spagitty-farm`
(FEAT-073).

Supervising a small engineering team from inside the Git client. The plan is on
the left, the selected task in the middle, and the log along the bottom — the
three questions a supervisor has, answerable without navigating between them. A
person who has to move between those three is reading a log.

**The log is a drawer, and it has two tabs** (FEAT-074). *Activity* is what the
farm did — tasks created, statuses moved, verifications run, merges landed —
timestamped, filterable to one task, and as long as the history. *Transcript* is
what one agent said, which is thousands of lines and belongs to one task at a
time; keeping them in one list would drown the record in the narration. The
drawer is dragged to the height you want and collapses to its own tab bar.

Two controls, and they are not the same thing. **Following** is where the
scrollbar is: the pane sticks to the newest line while the reader is at the
bottom and lets go the moment they scroll up. **Hold** is a decision — it
freezes the list so a line can be read while it is still arriving, and counts
what arrived while it was held, so holding is never losing.

**The header is a ring, and the strip above the plan says who is working**
(FEAT-077). What is finished fills the ring, what is running is a brighter arc
at its leading edge, and anything blocked colours the remainder — an unfinished
farm and a stuck one are not the same state. One chip per working agent names
what it is on and for how long, and the strip is absent when nothing runs.

**A quiet run says so, and is never stopped for you.** After six minutes without
a word the chip and the row say how long it has been silent and the pulse stops.
A model may think for a long time; killing it throws the work away, so the farm
flags and leaves the decision where it belongs.

**Finished work is scored.** A task taken to Done hands the delight layer
(FEAT-072) what it needs to credit the agent that did it — including that
"nothing checked it" is not a pass.

**A planning run is visible while it runs.** A card under the header carries how
long it has been going, the last thing the planner said, and a control that
stops the planner without cancelling the farm (BUG-021).

**Nothing polls.** A farm changes when an agent says something, which is at a
model's pace and on no schedule. The backend emits, the store applies, and the
screen is a function of the store; a snapshot is refetched shortly after a burst
so nothing drifts if an event was missed.

**A branch and a worktree per task**, named `spagitty-farm/<task>/<provider>`.
The name is derived rather than chosen, in one place, because it is how the
graph, the worktree list and this screen find each other. Nothing an agent does
reaches the working copy, and deleting a task keeps the commits on its branch.

**An agent saying "done" is not done.** Verification runs the repository's own
commands in the task's worktree; review is performed by a different agent than
the one that wrote the change. A task that reached review with nothing
configured to check it says so, in those words, rather than reading as passed.

**The starter page answers three questions in the order they are asked** — what
a farm is, how to start one, and whether this machine can run one. The goal
field is on it, so starting a farm needs one sentence and no navigation. The
readiness rows say whether an agent was found, whether the repository has an
`AGENTS.md` for one, and whether anything verifies the work; none of them blocks
starting a farm, because a farm with no agent is still a plan.

**Spagitty runs agents; it does not contain them.** Claude Code, Codex, Cursor
and Oh My Pi are detected on `PATH`, and anything else with a command line can
be added by hand. There is no model here and no key.

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
The graph's drag still merges into the branch that is checked out; Merger is
for seeing the result first. Conflicts (1D) stays for operations git stopped
on by itself.

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

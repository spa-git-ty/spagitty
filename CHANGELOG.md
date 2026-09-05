<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Changelog

All notable changes to Spagitty, newest first. Entries are written into
`Unreleased` in the same change as the work they describe (Amendment 20), and a
version's section becomes that version's release notes verbatim — gate 6 reads
it with `tools/release-notes.mjs` and refuses to release without it.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).
Spagitty is at `0.x`: the surface is not yet stable, MINOR may break, PATCH
stays backward-compatible.

## [Unreleased]

### Fixed

- Keep development window sizing and alignment intact when component stylesheet
  requests arrive before their JavaScript.

### Documentation

- Correct the repository's amendments reference to the current shared book,
  including the frozen range and the roles of Amendments 19 and 20. Direct
  agents to read the canonical book from `AGENTS.md`.

### Reliability

- Measure frontend routes in coverage and check descendant termination and noisy
  verification on Linux, macOS and Windows before release.

- Automatic merges now require checks and successful independent review for
  the current committed work, plus merge permission. Tasks retain their intended
  destination, and competing farm merges serialize.

- Drain verification output continuously with bounded retention. Stopping a
  check or reaching its deadline terminates its process tree.

- Claim one completion watcher per run, including review runs, and keep late
  results attached to the attempt that produced them.

- Keep running agents and planners cancellable while background threads wait for
  them; contain Windows agent descendants in jobs before execution starts.

## [0.5.0] - 2026-09-05

The farm could run a plan but there was almost no way to watch it run. This is
that release: a ring rather than a count, a chip for every working agent, a log
you can read and filter, a queue that says why a task is waiting, and a task
too big for one agent that can be cut into smaller ones.

### Added

- **The Farm screen is worth watching.** `3 / 7 done` becomes a ring — what is
  finished fills it, what is running is a brighter arc at its leading edge, and
  anything blocked colours the rest, so a farm that is working looks different
  from one that stopped at the same point. Above the plan, one chip per working
  agent says who is on what and for how long, and disappears when nothing is
  running.
- **A run that has gone quiet says so.** After six minutes without a word the
  chip and the task row say how long it has been silent and the pulse stops.
  Nothing is stopped for you: a model may think for a long time, and killing it
  throws the work away.
- **Agents earn their badges from the farm now.** A task taken all the way to
  Done scores the agent that did it — whether the checks really passed, whether
  the reviewer approved, how many times it was sent back — which is the event
  the badge catalogue has been waiting for since it was written.
- **Every task says who asked for it.** A farm mixes work you decided on with
  work a model produced, and after twenty tasks they looked identical. Your own
  tasks stay unmarked; anything an agent asked for carries one quiet mark, and
  its panel says which agent and why — cut out of the goal, cut out of another
  task, or proposed while working on something else and asked for by nobody.
  Tasks from an existing farm read as yours, because at the time they were.
- **A task that turns out to be too big can be broken down.** **Break it down**
  asks an agent to cut one task into smaller ones; they arrive as a proposed
  plan under it, and the task becomes a heading that finishes when they do.
  Deleting the heading keeps the work that was under it. A plan may hold 24
  tasks rather than 12, a farm may run 8 agents at once rather than 4, and how
  many attempts a task gets before it needs a person is a setting rather than a
  constant.
- **A proposed plan is accepted in one action.** A planner's tasks arrive as
  drafts so a person reads a decomposition before five agents act on it — but
  approving one meant opening each task and pressing a button in its panel, and
  the list said nothing about tasks waiting on a decision. There is a band now:
  how many were proposed, a checkbox per task, add them all or the ones you
  want, or discard.
- **A queued task says why it is not running.** "Pending" with no explanation
  becomes the scheduler's own answer, on the row: which task is holding the same
  files, how many agents are working against the limit, that nothing installed
  can do this kind of work, or that the farm is not running. A task's own note —
  a verification failure, a reviewer's words — still comes first.
- **The farm's log is a drawer you can read.** Six lines in a footer, with no
  times and no scrollback, become a resizable drawer with two tabs: *Activity*,
  the farm's own record — timestamped, filterable to one task, as long as the
  history — and *Transcript*, what one agent is saying, which used to be
  collected and never shown outside a task's panel. The pane follows the newest
  line while you are at the bottom and lets go when you scroll up; **Hold**
  freezes it so a line can be read while it is still arriving, and counts what
  arrived meanwhile. Copy takes what is shown, filter and all.
- **A little motion, where it says something.** A log line arrives rather than
  appearing, and a task row washes with the accent for a second when its status
  changes under you — nothing that moves anything already on screen, and nothing
  at all for a reader who has asked for reduced motion.

### Changed

- **A long session stops growing.** Every run of every task stayed in memory for
  the life of the process and was copied into every refresh, so a farm left open
  for a day paid for everything it had already done. The history is bounded now
  — the recent runs, plus the newest of every task the farm still has, so
  nothing you can click on loses its record. Transcripts were always on disk and
  still are: the drawer can now read a task's **whole log** back from one.
- **Every farm event now carries the time it happened**, which is what makes a
  log a log. Events recorded before this show no time rather than a 1970 one.
- **Watching a farm costs almost nothing now.** The Farm screen refreshes after
  every burst of events, and that refresh used to run `git worktree list` and
  re-parse the whole activity log — on the thread that paints the window, four
  times a second while an agent was talking. The history is held in memory, the
  leftover-worktree scan happens when it can actually change, and a transcript
  line no longer asks the backend anything.

### Fixed

- **The farm sees the agents that are installed.** Opening the Farm screen said
  `Agents 0` and marked Claude Code, Codex and Oh My Pi "Not installed" while
  all three sat on `PATH`, with **Plan it** disabled and nothing able to run.
  Detection was working the whole time: the screen subscribed to the farm's
  events *after* asking it to detect, so the answer arrived a few hundred
  milliseconds later with nobody listening, and nothing asked again.
- **An agent's work is visible while it happens.** A task ran for minutes with
  an empty transcript behind it and everything arrived at once at the end,
  because Claude Code was being asked for its answer rather than for its work.
  It now runs in streaming mode, and what it streams is narrated into lines a
  person reads — `· Read src/auth.rs`, `· Bash cargo test`, and the agent's own
  words. Agents that already narrate their work are unchanged.
- **A planning run can be watched and stopped.** The Farm screen carries a card
  while a planner is working: how long it has been going, the last thing it
  said, and a control that stops the planner without cancelling the farm. A
  cancelled planner adopts nothing, and a planner that produces no tasks says so
  instead of leaving an empty plan that looks untouched.
- **Arrow keys on a panel divider resize that panel.** Keyboard resizing moved
  the graph's detail panel whichever divider was focused, so four of the six
  resizable panels could not be sized from the keyboard at all.
- **A verification is recorded, not just shown.** Verification events reached
  the screen live and were written to neither the log on disk nor the history a
  reopened farm reads back.
- **The window no longer freezes while the farm plans.** Asking an agent to
  break a goal into tasks locked the whole application — every screen, the
  menus, the window itself — until the planner finished, typically minutes. The
  planning run was waited on with the farm's session lock held, and every
  command that starts, stops or schedules a task needs that lock and ran on the
  main thread. Stop was locked out too, which is the control a person reaches
  for when nothing responds. The farm's commands now run off the main thread,
  and no lock is held across a wait, a cancel, or reaping an agent's process.
- **A flaky test stops failing the pipeline.** One of `spagitty-core`'s clone
  tests read the process-wide git-command record and could pick up a *different*
  test's clone, failing on a change that had nothing to do with it. It failed
  about half the time when the three clone tests ran together.

## [0.4.1-alpha] - 2026-09-03

The farm shipped in `0.4.0-alpha` with the brand's amber on every palette and
the repository's state stacked in the rail. This is the pass over both.

### Added

- **Four more palette families.** Nord (Snow Storm / Polar Night), Rosé Pine
  (Dawn / Moon), Solarized and Everforest, each in light and dark, bringing the
  set to eight families and sixteen palettes.

### Changed

- **Every theme now accents in its own hue.** The palettes all wore the brand
  amber — `#976317` in light, `#eeb04d` in dark — which put one colour on seven
  palettes built around a different one: an amber primary button in the middle
  of Dracula's purples, and a brown that read as muddy on every light
  background. Each family accents with a colour of its own now, contrast-checked
  the same way, and `themes.test.ts` fails if two families ever share one.
- **The Farm is the rail's first screen**, and the Graph follows it. The Graph
  still owns `/` and is still what the window opens on.
- **Open repository leaves the rail once a repository is open.** It was a filled
  accent button above every screen offering to replace the repository you were
  working in. The tab strip's `+`, the repository menu and All repositories all
  still open one.
- **The repository's state moved from the rail's foot to the status strip.** It
  was four lines stacked under the screens — the working-copy count, how fresh
  the walk and the remote were, tags and submodules — and two of them repeated
  counts the rail's own rows already carry as badges. It is one line along the
  bottom of the window now, between the identity and the licence, and the rail
  is navigation again. The line is two groups with a rule between them: what is
  changing (the walk, the working copy, the last fetch) and what is merely true
  (commits, tags, submodules), because reading them as one dot-separated run
  made "1 changed file" look like the same kind of fact as "Tags 42".
- **The Farm's first screen is a starter page**, not an empty state: what a farm
  is, the loop in four steps, a goal field that starts one without navigating
  anywhere, and three readiness rows — whether an agent was found, whether the
  repository has an `AGENTS.md`, and whether anything verifies the work.

## [0.4.0-alpha] - 2026-09-03

The first release to carry its pre-release suffix in the manifest: `main`
publishes `v0.4.0-alpha`, and the draft and prerelease lanes still number their
own alphas from the `0.4.0` base.

### Added

- **The agent farm (FEAT-073).** Spagitty runs and shepherds coding agents
  instead of only reading what they did. A new `crates/spagitty-farm` is the
  control plane and nothing else — every git operation still goes through
  `spagitty-core`. Inside it: the domain model, one adapter per provider,
  a branch and worktree per task with path-level lease locks, a deterministic
  verifier, a peer-review router, a dependency-DAG scheduler, and local
  persistence.
- **Adapters, not models.** Claude Code, Codex, Cursor and Oh My Pi are found on
  `PATH`, and anything else with a command line can be added by hand. Spagitty
  runs them as the user; it contains no model and ships no key.
- **A branch and a worktree per task**, named `spagitty-farm/<task>/<provider>`
  so the graph, the worktree list and the Farm screen find each other by one
  derived name. Nothing an agent does reaches the user's working copy.
- **An agent saying "done" is not done.** Verification runs the repository's own
  commands in the task's worktree, and the review is performed by a *different*
  agent than the one that wrote the change. Both are in the path to `Done`, and
  an agent's own report cannot skip either.
- **Agents never talk to each other.** Every handoff goes through Spagitty, so
  there is one audit trail and one place that decides what happens next.
- **Five autonomy levels**, from Manual to Unattended — a sentence about where
  the human is, rather than a magnitude.
- **The farm on disk** is JSON under `.spagitty/`, written by rename so a crash
  leaves the previous state intact, with events appended one object per line.
  The directory is added to `.git/info/exclude`; a farm is never committed.
- **Farm (1Q)**, the screen over it: the plan on the left, the selected task in
  the middle, and what just happened along the bottom — the three questions a
  supervisor has, all visible at once. Events drive it; nothing polls.
- **Tauri commands and an event bridge** streaming farm execution and progress,
  with unit and pipeline suites over the engine, the bridge and the frontend.
- **`go.farm` palette commands**, for reaching the screen without the rail.

### Fixed

- **Linux hung on interaction ("Application Not Responding").** WebKitGTK
  deadlocked with `at-spi2-registryd`; `platform::prepare_webview` now sets
  `NO_AT_BRIDGE=1` unless explicitly opted out.
- **Saving an agent took seconds.** `save_agent` probed the binary on every
  save, so toggling a role or a capability paid for a subprocess. It probes
  only when the executable path actually changed or was added.
- **A custom agent was judged by `--version`**, which many agents do not answer
  to — `dash` exits 2 on it, so every scripted agent was reported broken on the
  Debian-family machines CI runs on. It is judged by whether it runs.
- **Settings could not load or save external tools with no repository open.**
  Farm settings can render with no session, so it falls back to `~/.gitconfig`
  and surfaces a retry when the tools catalogue fails to load.
- **The Tasks view hijacked the settings form** when no farm had been started.
  Tasks now shows "No farm started yet", and Settings always renders the full
  panel — goal, autonomy, verification commands, repository rules and stale
  worktrees — whether or not a goal exists.
- **Views did not fill the window.** `.screen-slot`, `.screen`, `.body` and
  `.pane` now carry an explicit full height and `min-height: 0` so children
  compute their scroll boundaries, the scroll containers span the full pane so
  the wheel works anywhere in it, and the activity footer is pinned to the
  bottom instead of floating mid-screen when there is little content.
- **Scrollbars touched the card borders** on the Farm panes.
- **The README wordmark sat a row below the mark.** Pillow's default text
  origin is the top of the em box, not the baseline; the generator treated a
  centreline as a baseline and dropped the name by a full ascender. Lockups and
  the hero now share one optical centreline, and the hero carries the approved
  tagline under the name.

### Changed

- **The frontend coverage floor is 65%.** The farm's screens mount but are not
  asserted on, which put branches at 68.3%. Rust keeps its own floor at 70%.
- **README rewritten** in the Quiblo style: centered mark, badges, short pitch,
  a features table that covers the whole surface, and a "learn from this
  repository" section for humans and agents — instead of a handbook dump.
  Leads with the product description (gateway to a Git-managed agent farm),
  and names the farm as still being planned.
- **Gates 5 and 6 stay off for documentation.** A merge into `main` that only
  touches the README, `docs/`, `agile/` or brand collateral no longer rebuilds
  three platforms or tries to re-tag the current version. Application code,
  manifests and lockfiles still publish as before.

## [0.3.0] - 2026-09-02

### Added

- **Worktrees management (FEAT-062).** Complete lifecycle support for git
  worktrees. Users can view all linked working trees from the repository tabs
  menu or command palette, add new worktrees (with new branches, existing
  branches, or detached HEADs), switch active tabs to linked worktrees, lock
  and unlock worktrees with custom reasons, and remove or prune stale working
  trees.
- **File history and interactive blame view (FEAT-063).** Added a dedicated file
  history view (screen 1O, `/history`) and interactive blame inspector. The view
  renders the file's commit evolution timeline with rename following (`--follow`)
  on the left, alongside a line-by-line blame gutter with author portraits, commit
  hashes, and relative timestamps on the right. Hovering commits highlights all
  contributed lines, and commit chips link directly into the commit graph.
- **Diff syntax highlighting (FEAT-064).** Grammar-based code syntax highlighting
  across all diff views (Diff screen 1B, Working copy 1C, Stashes 1G, Pull
  requests 1H, and File history 1O). Features fast, memory-safe tokenization for
  Rust, TypeScript/JavaScript, Python, Go, C++, HTML, CSS, JSON, TOML, YAML,
  Shell, and SQL with automatic theme adaptation across all light and dark palettes.
- **Image and binary diffs (FEAT-065).** Rich visual image comparisons and binary
  delta inspection across all diff views. Supports 2-up (side-by-side) comparison,
  horizontal swipe split sliders, and opacity onion-skin overlays with transparency
  checkerboard backgrounds for PNG, JPEG, SVG, WebP, GIF, ICO, and AVIF images.
  Non-image binaries display formatted previous/new file sizes and byte deltas.
- **Diff content search (FEAT-066).** Search inside added and removed patch lines
  directly from Log search (screen 1I, `/search`). Extends `search::walk` with tree
  diff line inspection (`diff_content` query filter), supports combined filtering
  with author, message, path, and date ranges, and provides removable query chips.
- **Submodules management (FEAT-067).** Added submodules lifecycle inspection and
  actions. Users can view all submodules with their sync status, URLs, and recorded
  commit SHAs from the sidebar rail footer or command palette, perform recursive
  updates/clones, synchronize configured remote URLs, and de-initialize submodules.
- **External diff and merge tool launchers (FEAT-068).** Added configuration and
  launch triggers for external 2-way diff and 3-way merge tools (VS Code, Meld,
  Beyond Compare, KDiff3, Sublime, Vimdiff). Settings (screen 1K) provides tool
  auto-discovery from `$PATH` and `diff.tool`/`merge.tool` configuration, while
  diff file lists provide right-click context menu triggers to launch external tools.
- **Multi-identity profiles (FEAT-069).** Added author identity profiles allowing
  seamless switching between personal and work commit credentials (name, email,
  and signing keys). Includes a profile manager in Settings (screen 1K) and an
  active committer identity badge with a quick-switch dropdown in the status strip.
- **Extended forge integration and in-app PR creation (FEAT-070).** Added support
  for GitLab and Bitbucket Cloud forge remotes in addition to GitHub. Users can
  connect accounts with personal access tokens / app passwords, view merge requests,
  and create new pull requests / merge requests directly from the Pull requests
  workspace with target branch selectors and draft toggles.
- **Pull request lifecycle actions (FEAT-071).** Merge, close, reopen and mark
  ready-for-review from inside the Pull requests workspace, with the merge
  strategy (merge commit, squash, rebase) chosen per request and the forge's own
  mergeability state shown before the action is offered.
- **The Delight Layer (FEAT-072).** A badge, title and reward system that reads
  what actually happened in the repository. Thirty-five badges across six
  categories and five rarities, including evolution chains, secret badges and
  anti-badges; a Badges screen (1P) with the collection, equipped title and a
  shareable profile; per-actor attribution so agents earn separately from the
  human driving them, with agent standings ranked by first-pass rate. Nothing is
  awarded for merely using the application — every rule reads skill, discipline,
  recovery or quality. A Personality setting (Professional, Balanced, Full
  Spagitty) governs how loudly any of it speaks, and a Sound setting (off,
  subtle, full) synthesises its cues rather than shipping audio assets.
- **God mode (FEAT-072).** A Settings section that fires any delight event,
  previews any badge, and grants or revokes them, so the reward moments can be
  seen without waiting for the repository to produce them.

### Fixed

- **The Accounts chip in Settings opened the License section.** The section list
  had eight chips and the screen had seven branches, so `accounts` fell through
  to the `{:else}` catch-all. Account settings already live on the You section,
  so the chip is gone, `#accounts` redirects to `you`, and the catch-all is
  replaced by an explicit branch — a chip with no branch is now a test failure
  rather than a silent redirect.
- **Every `<select>` in the application was drawn by the operating system.**
  `appearance` appeared nowhere in `app.css`, so WebKitGTK painted its own
  widget and ignored the theme: a white field on a dark palette. All five
  selects are now themed, with the chevron drawn in `currentcolor` so it follows
  the palette.
- **The External Tools section was themed against tokens that do not exist.**
  `--fg`, `--dim` and `--bg-2` each fell through to a hard-coded dark fallback,
  which meant the section was the wrong colour on every theme but one.
- **The Open repository button pushed its label to the far edge** of the
  navigation rail, because it inherited the rail item's `space-between`.


## [0.2.0] - 2026-08-30

### Added

- **The PR review workspace (FEAT-059).** Clicking a pull request opens a
  dedicated full-window workspace: a header with the title (auto-scrolling on
  hover), author, timestamps, checks, commit count and review status; a
  collapsible left pane of all changed files and commits; a rendered CHANGELOG
  view of the PR description; an interactive diff with inline comment threads —
  replies and resolving change requests — and draft comments that persist in
  `localStorage` until a batch review is published. A flat shimmer replaces the
  old side panel while PRs load.
- **Spagitty, with a face: the brand (FEAT-060).** The app identity is now the
  author's own hand-drawn mark — an amber plate (`#EEB04D`) with four dark
  strands — copied verbatim into `assets/brand/mark.svg` and shipped as the
  app icon at 16/32/128/256/512/1024 (`@2x`, `.ico`, `.icns`) plus the
  favicon, README hero, wordmark lockups and tray/menubar marks, all
  regenerated from that one SVG. `docs/branding.md` is the reference, and the
  app's interactive accent follows the brand amber (`#EEB04D` on dark
  surfaces, darkened to `#976317` on light ones). Gate 2 refuses drift between
  the generators and the committed art. The README, previously empty, now
  ships the hero and the story.
- **Brand guide, interactive showcase, and UI identity integration (FEAT-061).**
  A complete brand authority guide was authored in `docs/branding.md`, and
  `assets/brand/preview.html` was rebuilt as an interactive offline showcase with
  theme toggling, click-to-copy tokens, clearspace visualizer, and platform tray
  simulators. The `BrandMark` vector component is integrated directly into the
  window TitleBar, All Repositories empty state, and Settings About section.

### Changed

- **The frontend toolchain runs on bun (TASK-027).** `bun.lock` replaces
  `package-lock.json`; the CI, build and release workflows install and run
  through `oven-sh/setup-bun` and `bun` commands; and the bundled license list
  in Settings reads the installed frontend tree instead of the npm lockfile.
  npm and node are no longer needed by the project's own tooling. Gate 4 audits
  JS advisories at `--audit-level=high`, matching the pre-bun `npm audit`
  policy.

## [0.1.0] - 2026-08-29

The first release. Everything below is new, so it is one `Added` section rather
than a pretence that anything changed.

### Added

- **The graph** — the commit history as a lane graph with author portraits,
  branch chips carrying divergence, square lane turns, lane compression, and a
  detail panel that can be put away. Search, reflog and tags each have a screen
  of their own.
- **Working copy** — stage, unstage, commit (with signing), discard changes,
  and a diff screen with split and unified views.
- **Branches** — a sortable, resizable table with two-sided divergence bars;
  create, delete and rename; checkout from the toolbar's branch picker.
- **Stash** — list, pop, apply, drop, and browsing a stash entry file by file.
- **Rebase** — interactive rebase planning and execution, and a conflicts
  screen that resolves and writes.
- **Remotes** — fetch, push and pull; clone; remotes management in Settings.
- **Pull requests** — read from GitHub for the open repository, split into
  what needs you and what is waiting on others, with review and check states.
- **The chrome** — repository tabs, a grouped toolbar, a collapsible nav rail,
  a status strip, themes, frosted-glass menus and dialogs, and the git command
  behind each action shown as it runs.
- **Update check** — Settings says when there is a newer Spagitty.
- **Release lane** — releases are tagged from `main` with notes taken from
  this changelog; a manually triggered `dev` build publishes an alpha
  (`-alpha.N`), and a draft build can be cut from any branch without tagging.
  Every lane builds Linux, Windows and macOS; the draft lane ships a `.dmg` for
  Apple silicon and one for Intel. Builds are unsigned, and the release notes
  say what that means on each platform.

`v0.1.0-preview.1` and `v0.1.0-preview.2` were early preview builds cut before
this changelog existed. They remain published as pre-releases and are
superseded by `0.1.0`; they are not withdrawn, only preceded.

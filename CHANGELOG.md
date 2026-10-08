<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Changelog

All notable changes to Spagitty, newest first. Entries are written into
`Unreleased` in the same change as the work they describe, and a
version's section becomes that version's release notes verbatim — gate 6 reads
it with `tools/release-notes.mjs` and refuses to release without it.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).
From 1.0.0, MAJOR is the only bump that may break what came before; MINOR
adds behaviour and PATCH fixes it, both backward-compatible.

## [Unreleased]

### Fixed

- Preserved and echoed `thoughtSignature` / `thought_signature` for Google
  Gemini thinking models (such as Gemini 3 and 2.5) across multi-turn tool
  exchanges, preventing 400 validation failures.
- Handled the `default_api:` tool prefix emitted by Gemini function calls so
  remote tools execute and narrate cleanly while preserving the original name
  in function responses.

## [2.0.0] - 2026-10-08

A MAJOR: agents can review and merge, and you choose how far each one goes
alone. With no agent set up, Review and Merger are exactly as they were.

### Added

- **Settings › Agents**, where agents are attached once for the machine. On
  this machine: Claude Code, Codex, Cursor and Oh My Pi as Spagitty finds them,
  with their version and path, and any command-line agent added by hand.
  Through an API: Anthropic, OpenAI, Google, or any endpoint that speaks the
  OpenAI-compatible chat API, such as OpenRouter, Ollama or LM Studio. Each
  agent says what it may do — Review, Merge, Farm — and **Test** runs it once
  and says what answered.
- **Each repository's rules** beside them: the highest level allowed, which
  agents may be used, the branches an agent may never land into unattended,
  whether an agent may approve or request changes on the host, and whether its
  comments say so. Unattended is not allowed anywhere until you raise it.
- **Assign an agent…** in Review, beside Start review and Check out branch, and
  in Merger, beside Resolve N conflicts. Pick the agent, the level and an
  optional note.
- **Four levels.** Suggest: the agent runs to the end and nothing is applied
  until you accept it. Step by step: it stops after the plan, each file, each
  conflict and the checks. Sign off: it drafts and resolves as it goes and stops
  before the review is sent or the merge lands. Unattended: it sends or lands
  itself. Whatever the level, it stops when it is unsure, when a check fails,
  when a limit is reached, and when the pull request or a branch moves.
- **An agent's work is your material.** In Review its findings are pending
  comments with its name, under their lines, with Accept, Edit, Dismiss and
  Why. Only the ones you decide go out with Finish review. In Merger its
  resolutions are the resolver's own choices: Take A, Take B, Both, Pick lines
  or Edit. Text it wrote gets a fourth badge, a hexagon in the agent's colour.
- **The Agent card** shows what the agent is doing while it does it: one
  sentence, a meter, a timeline of its steps with what it read and ran, and
  the raw output. Pause, Stop, Take over, change the level, or tell it
  something. A stopped run can be resumed from its last finished step.
- The rail's dot shows on Review or Merger while an agent waits for you, with a
  notification when Spagitty is not in front. Every agent control is also in
  the command palette.

### Changed

- **Agents are set up for the machine**, in Settings, rather than per
  repository in Farm › Setup. A repository's `.spagitty/farm/agents.json` still
  overrides what the farm uses. Agents added by hand in your repositories'
  farms are offered for the machine list once. A 1.x Spagitty opening a
  repository that 2.0 has used will not see agents defined only on the machine.
- **Spagitty can now send code to a model provider**, which 1.x never did. It
  happens only when you assign an API agent, and only to that agent's provider,
  with your own key. What is sent: the pull request's description, its open
  threads and the changed files, or the conflicting regions and their sides,
  and any other file the agent asks to read. Each repository asks once before
  the first time, each step lists the files it sent, and an endpoint on this
  machine, such as Ollama, sends nothing anywhere. Command-line agents run on
  this machine, as the farm's always have.
- An agent's comments carry *Drafted with* and its name on the host, and the
  review says how many were drafted. A merge commit that lands an agent's
  choices credits it in a `Co-authored-by` trailer.

### Security

- An API key goes to the system keychain, beside the forge tokens, and nowhere
  else: not the settings file, not the window, not a log line or an error.
- An agent never works in your working copy. It reads a scratch copy under the
  git directory, in its own read-only mode where it has one. Spagitty checks
  after every step and puts back anything it wrote. It cannot push, delete a
  branch, touch another person's thread or review its own commits.
- Requests to a model provider go through one module, over `https` except to an
  endpoint on this machine, with no redirects.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.3.2. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, compare the download
  with `SHA256SUMS-*.txt`: download it again if it differs, and open an issue if
  it matches. After updating, macOS may ask once for the keychain password to
  read a connected account's token or an agent's key; choose **Always Allow**.

## [1.3.2] - 2026-10-08

### Fixed

- Review lists the pull requests you opened, last, under **Yours**. It used to
  leave them out, so on a repository where every open pull request was yours it
  said there was nothing to review.
- A long comment in the review room's Conversation card can be read in full:
  **Show more** opens the whole comment and its replies.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.3.1. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, compare the download
  with `SHA256SUMS-*.txt`: download it again if it differs, and open an issue if
  it matches. After updating, macOS may ask once for the keychain password to
  read a connected account's token; choose **Always Allow**.

## [1.3.1] - 2026-10-08

### Fixed

- The pull request diff colours its code, in both the unified and the split view,
  the same way as the Diff screen.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.3.0. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, the download is
  corrupt: compare it with `SHA256SUMS-*.txt` and download it again. After
  updating, macOS may ask once for the keychain password to read a connected
  account's token; choose **Always Allow**.

## [1.3.0] - 2026-10-08

### Added

- **Pull request notifications.** Settings → Notifications watches your
  connected GitHub and GitLab accounts while Spagitty is open. It tells you
  when your pull request is merged or closed, when someone comments or
  reviews, when you are asked to review, and when your checks start failing.
  You get a notice in the app, an optional system notification, and
  Spagitty's own sound. Choose which kinds to hear about and how often to
  look. Off by default.

### Changed

- **Sound is its own setting.** Personality Off or Professional no longer
  forces sound off or greys out the Sound choices.
- **The Farm says less.** Explanatory paragraphs, column captions and repeated
  lines are gone from setup, planning, plan review, the board, crew and wrap-up.
- **God mode is offered only to the author's connected GitHub account.**

### Fixed

- **Check for updates reports a GitHub rate limit as a rate limit**, not as
  "the token does not have access to this repository". Every forge request now
  keeps the host's error body, so its message reaches the screen.
- **macOS asks for the keychain password once per account per launch**, not on
  every forge request.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.2.0. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, the download is
  corrupt: compare it with `SHA256SUMS-*.txt` and download it again. After
  updating, macOS may ask once for the keychain password to read a connected
  account's token; choose **Always Allow**.

## [1.2.0] - 2026-10-07

### Added

- **Farm shows the whole journey:** Crew, Goal, Plan, Build and Wrap up on
  every screen, with stable agent badges and six-step task tracks.
- **The Farm board says what needs you.** Checked and approved tasks can land
  from their cards; stuck tasks can retry with another available agent. The
  board separates waiting, working, checking and landed work, and flags quiet
  agents after three minutes. Pause leaves current agents to finish. The
  rail's Farm item carries a dot while something waits on you.
- **Task links open their full journey**, live output, real file-change counts,
  checks, review and hand-off, with the task's brief beside them.
- **Farm setup brings crew, goal and rules together.** Planning streams into
  a proposal grouped by dependency waves. Omitted prerequisites prevent
  starting a broken plan, and accepting, discarding and starting happen in order.
- **Crew and Activity show who did what:** agent availability, arguments and
  records, a timeline of real runs, and a filtered event log.
- **Farm wrap-up lists what landed**, merge hashes, contributions and every
  task's hand-off. Cleanup removes clean merged farm worktrees and branches;
  uncommitted or unmerged work stays.

### Fixed

- **Manage Profiles opens Settings without restarting the session.** The
  status-strip menu now navigates within the app to Settings → You.
- **Review and Pull requests use the same header and empty-state layout.**
  Both name the repository in the same pill and centre empty states in the
  window. Review shows the current repository; the scope controls are removed.
  A missing account on either screen says *No account is connected* and offers
  Settings → Accounts.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.1.0. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, the download is
  corrupt: compare it with `SHA256SUMS-*.txt` and download it again.

## [1.1.0] - 2026-10-07

### Added

- **Git hooks you can see, skip and watch.** Settings → Hooks shows the
  repository's hooks and what each runs — Husky's scripts, lefthook's and
  pre-commit's steps — and switches them off for that repository. Before a
  commit runs hooks you are asked: Run, Skip or Cancel; *skip hooks* on the
  commit bar skips them for one commit. Running hooks show their output live
  in a window, and a failing one keeps your message.
- **Rebase, redesigned to read like Merger.** Your branch on one side, the
  branch it is replayed onto on the other, and the result between them —
  how many commits, what is folded or dropped, which may stop on a conflict
  — before anything is written. Pick, reword, squash or drop each commit on
  its own card, reorder by dragging, type a reword's new message in place,
  and see the history the plan leaves.
- **Review a pull request from Pull requests.** An open pull request has a
  **Review** button that takes it to the review room, to comment on lines,
  answer threads and finish the review; your own has **Reply in Review**.
- **Branch names a branch where you are.** The bottom bar's Branch opens a
  name field on the graph, in HEAD's row: type a name, press Enter, and the
  branch is made there and checked out — no dialog. *Create branch here* on
  any commit does the same in its row.

### Fixed

- **Changes made in your editor show up straight away.** The working-copy
  count and the graph's uncommitted row update as soon as a file is saved;
  ignored build output does not trigger anything.
- **Branch makes the branch where you point**, at the selected commit or HEAD,
  and asks before creating and checking it out. **Stash** stashes your work
  right there, with an editable *WIP on <branch>* message. Clone and Rebase
  left the bottom bar.
- **Review says when no account is connected**, like Pull requests, and both
  centre their empty states.
- **Review loads with the main loader**, and its Conversation card no longer
  spills its chips past its edge. The card can be hidden into a tab at the
  edge and brought back.
- **Scrollbars stay out of sight until something scrolls.**
- **The Log screen matches the rest of Spagitty.** The search, the results
  with each author's face, the opened commit and Blame are on cards, laid
  out like Merger and Rebase.
- **Graph nodes are bigger, and merges turn like GitKraken's.** Portraits and
  merge dots are larger, and a merge's line to its other parent leaves the dot
  sideways and turns once, with no short stub beside the node.
- **Double-click the Branch / Tag column's border to fit the names.** The
  column widens to the longest labels, like a spreadsheet column.
- **Author pictures on GitLab and Bitbucket.** Self-hosted GitLab included:
  the instance is asked for each author's picture, with your connected
  account's token where it needs one.
- **Pull requests loads with the same loader as every other screen.**
- **The status bar shows the right version**, without the licence label.

### Downloads

- **Linux, Windows and macOS** (Apple silicon `*-macos-arm64.dmg` and Intel
  `*-macos-x86_64.dmg`), as in 1.0.1. The Mac builds are signed ad hoc and are
  not notarized: on first open macOS says the developer cannot be verified.
  Open the app once from the right-click **Open** menu, or allow it under
  **System Settings › Privacy & Security › Open Anyway**, and macOS remembers
  the choice. If macOS says the app is **damaged** instead, the download is
  corrupt: compare it with `SHA256SUMS-*.txt` and download it again.

## [1.0.1] - 2026-10-06

### Added

- **macOS downloads in the release.** Each release now carries a `.dmg` for
  Apple silicon (`*-macos-arm64.dmg`) and one for Intel (`*-macos-x86_64.dmg`)
  beside the Linux and Windows downloads, so one release has every platform.

  The Mac builds are signed ad hoc and are not notarized: they carry a real
  code signature, but no Apple developer identity. On first open macOS says the
  developer cannot be verified. Open the app once from the right-click
  **Open** menu, or allow it under **System Settings › Privacy & Security ›
  Open Anyway**, and macOS remembers the choice. If macOS says the app is
  **damaged** instead, the download is corrupt: compare it with
  `SHA256SUMS-*.txt` and download it again.

## [1.0.0] - 2026-10-06

### Added

- **Merger: see a merge before it happens.** A new screen on the sidebar.
  Pick any two branches and where the result lands — into either one, or a
  new branch — and how: a merge commit, a squash, a rebase, or a
  fast-forward. Before anything is written it shows which branch receives the
  result, how many commits come in, every file that changes, and whether and
  where they conflict, found by a dry run that leaves your repository as it
  was. Merge now lands it — into a branch that is not checked out, too,
  without checking it out or touching your working copy.
- **Resolve conflicts every way, and see where each line came from.**
  Merger and Conflicts share a new three-column resolver: your side, the
  result and theirs, one card per conflict. Take either side, both in either
  order, single lines from each, or type the result yourself; every result
  line is marked with where it came from, and the base is a click away.
  Choices made in Merger are kept, so you can leave and come back.
- **A rebase you can stop and resume.** In Merger, Rebase, then
  fast-forward replays one commit at a time in a worktree of its own, stops
  where a commit conflicts, and lets you resolve it, skip it or abort; neither
  branch moves until you finish, and leaving the screen does not lose your
  place.
- **Review, a place to read pull requests.** A new screen beside Pull
  requests, with an eye on the sidebar and a dot while a review is waiting on
  you. Its inbox groups pull requests by what they need from you — asked to
  review, answered after you commented, or open — and shows how big each is
  and how far you got. A preview says what is worth knowing before you start.
  *All my repos* lists the ones involving you everywhere on the host.
- **GitLab, all the way through.** Merge requests now show their files,
  commits and line comments; replies land in their thread; a review with
  comments is published as one batch and an approval is pinned to the version
  you read. Projects in nested groups are found, and a self-hosted GitLab
  under any name connects as GitLab.
- **Check out a pull request's branch.** From Review, a pull request's head
  is fetched and checked out as its own branch, so you can build and run it. A
  branch of yours is never moved: if the name is taken here, it is `pr-N`.
- **Settings › Reading.** Choose how code is set — Atkinson Hyperlegible Mono,
  OpenDyslexic Mono, Lexend, JetBrains Mono or your system's — its size, line
  spacing and letter spacing, the interface font, and calm or classic diff
  colours with the changed words marked. Diff, Working copy, File history and
  Review all follow it, and every face ships with Spagitty, so it works
  offline. Diffs now default to the calm reading set.
- **The review room.** Open a pull request from Review and it lands on the
  first file you have not viewed. Read each file as its changes, with the
  unchanged lines folded, or whole; one file at a time or all in one column.
  Tick files as viewed — a file the author changes afterwards comes back
  unticked. A focus ruler follows the line you are on (`j` and `k`), `Aa`
  switches the reading font, and the threads sit under their lines and in a
  Conversation card that takes you to them. Code is in colour, comments read
  as their host draws them, and the files and the Conversation card — like
  the inbox's preview — widen from their edge.
- **Conflict fixes, told apart.** Code a pull request's author wrote while
  resolving a merge conflict is framed in blue in the review room and never
  shown as their own work: which merge wrote it, the exact lines, and what
  each side had there. Filter the files by the author's work or the conflict
  fixes. Needs git 2.36 or later.
- **Review comments that wait for you.** In the review room, comment on a
  line or shift-click to cover a range; what you write stays pending — even
  across a restart — until *Finish review* sends it all with Approve, Request
  changes or Comment and a note on the whole pull request. Reply to threads,
  and resolve or reopen them on GitHub and GitLab from Spagitty.
- **Timings in God mode.** Settings › God mode › Timings shows what each
  command spent holding the open repository and waiting for it, every call
  from the screen and how long it took, and how long screens and diffs take
  to paint — so slowness can be measured before it is fixed.
- **Extensions.** Spagitty can run extensions: separate programs that add
  commands, buttons on the Commit, Farm and Pull request screens, settings,
  panels and code reviews. Install one from a `.spagitty-extension` file in
  Settings › Extensions, turn it on per repository, and see before installing
  exactly what it is, what it asks to do and which files it contains. Updates
  keep the previous version for rolling back, and removing one removes its
  commands everywhere at once. An extension is a program that runs with your
  permissions; Spagitty only does for it what you allow, but it is not a
  sandbox, and the screens say so.
- **A kit for writing them.** `bun run ext` creates, tests, attaches, builds,
  packs and inspects extensions, with a TypeScript SDK, a fake Spagitty to test
  against, and a published manifest schema and protocol any language can
  implement. `examples/extensions/hello` is the whole path.
- **CodeRabbit reviews, in Spagitty.** The first official extension reviews
  your uncommitted or committed changes with your own CodeRabbit CLI and
  account, after showing exactly which files would be sent. Findings arrive as
  the review runs, with their severity and location as CodeRabbit gave them;
  a review can be cancelled, is marked out of date if the code changes under
  it, and is never mistaken for approval. A billing confirmation is shown and
  never answered for you. Findings on a committed change can be sent to the
  farm as a draft repair task. It is off until you turn it on per repository,
  and needs the CodeRabbit CLI 0.7.7 or newer, installed separately.

- **Additional farm reviews.** Off, Advisory and Required policy, exact committed
  evidence, live provider checks on manual and automatic merges, and bounded
  repairs through the existing task flow preserve independent review and autonomy.
- **CodeRabbit on GitHub PRs.** Attributed summaries, findings and review/check
  status, explicit incremental/full requests, revision staleness and uncertain
  delivery handling use backend-owned forge credentials.
- **Bundled official worker.** Target-specific native packages are built before
  Tauri bundles them; macOS workers use its sidecar signing path.

### Changed

- **The sidebar shows the tools again.** Stash, Tags and Reflog are back as
  places of their own, and Rebase, Log and All repositories are always there
  rather than only while open.
- **Dragging a branch onto another opens Merger** with that pair, so you see
  what the merge would do — and whether it conflicts — before it happens. The
  right-click menu still merges into the branch you are on straight away.
- **Code in colour, as it is.** Diffs and file history colour Kotlin, Java,
  Gradle, Swift, C#, CSS, HTML, XML, Markdown, Dockerfiles and more, and each
  kind of word — keyword, string, number, function, type — has a colour of its
  own; keywords and numbers, types and strings no longer share one.
- **A new face.** The icon is three cream strands on a tomato plate that cross
  and then run straight, each ending in a commit — clear down to the smallest
  taskbar size. The name is set in Sora with "git" in tomato, in the title row
  and on the welcome screen, and every icon, tray mark and favicon is new.
- **Pomodoro, a theme of Spagitty's own**, and the only one: tomato, basil and
  saffron on warm cream by day and warm charcoal by night. The eight borrowed
  palettes — Catppuccin, Dracula, Tokyo Night, Gruvbox, Nord, Rosé Pine,
  Solarized and Everforest — are gone, and an install that was on one of them
  opens on Pomodoro. Under Omarchy the desktop's own palette can still be
  followed.
- **A spatial shell.** The window is one pane floating over a softly lit
  environment, with the controls around it as floating objects instead of bars
  across it. The open repositories are pills in the one row above the pane. The
  sidebar is a pill of icons beside it that shows its labels when you hover or
  tab into it. The toolbar is a pill centred below the pane, with the status on
  either side. Controls, menus and dialogs are rounder. Every screen's content,
  the graph above all, is unchanged.
- **The screens inside it speak the same language.** Headers and the graph's
  column no longer sit in filled bars; columns are separated by space instead
  of lines; hovering or selecting a commit draws a rounded highlight; the
  working copy and the commit detail are floating cards; branch labels are
  capsules and tags keep a shape of their own. The lanes are drawn exactly as
  before.
- **Less to read.** Settings lost the labels and sentences that explained its
  own controls: a value shows where it is set, the rest lives in the control's
  tooltip. Screens no longer end in a paragraph about themselves, and keyboard
  hints and teaching empty states are gone.
- **Every screen in the same language.** Branches, Tags and Reflog are lists
  like the rest: no line under every row, the same text size, and their
  buttons appear on the row you point at instead of down every row. Screens no
  longer draw a line under their header or over their footer. A long tag name
  ends in "…" instead of running into its message. The Farm's title matches
  the other screens and its start page is one card, not a card of cards; File
  history's prompt sits in the middle of the window.
- **File lists you can read.** Every list of changed files names the file
  first and whole, with its folder after it, instead of cutting the path from
  its start. Each change carries a coloured letter — M, A, D, R, U — in place of
  `~` and `?`. Rows are no longer boxes, and stage and discard appear as
  round buttons on the row you point at. A stash shows what you wrote first.
- The commit detail's **Cherry-pick** and **Revert** work; they were labels
  that did nothing. The status line says the repository is ready once its
  history is on screen, rather than "Loading history…" for the whole session.
- **A commit bar.** On Working copy, the commit message is one line along the
  bottom — the summary, *Add description*, amend and Commit — instead of a box
  over the top half of the diff. The description opens when you ask for it.
- **A shorter sidebar.** It shows Farm, Graph, Working copy, Branches, Pull
  requests and Settings. Conflicts joins it while something is in conflict.
  Rebase, Log and All repositories are still one click away — the toolbar,
  `Ctrl+F`, the tab strip's `+` or the command palette — and show in the
  sidebar while they are open.
- **Branches, Tags, Stash and Reflog are one place.** Each opens with tabs for
  all four, with their counts.
- **Badges and reward moments are off by default.** A new *Off* personality
  records badges without showing anything; choose Professional, Balanced or
  Full Spagitty in Settings → Personality to see them. An existing settings
  file keeps the personality it names. God mode and the Badges screen are
  offered only while the layer is on.

### Fixed

- **Loading shows, and notices look like the rest.** Screens that are reading
  show three weaving strands instead of the word "Reading…", buttons show them
  while their action runs, and notices are glass with a green tick or a red
  exclamation. The graph's lanes keep closer to their full spacing.
- **The graph's lanes are no longer squeezed together.** A busy history has more
  room by default, and dragging the graph column wider spreads its lanes back
  apart.
- **Branches, Tags, Stash and Reflog show only their own title**, now that each
  has its own place on the sidebar.
- **Dragging the Graph column wider shows more of a busy graph.** On a
  history with more branches side by side than the column has room for, the
  lanes past its edge were stacked on one line, and widening the column left
  them there. Now a wider column draws more of them apart; the lanes already
  apart stay exactly where they were.
- **One monospace face.** Commit ids, counts, ref names and the version were
  set in your desktop's monospace — Consolas on Windows — beside code set in
  the face chosen in Settings › Reading. They now use that face too, or
  Atkinson Hyperlegible Mono if the code face is Lexend, which is not
  monospaced. In the reflog, "created at" no longer wraps onto two lines.
- **Pull request descriptions read as written.** Tables, nested lists, task
  lists and the `<details>` blocks bots write are drawn as on GitHub and
  GitLab, and code blocks are coloured. A link to anything but a web or mail
  address — a `javascript:` one above all — was made clickable; it no longer
  is, and nothing in a description can run.
- **No more console windows on Windows.** Every git command Spagitty ran
  opened a console window of its own: a flash for a commit, and a window that
  stayed for as long as a fetch took. They run out of sight now.
- **GitLab no longer says every merge request needs you, or that it passed.**
  A merge request needs you when you are one of its reviewers, and its checks
  are its latest pipeline's — shown once read, never assumed.
- Extension operations waiting on host-owned tools no longer trigger inactivity
  expiry; absolute deadlines and cancellation still apply, including completion races.
- Expired PR confirmations close, and late review responses cannot replace another
  repository’s dialog or history. Windows desktop tests and releases share one manifest.
- **A self-hosted forge behind a company certificate connects.** Spagitty
  checked certificates against a bundled list of public authorities and
  ignored the ones your computer trusts, so a GitLab signed by an internal CA
  was reported as unreachable. It now uses the system's certificate store.
- **A GitLab token connects as GitLab.** Settings checked every token as if it
  were for GitHub, so a GitLab token failed with "Field 'viewer' doesn't exist".
  The host's name now decides which service the token is checked against.
- **A one-line change on Windows no longer shows as the whole file rewritten.**
  Files checked out with Windows line endings were compared with the
  repository's copy byte for byte, so every line differed. The working file is
  now read the way git reads it before a diff, and staging or discarding a
  single hunk touches only that hunk again.
- **The window keeps drawing while Git works.** Reading a repository —
  opening it, its history, a diff, blame, status — used to run on the thread
  that paints the window, so on a large repository the window stopped
  responding until it finished. It now runs beside it. Switching tabs quickly
  leaves the one you clicked last open.
- **No pale strip beside the commit detail.** The graph's column header left a
  band beside the detail card's corner that the card's shadow did not reach.
- **No box inside the box.** The diff on Working copy, Diff and Stash, the
  sides of a conflict and the Farm's columns each drew a second border, corner
  and shadow inside the window's pane.
- **Interactive rebase works on Windows.** Starting a planned rebase failed
  with "there was a problem with the editor" whatever the plan, because the
  helper that hands git the plan could not read the path git gave it.
- **Commit messages in the graph are the size of everything else.** They were a
  step larger than every other list, so the history looked out of place.
- **Scrollbars on Windows are the theme's**: thin and rounded, with no arrow
  buttons and no square corner where two meet. The open repository tab no
  longer has a smudged shadow.
- **No transparent band round the window on Windows.** The application drew
  its own corner and shadow inside the ones Windows 11 already gives it, and
  the desktop showed through the gap between them. The window now fills to its
  edge and the corner and shadow are the system's.
- The test suite passes on Windows. One test compared file paths written with
  `/` against the paths Windows produces with `\`, so it had only ever passed on
  Linux and macOS.
- The core library's tests pass on Windows. Their test repositories picked up
  Git for Windows' line-ending setting, so files came back with different line
  endings than the tests had written.

## [0.8.1] - 2026-09-13

### Fixed

- Scrolling the graph no longer re-spaces the lanes or moves the message
  column: the lane column is sized to the whole history, not to the rows on
  screen.
- A branch label's lead line no longer steps by a pixel where the Branch/Tag
  and Graph columns meet.

## [0.8.0] - 2026-09-13

### Added

- **Hover a commit subject for its whole message.** After a short rest the
  graph shows the full commit message — body, paragraph breaks and trailers —
  without selecting the commit or changing the detail panel. Hovering a commit
  that carries no branch label also names the branch it belongs to, faintly, in
  the Branch/Tag column.
- At its narrowest the Graph column's header shows the graph icon instead of a
  clipped word.

- **Follow Omarchy.** On a machine running Omarchy, Settings → Appearance
  offers to take Spagitty's colours from the desktop's own theme and to keep
  following it: change the desktop theme and the window repaints, graph
  included, without a restart. The desktop's palette is not copied literally —
  a terminal palette's roles and an application's are different requirements,
  and on the theme this was built against the desktop's own secondary text
  colour sits at 2.75:1 against its background, which is not readable. The
  hues are the desktop's; the roles are derived and held to the same contrast
  as the eight built-in palettes. Choosing a family opts back out, and stays
  opted out. Machines without Omarchy see nothing new.
- Appearance can follow the desktop's light and dark setting, and keep
  following it. Choosing Light or Dark still takes over, and still sticks.
- A **compact graph** setting, in Settings → Appearance. The graph column is
  sized for five lanes whatever the history is, so an ordinary two- or
  three-lane repository pays for lanes it does not have; compact narrows the
  lanes and gives an ordinary history about 77 pixels back, straight into the
  commit subjects. Nodes become marks rather than author portraits at that
  size — the picture is still there in the commit detail, where a face has room
  to be a face.

### Fixed

- **Dragging the Graph column narrower folds the graph instead of squeezing
  it.** The lanes used to crowd towards the left as the column narrowed, and at
  certain widths a whole lane snapped sideways. Now the lanes that fit stay
  exactly where they are, and each lane the Commit Message column reaches folds
  onto its edge — line and commits together — until, at the narrowest, the
  whole graph is one lane. Widening puts every lane back. The drag also updates
  once per frame and saves the width once, when you let go, instead of on every
  mouse movement.
- **Author pictures are the same size everywhere, and show up for more
  people.** Commit circles no longer shrink and grow as you scroll between
  simple and busy parts of history, and the author pictures in the Author
  column and the commit detail are the same size as the graph's. For a
  repository on GitHub, an author whose email has no Gravatar now gets their
  GitHub picture, found from one of their commits. A dropped connection or a
  rate limit no longer hides pictures for three months; Spagitty tries again
  later. Pictures cached by earlier versions are fetched once more.
- Spagitty opens in the theme you chose, on the first frame. It used to paint
  the default light palette and then change its mind once the application had
  loaded — a flash on any machine, and several frames on Linux, where the
  release build renders in software. A fresh install on a dark desktop opens
  dark now too.
- The desktop's light/dark preference is no longer read once and written down
  as though you had chosen it. It was: the first launch sampled the system,
  saved the answer as an explicit setting, and from then on a desktop that
  switched to dark in the evening moved everything except Spagitty — with no
  way back short of clearing stored data.
- Switching screens no longer animates for anybody who has asked their machine
  to stop moving things. The stylesheet said it did and could not: the slide is
  driven from JavaScript, writing a new transform every frame, so a CSS rule
  about transition durations had nothing to shorten.
- The toolbar no longer offers Undo and Redo. Neither was built: they had no
  handler, they were not disabled, and clicking either did nothing — while
  taking focus, taking the pointer, and announcing themselves to a screen
  reader as buttons. The recovery Spagitty actually has is the Reflog screen.
- Pull and Fetch show their alternatives. The three ways to pull and the
  choice of which remote to fetch existed only behind a right-click, which
  nobody can see and a keyboard cannot reach at all — so fast-forward-only was
  the only pull anyone working without a mouse could perform. Both are split
  buttons now: the main half does the safe thing, and a caret beside it opens
  the choices, from the keyboard as well as the pointer. Right-clicking still
  works.
- Keyboard shortcuts are written the way the platform writes them. macOS reads
  `⌘F` and everything else `Ctrl+F`, from one place rather than three that
  disagreed — the Appearance section used to say `Ctrl` on a Mac.
- Eleven components stopped painting themselves. The worktrees manager, the
  submodules dialog, Create pull request, Settings → Profiles, the binary and
  image diffs, file history and the status strip all read colour variables that
  do not exist and fell through to fixed dark values, so they rendered the same
  dark dialog on every one of the sixteen palettes — obviously wrong on the
  light ones. They use the theme's own colours now, as does everything they had
  written down as a literal: the near-black scrim behind four modals, the brand
  amber used as "the accent" where every family has its own, Material Design's
  red and yellow where the palette's `danger` and `warn` exist, and a
  transparency checkerboard in two fixed greys that was the darkest thing on a
  light screen.
- The text-size preference reaches those eleven components. They carried 83
  fixed pixel sizes, so raising the text size grew the whole application except
  them — 10px labels in Profiles against a 15.6px default everywhere else.

### Changed

- Settings stops explaining itself. The privacy paragraph under the token field
  was five sentences; it is one line, with the rest on a hover. The licence
  section, the update preference and the empty remotes state got the same
  treatment. Nothing was dropped — every claim about what leaves the machine is
  still there and still tested — it is just no longer in the way.
- macOS gets a macOS window. Spagitty drew the same three neutral glyph buttons
  on the right on every platform, over a frame the system was not allowed to
  draw; on a Mac that is most of the difference between an application and a web
  page in a custom frame. The window is properly decorated there now, with the
  system's own traffic lights at the top left and no second set on the right, no
  card drawn inside the real frame, and the system's resize edges instead of
  eight invisible ones laid over them. Linux and Windows are untouched.
- Controls stop moving while you aim at them. Buttons, chips and the nav rail's
  rows all lifted a pixel or two under the pointer — the rail's slid sideways,
  along the direction you were already travelling — so crossing a row of buttons
  or running down the rail set off a small shift at every step. Hover changes
  colour now; pressing still moves, because a press is something you did.
- The last eight type sizes follow the text-size preference. The title bar, its
  window controls, the toolbar's branch name, the badges and the repositories
  heading were fixed pixel values, so raising the text size grew everything
  except them. One badge label was 9px.
- The window frame takes less of the window. The toolbar is 40 pixels rather
  than 50 — its controls only ever measured 38 — and the nav rail's fourteen
  destinations are four groups instead of one flat list, so supervising the
  farm, doing routine git work and reaching for an occasional tool no longer
  look like the same thing. Nothing moved: the rows are in exactly the order
  they were.
- Side panels open at a width that suits the window they open in. The defaults
  were a 1440-wide window's defaults, and on the 1280 the application actually
  opens at they left 489 pixels for commit subjects on a screen whose whole job
  is reading them. A width you have dragged is still yours, at any window size.
- The macOS downloads are signed. They were not signed at all before, and an
  app with no signature is not what macOS calls an unidentified developer — it
  is what macOS calls **damaged**, with a Move to Bin button and no Open Anyway
  path. Every build now carries at least an ad-hoc signature, so the dialog a
  Mac user meets is the one with a documented way through it. Ad-hoc is not
  notarization and the release notes no longer imply otherwise.
- Every release lane builds both Mac architectures. `main` and the alpha lane
  each built one Apple silicon job, so no published release has ever carried an
  Intel Mac download; the draft lane split them onto a runner image GitHub has
  since closed down. Both now build `arm64` and `x86_64` on supported runners.
- The finished `.dmg` is opened and checked on the build machine before it is
  attached to anything — the container, the app's signature, Gatekeeper's
  verdict and the architecture, each reported separately because they answer
  different questions.
- Releases carry `SHA256SUMS` files. A download that misbehaves can now be
  compared against what was built, which is the first question any report of a
  damaged app has to answer.
- The macOS install instructions no longer tell everybody to remove the
  quarantine attribute. It silences the check rather than repairing the file,
  and as *the* documented step it taught every Mac user to disarm the one thing
  that would have caught a genuinely broken download. It survives in
  `docs/ci.md` as a diagnostic, labelled as a bypass.
- A release published from `main` now refuses to build macOS at all without a
  Developer ID certificate, rather than quietly publishing an unsigned build.
  Until an Apple Developer account exists, the draft and alpha lanes are where
  a Mac download comes from.

## [0.7.0] - 2026-09-07

### Added

- A commit's node on the Graph screen shows the author's real picture, and
  hovering it says who they are — their name, their address, and their account
  handle where the address carries one. Pictures come from the address already
  in the commit: a GitHub no-reply address resolves with no lookup at all, and
  anything else is asked of Gravatar as a hash, never as an address. Each
  author is fetched once and cached on disk, so a second look at a repository
  makes no requests. With no network, no picture, or the preference off, the
  generated face is drawn exactly as before.
- A Behaviour preference — on by default — turns the pictures off. Turning it
  off stops every request and empties the cache.

### Changed

- The Settings screen stops explaining itself. Section subtitles and the
  paragraphs under individual controls are gone; the detail they carried is on
  the control, one hover away. Everything that says what leaves the machine,
  everything a control cannot demonstrate — a keyboard shortcut, a token's
  required scopes, where a value in effect came from — and every admission that
  a switch is not honoured yet has been kept.

### Fixed

- The window no longer draws a card inside the one the desktop already drew it.
  On Linux the compositor supplies the corner, the border and the shadow, and
  Spagitty's own left a transparent margin between the two which Hyprland
  filled with blurred desktop — a band of smeared wallpaper around the
  application. The Linux window is now flush and opaque; macOS and Windows are
  unchanged.
- Development builds really do keep component styles with their components. The
  guard added for this in 0.5.1 asked whether the environment was *not*
  development, and a plain `vite dev` reaches it with nothing set at all — so
  every development run took the production path and could still serve raw
  component source as a stylesheet.

## [0.6.0] - 2026-09-06

### Added

- The Linux AppImage carries zsync update information, and every release
  attaches the matching `.AppImage.zsync`. An AppImage manager — `AppImageUpdate`
  or AppShelf — can now see that a newer Spagitty exists and replace an
  installed build in place. Spagitty still never replaces its own binary.

### Fixed

- A released Spagitty knows which release it is, so the update check can
  actually report a newer one. The tag was baked in only by the draft lane, so
  every published release and every alpha called itself a development build and
  said it was up to date whatever had been released since.
- The update check reads the project's own repository path, rather than a
  differently capitalised spelling of it that GitHub happened to tolerate.

## [0.5.1] - 2026-09-06

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

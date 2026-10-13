<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: the workbench and recipes — Spagitty 3.0.0

**Prepared:** 2026-10-12  
**Target:** 3.0.0  
**Status:** Accepted. Recorded as five backlog items, FEAT-130 to FEAT-134. The author answered the questions in [section 14](#14-the-authors-answers) on 2026-10-13.  
**Raised by:** the author, 2026-10-12: a full terminal on the rail with tmux and pmux shipped in, so nobody leaves Spagitty for one; a file viewer like Neovim with AstroNvim — mouse-clickable, tree-sitter coloured, any branch, blame on any line, and no editing; an investigator that shows a file's, a folder's or a branch's history from birth; and an area where the person builds what they want done with git out of ready-made blocks — stash, pull, merge, pop, commit, push — with a little text, and a selected agent does it, guided by a brief that Spagitty ships. Then, the same day: a settings screen to pin or hide what is on the rail.  
**Screens:** 1T (Terminal, new), 1U (Files, new), 1O (Investigate — File history, rebuilt and put on the rail), 1V (Recipes, new), 1K (Settings › Rail, a new section), chrome (the rail).  
**Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/), listed in [section 13](#13-designs).

## 1. Outcome

Spagitty 3.0 is somewhere a person can stay all day.

- **Terminal (1T)** is a real terminal on the rail. It runs the person's shell inside a multiplexer that ships with Spagitty — tmux on macOS and Linux, psmux on Windows — and it follows the repository that is open.
- **Files (1U)** shows any file at any branch, tag or commit, coloured by tree-sitter, with blame beside every line. It reads git's objects and never the working tree, so it cannot write anything.
- **Investigate (1O)** tells the life of a file, a folder, a branch or a line: when it was born, by whom, how it grew, when it was renamed, moved or rewritten, and who owns it today.
- **Recipes (1V)** turns "stash, pull, merge, pop, commit, push" into a stack of blocks. Spagitty checks the stack before anything runs, saves a restore point, and runs it with an agent at the level the person picks.
- **Settings › Rail** lets the person pin or hide each screen on the rail, because four new rows make the rail long.

## 2. Rules that shape the rest

1. **Nothing new is louder than the work.** Each screen is one rail row and one route. The title bar, the rail, the toolbar and the status strip are unchanged: the artboards redraw them only as context, and where they differ from `TitleBar.svelte`, `NavRail.svelte`, `Toolbar.svelte` and `StatusStrip.svelte`, the components win.
2. **View-only is structural, not a flag.** Files and Investigate read blobs and trees from the object database at a revision. They hold no handle to the working tree, so there is nothing to disable.
3. **An agent acts only through blocks.** In Recipes, the blocks are the whole vocabulary. Free text is limited to a commit message (72 characters) and one note to the agent (140). Everything else is chosen from menus.
4. **Everything a recipe does can be undone in one move.** A restore point is written before the first step, and *Undo the whole run* puts the branches, the stash and the working tree back.
5. **The human path is unchanged.** The toolbar's Pull, Fetch and Push, Merger, Conflicts and the 2.0 agent assignments work exactly as before. A recipe that stops on a conflict hands it to Merger (FEAT-117) rather than growing a second resolver.
6. **Labels only where they are needed.** The author wants screens light on wording. Hover titles carry the detail (TASK-038, TASK-044).

## 3. Words used here

| Word | Means |
| --- | --- |
| **Multiplexer** | tmux, or psmux on Windows: what keeps terminal sessions alive and splits them into panes. |
| **Session** | A multiplexer session. Spagitty keeps one per repository on its own socket. |
| **Revision** | A branch, tag or commit chosen in Files or Investigate. |
| **Peek** | The card that opens under a line in Files: who, when, why, and what the line was before. |
| **Event** | One thing in a life on Investigate: born, renamed, moved, rewrite, edit, merged, deleted. |
| **Block** | One git step in a recipe, from a closed catalogue ([8.2](#82-the-block-catalogue)). |
| **Recipe** | An ordered stack of blocks, with a level, an agent and a note. Saved by name. |
| **Brief** | The markdown the agent is given: Spagitty's `recipes.md` followed by what this recipe generates ([8.5](#85-the-brief)). |
| **Restore point** | Refs under `refs/spagitty/restore/<id>/` written before a run, enough to put everything back. |

## 4. Screens, codes and the rail

| Code | Screen | Route | Rail group | Replaces |
| --- | --- | --- | --- | --- |
| 1T | Terminal | `/terminal` | Repository | — |
| 1U | Files | `/files` | Repository | — |
| 1V | Recipes | `/recipes` | Repository | — |
| 1O | Investigate | `/investigate` | Tools | File history (`/history`, off-rail). `/history` redirects. |

The order in `NAV_ITEMS`: Farm · Graph, Working copy, Conflicts, **Terminal**, **Files**, Merger, Branches, Pull requests, Review, **Recipes** · Rebase, **Investigate**, Log, Stash, Tags, Reflog, Badges, All repositories · Settings. The four new icons are in the artboards' scripts (`terminal`, `files`, `recipes`, `investigate`). `icons.ts` already has a `terminal` glyph, and the artboards' version is close to it: keep the one in `icons.ts`.

## 5. Terminal (FEAT-130)

### 5.1 What runs

- **A PTY per tab**, spawned by `crates/spagitty-process` through `portable-pty` (MIT, from WezTerm). It uses ConPTY on Windows and openpty on macOS and Linux. Bytes go to the webview over a Tauri `Channel`; keystrokes and resizes come back as commands. The process runner's existing kill and cleanup on exit apply.
- **The emulator is xterm.js** (`@xterm/xterm`, MIT) with the fit, WebGL and web-links add-ons. WebGL is what keeps a busy `cargo test` from costing the UI frames. It falls back to the canvas renderer where WebGL is unavailable.
- **The shell**: `$SHELL` on macOS and Linux. On Windows, `pwsh` if found, otherwise Windows PowerShell, otherwise `cmd`. It can be overridden in Settings › Behaviour.

### 5.2 The multiplexer

- **Spagitty uses its own socket**, `tmux -L spagitty`, so it never touches the person's own tmux server. The person's `~/.tmux.conf` still applies, because tmux reads it whatever the socket.
- **One session per repository**, named after the repository folder: `tmux -L spagitty new-session -A -s <name> -c <repo path>`. The windows inside it are the person's. Closing Spagitty detaches; it never kills.
- **What ships**: psmux on Windows (MIT; it reads `.tmux.conf` and answers to `tmux`, `psmux` and `pmux`). On macOS and Linux, Spagitty uses the system tmux if it is at least 3.2. Otherwise it uses a bundled static tmux (ISC, with libevent BSD-3-Clause, ncurses MIT-style and utf8proc MIT), all compatible with GPL-3.0-or-later. Bundled binaries are Tauri `externalBin` sidecars, built or downloaded in CI at pinned versions with checksums, and recorded in `NOTICE` and `THIRD-PARTY-LICENSES.md`.
- **With no multiplexer**, the terminal is a plain PTY per tab and the Sessions panel is hidden. Nothing says a multiplexer is missing.
- **The pill in the header** says which multiplexer is in use and whether it is bundled or the system's. Its hover title gives the path and version.

### 5.3 The screen

- **Header**: a tab per terminal (each tab is a session's window), `+`, the multiplexer pill, *Split right*, *Split down*, *Detach*.
- **Sessions panel**: Spagitty's sessions on its socket, with *Windows* for the selected one. Below them are *Follow repository* and the shell. *Follow repository* means switching repository tabs switches the terminal to that repository's session, creating it if needed. It is on by default.
- **Farm sessions** (second part of FEAT-130, can be split out): when the farm runs a local agent, it may run it inside `tmux -L spagitty` as session `farm/<task>`. Those are listed with the agent's hexagon and state, and open read-only (`attach -r`). The person watches; to type, they take over in Farm.
- **Links**: a 7–40 hex word that `rev-parse` resolves in this repository is underlined and opens it on the Graph (`graph.want`). A `path:line[:col]` that exists at HEAD opens in Files. Both are checked before they are underlined, never guessed.

### 5.4 Keys

- **Opening**: `Ctrl+`` (`Cmd+`` on macOS) opens Terminal from anywhere and focuses it; from Terminal it goes back to the screen before.
- **While the terminal has focus**, every key goes to the shell except `Ctrl+Shift+P` (the palette) and `Ctrl+Shift+`` (leave the terminal). The app's other shortcuts do not fire.
- **Copy and paste**: copy on select is off by default; `Ctrl+Shift+C/V` on Windows and Linux, `Cmd+C/V` on macOS.

## 6. Files (FEAT-131)

### 6.1 Reading

- `files::tree(repo, rev, dir)` lists one directory lazily, and `files::read(repo, rev, path)` returns text or `binary` with a size. Both come from gix objects at `rev`. A Git LFS pointer is shown as a pointer, with its size and object id.
- **Limits**: over 5 MB a file is shown without colour; over 50 MB it is not opened, and the screen says the size. Images use the image viewer from `lib/diff/ImageDiff.svelte` in single mode.
- **The revision picker** is the branch menu (FEAT-122) with Tags and *A commit…* added. A file opened from another screen opens at the revision that screen was showing.

### 6.2 Colour

- **Tree-sitter in Rust**: `tree-sitter-highlight`, with the grammars compiled in: Rust, TypeScript, TSX, JavaScript, Svelte, Python, Go, Java, Kotlin, C, C++, C#, JSON, TOML, YAML, Markdown, CSS, HTML, Bash, SQL, Dockerfile. All are MIT or Apache-2.0. Each grammar adds binary size, so the list is measured before it is final.
- **Span cache**: `highlight::spans(blob_id, language, line_range)` returns token spans for the rows in view, cached by blob id (LRU). A blob never changes, so the cache never has to be invalidated.
- **Fallback**: a language with no grammar uses `lib/diff/highlight.ts`. The token classes map to the theme's syntax tokens (FEAT-064), so Diff and Files read the same.

### 6.3 Blame and the peek

- The **gutter** is FEAT-063's `history::blame(repo, path, rev)`. It is computed after the file has painted, never before. Each blame block shows one row: the author's portrait or initial, name, summary and age. Every row has an age bar.
- **Agent-written lines** use the agent's colour and hexagon. "Agent-written" means a `Co-authored-by` trailer naming an agent, judged by the same rule as the 2.0 attribution check (TASK-067).
- **Clicking a blame block** lights every line from that commit (FEAT-063). Clicking a line opens the **peek** under it, inline, never as a popover over the code. The peek shows:
  - who, when, the pull request number if the forge knows it (FEAT-017), and the commit summary;
  - what the line was before: the removed line that commit's hunk replaced, struck through;
  - *Commit* (the Graph), *Blame before this* (blame again at the commit's parent), *Investigate line* (1O in Line mode).

### 6.4 Mouse and keys

- **Mouse**: every element is clickable: tree, tabs, breadcrumb segments, gutter, line numbers. Shift-click selects a range.
- **Keys**, Neovim-shaped and limited to reading:

| Keys | Action |
| --- | --- |
| `j` `k`, `Ctrl+d` `Ctrl+u`, `gg` `G` | Move and scroll |
| `/` `?` `n` `N` | Search |
| `]b` `[b` | Next or previous blame block |
| `b` | Blame on or off |
| `Enter`, `Esc` | Open or close the peek |
| `y` | Copy `path:line@sha` |

  No key enters a mode that types into the file. There is no go-to-definition, because there is no language server.
- **The status line** reads `VIEW · revision · file · language · tree-sitter · line:col · %`. Tabs persist per repository.

## 7. Investigate (FEAT-132)

### 7.1 Four targets

| Target | Born | Events | Size over time |
| --- | --- | --- | --- |
| **File** | The commit that added it, following renames | Born, renamed (same folder), moved (another folder), rewrite, edit, deleted | Lines after each commit |
| **Folder** | The first commit that added a file under it | Files added, removed and renamed under it, and the biggest edits | Files and lines |
| **Branch** | Its fork point: the merge base with the default branch | Commits, merges in, merged into (where it landed), rebases and resets from the reflog when local | Commits ahead and behind |
| **Line** | The commit that first wrote it (`git log -L`) | Every commit that changed it, with the text then | — |

**Rewrite** means one commit changed at least half of the file's lines as they were then (`REWRITE_SHARE = 0.5`, a named constant).

### 7.2 The screen

- **Header**: the target kind, the target (path, branch or line), *Follow renames* and the range (*All time* by default).
- **The life card** has three parts:
  - one line of facts: born when, by whom and in which commit; how many commits, authors and renames; size today and at its peak;
  - the size chart, with an event marker on each born, renamed, moved and rewrite;
  - one row per author with a tick per commit, agents in the agent's colour.
- **Events**, newest first, each with its kind, author, summary and `+/−`.
- **The selected event**:
  - what share of the file it changed;
  - how many of its lines are still alive, counted from today's blame;
  - who owns the file today, from today's blame grouped by author;
  - the event's largest hunk;
  - *Open file at <sha>* (Files), *Compare with today* (Diff) and *Show in Graph*.

### 7.3 Cost

History is streamed: the first screen paints from the first page of commits, and the rest fills in. A life is cached by target and HEAD. It reads at most 5,000 commits, then *Load more*.

## 8. Recipes (FEAT-133)

### 8.1 The screen

- **Header**: saved recipes as chips, `+`, the branch the recipe runs on, *Copy as script*, *Save*.
- **Blocks** (left): the catalogue by group, each block with its one-key shortcut. A click or its key adds it to the end of the stack.
- **The stack** (middle): the *On Run* row, then each block's parameters as small fields, its git command in mono beside it, *Move up* and *Remove*. The validator's findings appear under the block they are about. The shortcuts are listed under the stack.
- **Agent** (right): the agent menu, the four levels from 2.0, the note (140 characters, with a counter), the brief as it will be sent, and *Run · N steps*. *Run* is disabled while any block has an error, and says how many.
- **Colour** is a small muted square per group, and nothing louder: the author asked for a tool for professionals, not a toy. The blocks are neutral cards.

### 8.2 The block catalogue

Closed. Adding a block is a change to Spagitty, not to a recipe.

| Block | Group | Key | Parameters | Command |
| --- | --- | --- | --- | --- |
| Stash | Save | S | include untracked | `git stash push [-u]` |
| Pop stash | Save | P | which stash (from step N, or latest) | `git stash pop` |
| Commit | Save | C | all or staged; message (≤72) or *agent writes* | `git add -A`, `git commit -m` |
| Amend | Save | A | keep message or new | `git commit --amend [--no-edit]` |
| Fetch | Sync | F | all remotes or one; prune per settings | `git fetch` |
| Pull | Sync | L | remote; fast-forward only, merge or rebase (the toolbar's three) | `git pull` |
| Push | Sync | U | remote; set upstream if missing. **Never force**, and no parameter for it | `git push` |
| Merge | Combine | M | branch; merge commit, fast-forward if possible, or squash | `git merge` |
| Rebase | Combine | R | onto | `git rebase` |
| Cherry-pick | Combine | Y | commits | `git cherry-pick` |
| Switch | Branch | W | branch | `git switch` |
| New branch | Branch | N | name (validated by `check-ref-format`); from here or a revision | `git switch -c` |
| Delete branch | Branch | D | branch; only if merged (`-d`, never `-D`) | `git branch -d` |
| Squash | Shape | Q | last N commits; message or *agent writes* | `git reset --soft HEAD~N`, `git commit` |
| Reword | Shape | E | commit; message or *agent writes* | `git commit --amend` |
| Run checks | Guard | T | one of the repository's configured checks (the 2.0 merge checks) | as configured |
| Stop and ask me | Guard | ? | — | — |
| Stop unless clean | Guard | G | — | `git status --porcelain` |

Branch and remote parameters are chosen from the branch menu (FEAT-122) and the remotes list. They are never typed.

### 8.3 The validator

The validator is `crates/spagitty-core/src/recipes/validate.rs`, one function that runs in two places: on every change to the stack, for the UI, and again at *Run*, against the repository as it is then.

**Errors** (the stack cannot run):
- Pop with no Stash above it.
- Push after Rebase, Squash, Amend or Reword of commits the upstream already has: the push would need force, and force is not available.
- Delete branch of a branch that is checked out at that point, or that another step still uses.
- Any branch parameter that does not exist and is not made by an earlier New branch.
- More than 30 steps; a message over 72 characters; a note over 140.

**Warnings** (the stack can run):
- Stash with no Pop below it ("changes stay stashed").
- Pull, Merge, Rebase or Switch while the working tree has changes and nothing above stashes, commits or guards them.
- Push to a branch on the repository's *never land unattended* list (2.0) at the Unattended level. This is an error at that level.

### 8.4 Running it

This was [question 1](#14-the-authors-answers), and the author chose it on 2026-10-13:

- **Spagitty runs every block's command itself**, through the process runner and the activity lanes of FEAT-121, exactly as the toolbar does. Each command is logged in the command log (FEAT-020).
- **The agent is called only where judgment is needed**:
  - a commit, squash or reword message set to *agent writes*;
  - a conflict, by an assignment in Merger exactly as in 2.0 (FEAT-115, FEAT-117), in its scratch worktree;
  - a failure, where the agent reads the output and `git status` and explains it, without writing.
- **The agent never writes in the person's working copy.** Recipes act on the real working copy, unlike 2.0 assignments, which is why the agent is kept away from it.

The alternative the author first described, where the agent runs every command, needs the agent in the working copy with a shell. Spagitty could put a `git` shim first on its `PATH` that refuses any command outside the brief's *Allowed* list and logs every call. But an agent with a shell can go around a shim. It was not chosen.

**Levels** are the 2.0 four, with the same names:

| Level | Behaviour |
| --- | --- |
| Suggest | Nothing runs. The commands and the brief are shown; *Copy as script* is the way out. |
| Step by step | Stops before every step. |
| Sign off | Runs, and stops before Push and Delete branch. |
| Unattended | Runs through. Refused for a push to a branch on the *never land unattended* list. |

**These always stop**, at any level: a conflict, a non-zero exit, a failed Run checks, a push the remote rejects, and the working tree or a branch changed by someone else mid-run (the watcher sees it).

**The restore point**, written before the first step, is under `refs/spagitty/restore/<id>/`:
- `HEAD`, recorded as the branch name or a detached sha;
- the tip of every branch the recipe names;
- the stash list, as refs;
- the working tree and index, as a commit made with `git stash create`.

*Undo the whole run* resets those branches, restores the working tree and index, and leaves the restore point in place until the person drops it. The newest 20 restore points are kept, and none older than 7 days. They are listed in Reflog (1M) under *Restore points*.

**The run view** shows:
- the steps with their state (done, waiting, not started) and time;
- the agent's card, with the commands Spagitty ran and their output;
- the stop as the one tinted card, with its choices. For a conflict these are *Open in Merger*, *Let <agent> propose* and *Abort merge, keep steps 1–N*;
- the restore point with *Undo the whole run*.

**The record**: each run is recorded like an assignment (FEAT-115 §5.5) and kept in application data. It holds the recipe as it was, every command and its output, every stop and who answered it, and the restore point. Nothing in it is sent anywhere.

### 8.5 The brief

The agent is given two parts, in order:

1. **`recipes.md`**, which ships with Spagitty. It is compiled in with `include_str!` from `crates/spagitty-core/src/recipes/recipes.md` and carries its own version number, shown on the Recipes screen. The proposed text is in [Appendix A](#appendix-a-recipesmd).
2. **This recipe**, generated from the stack:

```markdown
---
recipe: sync-and-ship
repo: spagitty · branch: feature/avatars
agent: claude-code · level: step
restore-point: yes
---
# Do exactly these steps, in order
1. Stash — include untracked
2. Pull from origin — fast-forward only
3. Merge main into feature/avatars — always a merge commit
4. Pop the stash from step 1 — on conflict, stop
5. Commit all changes — message: "Sync with main"
6. Push to origin — never force

## Allowed commands
git stash push -u · git pull --ff-only · git merge --no-ff · git stash pop · git add -A · git commit -m · git push · git status · git log

## Never
- force-push, reset --hard, clean -f
- touch a branch not named above
- edit files, except to resolve a conflict you report

## When a step fails
Stop. Report the step, the command, its exit code and `git status`.

## Note from <person>
<the note, if any>
```

### 8.6 Storage and sharing

- **Recipes are kept** in application data, per repository. They are not under `.spagitty/`, because a file there shows up as a working-copy change.
- ***Copy as script*** writes the commands as a POSIX shell script with `set -e`, or as PowerShell on Windows. Each step is a commented line.
- **Export and import** use the JSON form: `{ "v": 1, "name", "branch", "steps": [{ "block", "params" }], "level", "agent", "note" }`.

## 9. Settings › Rail (FEAT-134)

- **A new chip, Rail**, after Appearance, at `/settings#rail`. The section id `rail` is added to `SECTIONS` and given its own branch in `routes/settings/+page.svelte`, whose comment explains why there is no catch-all.
- **Every screen has a row** in its rail group, showing its icon, label and code, with *Pin* or *Hide*:
  - **Farm and Settings** read *Always*, because Settings must stay reachable to undo a hide.
  - **Conflicts and Badges** keep their own rules (`shows: 'conflicts'` and `'open'`) and show them instead of the choice.
- **Hidden screens go *Under ⋯ More*** (a rail button that opens the `Menu` component, with the hidden screens and their codes) or ***Palette only***. The palette always lists every screen.
- **The screen you are on is always shown**, hidden or not, for as long as you are on it: the same rule `isShown` already applies to `open` rows, so "where am I" always has an answer.
- **Reorder** by dragging the grip or with `Alt+↑/↓`, within a group only.
- **Stored** in Spagitty's settings, not the repository: `rail: { hidden: ScreenCode[], order: ScreenCode[], where: 'more' | 'palette' }`.
- **The default is every screen pinned**, which keeps TASK-056's decision, and *Reset* returns to it. The artboard shows a person who has hidden five screens; that is an example, not the default.
- **The preview** in the section is the real rail component rendered from the draft settings, not a copy.

## 10. Dependencies

Each one is listed with the reason it is needed, as `AGENTS.md` asks.

| Dependency | Licence | For |
| --- | --- | --- |
| `portable-pty` (crate) | MIT | A PTY on all three platforms, ConPTY included |
| `@xterm/xterm` and its fit, WebGL and web-links add-ons | MIT | The terminal emulator |
| tmux (bundled where the system has none ≥ 3.2), with libevent, ncurses and utf8proc | ISC, BSD-3-Clause, MIT-style, MIT | The multiplexer on macOS and Linux |
| psmux | MIT | The multiplexer on Windows |
| `tree-sitter`, `tree-sitter-highlight` and the grammars in [6.2](#62-colour) | MIT / Apache-2.0 | Files' colour |

No new dependency is needed for Investigate, Recipes or the rail.

## 11. Performance

The author has said the UI is a bottleneck in large repositories (the GPUI question). None of the 3.0 screens may make that worse:

- **Terminal**: WebGL renderer; output is batched per animation frame; scrollback is 10,000 lines by default.
- **Files**: virtual rows (`ui/VirtualRows.svelte`); colour only for the rows in view; blame after first paint; no work on screens that are not open.
- **Investigate**: streamed, cached, capped ([7.3](#73-cost)).
- **The rail**: nothing measurable.

## 12. Order of work

1. **FEAT-134 — the rail.** It is small, and the four new rows need it.
2. **FEAT-131 — Files.** It brings tree-sitter and blame at a revision, which FEAT-132 reuses.
3. **FEAT-132 — Investigate.** It replaces `/history`.
4. **FEAT-130 — Terminal.** The bundling work in CI is the long part; farm sessions can follow as their own item.
5. **FEAT-133 — Recipes.**

## 13. Designs

All are in `design_handoff_workbench/source/`, 1440×900 with a dark reference theme (each has a `theme` prop for light).

| Artboard | Item | Shows |
| --- | --- | --- |
| `Main.dc.html` | FEAT-130 | Terminal: tabs, two panes in tmux, sessions with two farm agents, *Follow repository*, links in output |
| `Viewer.dc.html` | FEAT-131 | Files: tree at `main`, tabs, breadcrumb, colour, blame gutter, the peek on an agent-written line, the status line |
| `Investigator.dc.html` | FEAT-132 | Investigate a file: the life card, events, the selected rewrite |
| `Recipes.dc.html` | FEAT-133 | The builder. **Interactive**: blocks add, move and remove, the validator and the brief update live |
| `RecipeRun.dc.html` | FEAT-133 | A run stopped at a merge conflict, with the restore point |
| `SettingsRail.dc.html` | FEAT-134 | Settings › Rail. **Interactive**: Pin and Hide update the preview and the More menu |

**When they disagree:** the spec wins on behaviour and the screens win on layout. Exact sizes and colours come from `metrics.ts` and `app.css`.

## 14. The author's answers

Asked and answered on 2026-10-13.

1. **Who runs a recipe's git commands?** Spagitty, with the agent called for judgment ([8.4](#84-running-it)). Not the agent behind a `git` shim.
2. **"pmux"** means psmux, whose binaries include `pmux`.
3. **Names**: *Recipes*, *Investigate* and *Files* are kept.
4. **The bundled tmux on macOS and Linux** is used only when there is no system tmux at 3.2 or later.
5. **Farm sessions in the terminal** come later, as their own item. The terminal and the multiplexer are in 3.0.
6. **Recipes in the repository**: no. Recipes stay personal, in application data.

## 15. Non-scope

- **Editing anything in Files**, an *Open in editor* button, or language-server features.
- **Recipes**: blocks beyond the catalogue, loops and conditions other than the three guards, recipes that span repositories, and scheduled recipes.
- **Terminal**: a terminal inside other screens (a drawer), and sharing a session with another machine.
- **Investigate**: submodules followed into, and blame through moved code (`-C -C`), which may come later behind a switch.

## Appendix A: `recipes.md`

```markdown
# Spagitty recipes — how to carry one out

You are carrying out a recipe a person built in Spagitty from fixed blocks.
The steps below this guide are the whole of what they want. Follow them
exactly and in order.

## Rules that no recipe changes
- Run only the commands listed under "Allowed commands". If a step seems to
  need anything else, stop and say what and why.
- Never force-push, never `reset --hard`, never `clean -f`, never delete a
  branch with `-D`, never rewrite commits the remote already has.
- Never touch a branch the recipe does not name.
- Do not edit files. The one exception is a conflict you have been asked to
  resolve, and then only the conflicted hunks.
- A restore point exists. You never need to undo anything yourself: stop
  and say what happened.

## Stopping
Stop at once, without trying to recover, when:
- a command exits non-zero;
- git reports a conflict;
- a check fails;
- the remote rejects a push;
- the working tree or a branch changed in a way the recipe did not cause.

When you stop, report the step number, the command, its exit code, the last
lines of its output and `git status --short`, and nothing else.

## Messages
When a step says "agent writes" for a message, write one line of at most 72
characters, in the imperative mood, about what the changes do. Never add
trailers, co-author lines or any mention of yourself or your tool.

## Talking
Be brief. One line per finished step: its number and "done", or what it
printed if that matters. The person reads the commands in Spagitty; do not
repeat them.
```

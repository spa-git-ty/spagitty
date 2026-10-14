<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: issues, hooks, pipelines and releases — Spagitty 3.1.0

**Prepared:** 2026-10-14\
**Target:** 3.1.0\
**Status:** Implementation scope for four forge screens.\
**Screens:** 1W Issues, 1X Hooks (Runs and Hooks), 1Y Pipelines (Preview and Live), 1Z Releases — all new. Chrome: four rail rows. *The codes are proposed; check them against the screen list.*\
**Designs:** [`design_handoff_forge_views/`](../../design_handoff_forge_views/), listed in [section 12](#12-designs).

## 1. Outcome

Spagitty 3.1 shows what happens around the code, not only the code.

- **Issues (1W)** lists the repository's issues from its host and opens one beside the code: thread, labels, milestone, and the branches and pull requests linked to it. From an issue, a branch is one click, and an agent can be handed the work.
- **Hooks (1X)** is where hooks are seen running and built. *Runs* shows each hook of a commit as a step, with its state, its time and its log, live. *Hooks* builds them from ready-made blocks instead of shell scripts.
- **Pipelines (1Y)** draws the CI configuration as a graph. *Preview* reads the YAML in the working copy and answers "if I push this to main, which jobs run?" before the push. *Live* follows the real runs on watched branches and pull requests.
- **Releases (1Z)** lists the host's releases and drafts the next one: tag, title, a changelog generated from commits, and the files to attach.

## 2. Rules that shape the rest

1. **Nothing new is louder than the work.** Each screen is one rail row and one route. The title bar, the rail, the toolbar and the status strip in the artboards are context; where they differ from `TitleBar.svelte`, `NavRail.svelte`, `Toolbar.svelte` and `StatusStrip.svelte`, the components win.
2. **`forge/http.rs` stays the only place a request is made.** Every call in this proposal goes through it, to a host the person connected, with the token they gave (`forge/keychain.rs`). No new HTTP client, no tokio: `ureq` is blocking and the workers are threads.
3. **The UI's vocabulary never names a host.** `Issue`, `Release`, `Run`, `Job` are Spagitty's shapes; `forge/github.rs` and `forge/gitlab.rs` map each host's JSON onto them, as `PullRequest` does today. A host that cannot do something (GitLab has no draft releases) shows that as a fact on the screen, not as a missing button with no reason.
4. **Writes to a host are a decision, not a default.** `forge.rs` records that the forge layer was read-only by decision; `forge/review.rs` later added the review verdict and merge. Issues, Pipelines and Releases each want writes (comment, close, cancel, re-run, publish, delete). Each write is its own action with its own confirmation and failure message, with scope defined in the implementation decisions ([13](#13-implementation-decisions), question 1).
5. **A preview never guesses.** Where Pipelines cannot decide something locally (an expression it does not evaluate, a remote include), the job is drawn as *decided on the host*, never as runs or skipped.
6. **Hooks work without Spagitty.** A commit made in a terminal runs the same hooks and leaves the same run record. Spagitty reads the record; it is not in the commit's path.
7. **Labels only where they are needed.** The screens use short labels; hover titles carry detail.

## 3. Words used here

| Word | Means |
| --- | --- |
| **Host** | GitHub or GitLab (`forge::Kind`). Bitbucket is a `Kind` already, for pull requests only. |
| **Block** | One hook from the closed catalogue in [6.3](#63-the-block-catalogue), with its settings. |
| **Stage** | The git moment a hook runs at: `pre-commit`, `commit-msg`, `pre-push`. |
| **Run record** | The JSON-lines file a hook run writes as it goes, read by *Runs* ([6.2](#62-the-run-record)). |
| **Workflow** | One CI file: a GitHub Actions workflow, or `.gitlab-ci.yml` with its local includes. |
| **Simulation** | The event, branch and changed files Preview evaluates a workflow against. |
| **Watched** | A branch or request whose runs Live follows. |

## 4. Screens, codes and the rail

| Code | Screen | Route | Item |
| --- | --- | --- | --- |
| 1W | Issues | `/issues` | FEAT-136 |
| 1X | Hooks | `/hooks` (`/hooks/runs`, `/hooks/edit`) | FEAT-137 |
| 1Y | Pipelines | `/pipelines` (`/pipelines/preview`, `/pipelines/live`) | FEAT-138 |
| 1Z | Releases | `/releases` | FEAT-139 |

The artboards put the four rows in a group of their own, after Recipes and before Investigate. That is a new group in `NAV_ITEMS`, and FEAT-134 lets people reorder only within a group, so the group is the author's call ([13](#13-implementation-decisions), question 2). Icons for `issues`, `hooks`, `pipelines` and `releases` are in the artboards' scripts; draw them into `icons.ts` at the same stroke as the others. All four rows are pinned by default (TASK-056) and can be hidden with FEAT-134.

*Settings › Hooks* (FEAT-107) stays where it is for the per-repository switch; it gains a link to the Hooks screen.

## 5. Issues (FEAT-136)

### 5.1 Reading

- **New module `forge/issues.rs`**, with `Issue`, `IssueComment` and `IssueEvent` beside `PullRequest`.
- **GitHub:** `GET /repos/{owner}/{repo}/issues` (it returns pull requests too: drop entries that have a `pull_request` key), `/issues/{n}/comments`, and `/issues/{n}/timeline` for linked branches, pull requests and commits that mention it.
- **GitLab:** `GET /projects/:id/issues`, `/issues/:iid/notes` (system notes become events, the rest comments), and `/issues/:iid/related_merge_requests`.
- **Paging and cost:** read the first page of 50 on open; more as the list scrolls. Use conditional requests (`ETag` / `If-None-Match`); a 304 on GitHub does not count against the rate limit. Cache the last answer per repository in application data so the screen paints before the network answers.
- **Refresh** on the timer `forge/watch.rs` already runs, not a second timer.

### 5.2 The screen

- **Header:** the host switch (only hosts this repository has remotes on), the filter box, *New issue*.
- **Filters:** *Open* / *Closed* with counts, *Mine*, *Label*, *Milestone*, *Sort*. The box takes `is:open label:bug assignee:@me` style terms; Spagitty parses them and sends each host its own query parameters.
- **List:** state, title, labels in their host colours (checked against the background like `--accent`), number, author, age, replies, and a branch mark when a branch or request is linked.
- **Issue:** title, state, thread rendered with the reading face (FEAT-087), timeline events, details column (assignees, labels, milestone progress, development, mentioned in).
- **Markdown** is rendered with the renderer the Review screen already uses. Images from the host load only after the person opens the issue.

### 5.3 Actions

- **Start branch** (local, no write to the host): creates `fix/<n>-<slug>` or `feat/<n>-<slug>` from the default branch, by the label, through the existing branch code, and checks it out. The name is editable before it is made.
- **Hand to agent:** opens the 2.0 assignment flow with the issue's title, body and comments as the task. The FEAT-115 rules hold: scratch worktree, the person's working copy is not touched.
- **Comment, Close, Close with comment, New issue:** host writes ([13](#13-implementation-decisions), question 1). Hidden, not disabled, when the token cannot write.

### 5.4 Other hosts

Bitbucket and "iceberg" are not in 3.1. The screen shows *Bitbucket · soon* only to say so. A host is a module and an arm of `Kind`, nothing more ([13](#13-implementation-decisions), question 3).

## 6. Hooks (FEAT-137)

FEAT-107 already lists the hooks (`hooks.rs`), knows the managers (git, Husky, lefthook, pre-commit), keeps the per-clone switch `spagitty.hooks`, and shows a commit's hook output. This item adds the run view and the builder.

### 6.1 Who runs a hook

Spagitty writes the hook. The file at `git rev-parse --git-path hooks`/`<stage>` is a POSIX `sh` script generated from the blocks: Git for Windows ships `sh`, and git runs hooks through it there. The script runs each block in order and needs nothing from Spagitty, so a commit from a terminal behaves the same.

- **Path conditions use git, not shell globs:** `git diff --cached --quiet -- ':(glob)services/payments/**'` exits 1 when a staged file matches. This is exact `**` matching, the same on every platform.
- **The staged list** comes from `git diff --cached --name-only -z --diff-filter=ACMR`.
- **The script is marked** with a first-line comment (`# spagitty-hooks v1`) and a hash of its blocks. Spagitty never overwrites a hook file without that mark: a hand-written hook or a manager's stub is left alone ([6.5](#65-managers-already-in-the-repository)).

### 6.2 The run record

Every run appends to `.git/spagitty/hooks/runs/<timestamp>-<stage>.jsonl` (under `git rev-parse --git-common-dir`, so worktrees share it):

```json
{"t":"run","stage":"pre-commit","subject":"feat(refunds): partial refund endpoint","at":"…"}
{"t":"start","id":"tests-payments","at":"…"}
{"t":"match","id":"tests-payments","files":3}
{"t":"end","id":"tests-payments","exit":0,"ms":41230}
{"t":"skip","id":"tests-checkout","why":"no-match"}
{"t":"restaged","id":"format","files":["…/RefundService.java"]}
{"t":"done","exit":0}
```

Each block's output goes to `<run>/<block-id>.log`. *Runs* watches the folder (the file watcher the repository already has) and tails the open run. The last 50 runs are kept; older ones are deleted by the script itself.

`pre-commit` has no commit message yet; the subject shown is filled in when `commit-msg` runs, or left empty.

### 6.3 The block catalogue

| Block | Settings | Generated as |
| --- | --- | --- |
| **Run tests when paths change** | paths (pathspec globs), command, folder, timeout, on failure (block / warn), skip when (only `*.md`, WIP subject) | path check, then the command with `timeout` |
| **Format & lint** | tool preset (spotless, prettier, eslint, ruff, gofmt, rustfmt, custom), files glob, *fix and restage* on/off | the command on the staged files that match, then `git add` on those files |
| **Commit message rules** | style (Conventional Commits / ticket prefix / custom regex), allowed types, scope required, subject max, ticket pattern in footer | `commit-msg` checks with `grep -E` |
| **Run a script** | path or command, stage | the command |
| **Block secrets** | built-in patterns + extra ones | `git diff --cached -U0` piped to `grep -E` |
| **Large files** | size limit | `git diff --cached --name-only` + size check |
| **Protect branches** | branch names | `pre-commit` / `pre-push` refuse on those branches |

**Restaging is careful.** A file that is both staged and changed in the working tree is not restaged: formatting it would stage the unstaged part too. The block reports it (*not restaged: has unstaged changes*) and fails if *on failure* is *block*. This is the trap lint-staged handles with a stash; doing the same here is out of scope for 3.1.

**One implementation of the message rules.** The *Try a message* box in the builder runs the generated `commit-msg` script on a temporary file. The rules are not written twice, once in Rust and once in shell.

### 6.4 Storage

- **Shared:** `.spagitty/hooks.toml` in the repository when *Share with repo* is on, so the team gets the same hooks; *Save & install* writes the hook files from it. A clone that has the toml but no installed hooks shows a one-line offer to install, never installs by itself.
- **Personal:** the same file under application data, per repository, when the switch is off.
- **Dry run on staged** runs the generated script against the current index without committing, and shows it in *Runs*.

### 6.5 Managers already in the repository

When Husky, lefthook or pre-commit owns `core.hooksPath`, Spagitty does not take the hooks over silently. The Hooks screen says who owns them and offers:

- **Import:** read `.pre-commit-config.yaml` or `lefthook.yml` and map known hooks onto blocks (`run` / `entry` lines that are not recognised become *Run a script*). The original file is left in place.
- **Leave it:** the builder is read-only and *Runs* still shows runs made through Spagitty's commit, from the output FEAT-107 already captures, as one step per manager command.

Which of these is the default is [13](#13-implementation-decisions), question 4.

## 7. Pipelines (FEAT-138)

### 7.1 Preview: reading the YAML

- **Files:** `.github/workflows/*.yml|yaml`; `.gitlab-ci.yml` and its `include: local:` files. Read from the working copy, so unsaved edits are previewed. `include: project/remote/template` and remote reusable workflows (`uses: org/repo/.github/workflows/x.yml@ref`) are drawn as one opaque node, not fetched.
- **Model (`crates/spagitty-core/src/pipelines/`):** `Workflow { triggers, jobs }`, `Job { id, name, stage, needs, condition, matrix, environment, services, steps, runs_on, source_line }`. Both hosts map onto it.
- **GitHub:** `on:` push / pull_request with `branches`, `branches-ignore`, `tags`, `paths`, `paths-ignore`; `jobs.<id>.needs`; `strategy.matrix` with `include` / `exclude` expanded; `if:` evaluated for a subset — `github.ref`, `github.ref_name`, `github.event_name`, `github.base_ref`, `startsWith`, `endsWith`, `contains`, `==`, `!=`, `&&`, `||`, `!`, `success()`, `always()`. Anything else is *decided on the host*. Local reusable workflows (`uses: ./.github/workflows/x.yml`) are expanded one level.
- **GitLab:** `stages`, `needs`, `rules:` (`if` with `$CI_COMMIT_BRANCH`, `$CI_COMMIT_TAG`, `$CI_PIPELINE_SOURCE`, `$CI_DEFAULT_BRANCH`, `=~`; `changes`), `only` / `except` (the same subset), `extends`, `default:`, `workflow: rules`, `parallel: matrix`, `when: manual`. YAML anchors and `!reference` are resolved by the parser and a pass after it.
- **Simulation:** event (push, pull request, tag, schedule, manual), branch or tag, and changed files — *working copy* (the diff against the branch's upstream) or a commit range. Each job ends up *runs*, *waits for approval* (`environment` with reviewers is unknown locally: GitHub `environment:` and GitLab `when: manual` are drawn as *waits*), *skipped* with the reason, or *decided on the host*.
- **Problems:** the YAML parser's errors, plus a schema check against SchemaStore's GitHub Actions and GitLab CI schemas, bundled (licence to be checked per `AGENTS.md`). Unknown keys get a nearest-match hint (`need` → `needs`). GitLab's own lint (`POST /projects/:id/ci/lint`) is a button, because it is a network call.

### 7.2 Preview: the graph

- **Needs** layout: columns by longest path through `needs`; jobs with no `needs` in the first column (or, on GitLab, their stage's column). **Stages** layout: one column per GitLab stage, or per depth on GitHub.
- **Edges** go from a job's right side to a dependent's left side; a dashed edge leads to a job that is skipped. Matrix jobs show as one card with `×N`, expandable.
- **Large workflows:** cards are placed once and drawn with the same virtual approach as the graph screen; over 200 jobs the graph opens collapsed to stages.
- **Side panel:** the selected job's runner, matrix, services, timeout, secrets used (names only), steps, the line in the file, and *why* it runs or is skipped in this simulation.
- **Run this job locally** is in the artboard; it would need a runner such as `act` and a container engine. [13](#13-implementation-decisions), question 6.

### 7.3 Live

- **GitHub:** `GET /repos/{o}/{r}/actions/runs?branch=…` and `?event=pull_request`, `/actions/runs/{id}/jobs` (jobs and their steps with status and times), `/actions/jobs/{id}/logs` for the log.
- **GitLab:** `GET /projects/:id/pipelines?ref=…`, `/pipelines/:id/jobs`, `/jobs/:id/trace` (it returns the partial log while the job runs, so GitLab logs can stream).
- **GitHub logs while a job runs:** check against the current API before promising streaming. If the REST log is only complete after the job ends, Live shows the running job's steps and their states, and the log when the job finishes. The artboard's *streaming* line is then GitLab-only.
- **No webhooks.** A desktop application cannot receive them. Live polls with conditional requests: every 5 s for a run in progress while the screen is open, every 60 s for watched branches otherwise, and stops when the window is hidden. The status strip says *updated N s ago*, never *live* when it is not.
- **Watched:** the default branch, plus any branches the person adds (*Watch a branch…*), plus the person's open requests (from `forge/watch.rs`). Stored per repository in application data.
- **Run strip:** the last 12 runs on the selected branch, bar height by duration, colour by result.
- **Cancel, Re-run failed:** host writes ([13](#13-implementation-decisions), question 1). Tokens need Actions read (and write for these) on GitHub, `read_api` (or `api` for writes) on GitLab. A missing scope is said in words, with the scope's name.

## 8. Releases (FEAT-139)

### 8.1 Reading

- **GitHub:** `GET /repos/{o}/{r}/releases` (drafts are included for people with push access), assets with `download_count`.
- **GitLab:** `GET /projects/:id/releases`, assets as links (`assets.links`) and sources. GitLab has no draft releases and no download counts on links; the screen does not show a count where there is none.

### 8.2 The screen

- **List:** tag, state (*Draft*, *Latest*, *Pre-release*), age, author, downloads, a `⋯` menu (Open on host, Copy link, Hide, Delete). Filters: *All*, *Drafts*, *Pre-releases*. *Show hidden* switch.
- **Release:** tag (existing, or new from a branch and commit), *Since* (the previous tag, for the changelog), title, *Pre-release*, *Latest*, *Write to CHANGELOG.md*. Changelog with *Write* / *Preview*. Files column with size, copy-link and remove, a drop zone, and *From pipeline*.

### 8.3 The changelog

- **Generate from commits:** the commits in `since..target`, grouped by Conventional Commits type (*New* = feat, *Fixed* = fix, *Changed* = refactor and perf, the rest under *Other* or left out), each line with its request number when the commit came from one. This uses the same parser as the commit-message block ([6.3](#63-the-block-catalogue)).
- On GitHub, the host's own `POST /releases/generate-notes` is offered as a second source.
- **Write to CHANGELOG.md** makes an ordinary commit through Spagitty's commit path (hooks run) on the target branch *before* the tag is made, so the tag contains it. If the branch has moved on the host meanwhile, Spagitty stops and says so.

### 8.4 Writing

All host writes ([13](#13-implementation-decisions), question 1):

- **GitHub:** create (`draft: true`), update, publish (`draft: false`), delete; assets uploaded to the release's `upload_url` (`uploads.github.com`), deleted by id. Publishing with a new tag lets GitHub create the tag at `target_commitish`.
- **GitLab:** no draft on the host, so a GitLab draft lives in Spagitty's application data until *Publish*, which creates the tag (if new) and the release in one call. Files are uploaded to the project's generic package registry and linked as release assets.
- **Hide** is local: a list of release ids per repository in application data. Nothing is sent. Hidden releases are still on the host; the screen says so on hover.
- **Delete** deletes the release, not the tag; deleting the tag too is a separate, unchecked-by-default box in the confirmation.
- **From pipeline:** lists the artifacts of a finished run on the target commit, downloads the chosen ones and uploads them. Progress per file; a failed upload can be retried alone.

## 9. Dependencies

Each with its reason, as `AGENTS.md` asks. Check every licence before adding.

| Dependency | For |
| --- | --- |
| A YAML parser that keeps line numbers and anchors (`saphyr` or `yaml-rust2` are candidates; MIT / Apache-2.0 to be confirmed) | Pipelines Preview |
| SchemaStore's GitHub Actions and GitLab CI JSON schemas (bundled data; licence to be confirmed) | Pipelines problems |
| A JSON-schema validator crate (to be chosen) | the same |

No new dependency for Issues, Hooks or Releases: `ureq`, `serde_json` and the existing markdown renderer cover them.

## 10. Performance

- **Issues:** cached list paints first; virtual rows (`ui/VirtualRows.svelte`); one request per page.
- **Hooks:** the run record is appended, never rewritten; *Runs* reads only the open run.
- **Pipelines:** parsing happens on a worker thread and only when a workflow file changes; the graph places cards once; Live polls only what is on screen, and nothing when the window is hidden.
- **Releases:** assets are listed, not downloaded, until asked.

## 11. Order of work

1. **FEAT-137 — Hooks.** No network, builds directly on FEAT-107, and the commit-message parser it brings is reused by Releases.
2. **FEAT-136 — Issues.** Read path first, writes after question 1.
3. **FEAT-139 — Releases.**
4. **FEAT-138 — Pipelines.** The largest: Preview first (local only), Live second.

The rail rows land with the first item that ships each screen.

## 12. Designs

All in `design_handoff_forge_views/source/`, 1440×900, dark reference theme (each has a `theme` prop for light). PNGs in `screens/`.

| Artboard | Item | Shows |
| --- | --- | --- |
| `Main.dc.html` | FEAT-136 | Issues: host switch, filters, list, issue #214 with thread, timeline, details, Start branch / Hand to agent |
| `HooksRun.dc.html` | FEAT-137 | Runs: a pre-commit run at step 4 of 6, the live log of *Tests · payments*, the restage notice, staged files, earlier runs |
| `HooksEdit.dc.html` | FEAT-137 | Hooks builder. **Interactive**: pick *Tests · payments* or *Commit message rules* on the left to switch the editor |
| `Pipeline.dc.html` | FEAT-138 | Preview of `ci.yml` for a push to main with the working copy: graph, skipped and gated jobs, a problem in `release.yml`, the job panel |
| `PipelineLive.dc.html` | FEAT-138 | Live: watched branches and requests, run #1873 on main with job states, run strip, the running job's log |
| `Releases.dc.html` | FEAT-139 | Releases: list with states and downloads, draft v3.1.0 with generated changelog and six files |

**When they disagree:** this proposal wins on behaviour, the screens win on layout. Exact sizes and colours come from `metrics.ts`, `app.css` and `themes.ts`. The data in the artboards is made up.

## 13. Implementation decisions

1. Host writes use individual confirmations.
2. The four screens form a group immediately above Settings.
3. Additional hosts are outside this release.
4. Existing hook managers retain control until explicit Import.
5. Shared hooks use `.spagitty/hooks.toml`; personal hooks use application data.
6. Local pipeline execution is outside this release.
7. Hidden releases are local to this machine.

## 14. Non-scope

- Bitbucket and other hosts for Issues, Pipelines and Releases.
- Editing workflow YAML in Pipelines (Files stays read-only; the person edits in their editor).
- Pipelines for Jenkins, CircleCI, Azure Pipelines and other CI systems.
- Lint-staged-style partial staging for *Format & lint* ([6.3](#63-the-block-catalogue)).
- Notifications for issues; FEAT-114 covers requests only.

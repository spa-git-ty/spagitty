<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Handoff: Review — a place to review pull requests that a dyslexic reviewer can read

## Why

Raised by the author, 2026-10-04: reviewing MRs and PRs on GitHub or GitLab is
cluttered, it hurts to read as a dyslexic reviewer, and major review points get
missed. The Pull requests screen (FEAT-010, FEAT-017, FEAT-071) is good for
browsing, creating and merging, but not enough for reviewing.

The decision: a **new Review screen** on the rail, right after Pull requests. It
holds an inbox and a review room. Pull requests stays as it is.

## About the design files

They are Design Components (`.dc.html`), the same format as
`design_handoff_gitlord/`. Treat the markup as a layout spec and the sample data
as illustration. Throw away the DC runtime and build with the repo's own
components (`Btn`, `Chip`, `Icon`, `Splitter`, `.card`, `.ornament`) and tokens.
The sample PR (#214, avatars on disk) is made up.

| File | What it is |
| --- | --- |
| `Review Room.dc.html` | The main screen, and it is interactive: click files, ticks, the switches in the bottom pill, line numbers and the `+` buttons. |
| `Review Inbox.dc.html` | The landing view: PRs grouped by what they need from you, plus a preview card. |
| `Settings Reading.dc.html` | The new **Reading** section in Settings, with a live preview. |

**Fidelity: high.** These follow the spatial shell (FEAT-082) and Pomodoro
(FEAT-086). Keep the structure, the copy, and the meaning of every colour. Exact
pixel values come from `metrics.ts` and `app.css`, not from these files.

## Where it sits in the shell

- **Rail:** a new `Review` item after `Pull requests` in the `work` group. It
  needs a new icon (an eye, drawn in `icons.ts` style) and a dot when a PR is
  waiting on you.
- **Pane:** the screen header has no fill (`--chrome-veil` is transparent in the
  pane). The right column is an **inset card**, like the commit detail on Graph.
- **Review controls are an ornament.** A second glass pill floats at the bottom
  of the diff column. It holds previous/next file and position, `Changes | Whole
  file`, `One | All`, the focus ruler, the reading font (`Aa`), and
  `Viewed, next`. The global toolbar under the pane is unchanged. *Open question
  for the author:* keep two pills, or swap the toolbar's contents while on
  Review.
- **Performance:** WebKitGTK paints in software, so the pill is the only new
  blurred surface. Never blur a diff row. In `All` mode, virtualise the rows.

## The review room: behaviour

1. **Touched files** (left). Each file shows:
   - a `viewed` tick, a `+/−` count, a `conflict fix` marker and an open-thread
     count;
   - filter chips: `All` · `Author` · `Conflict fixes`;
   - viewed state persisted per PR **and per head SHA**, so files the author
     changes after you viewed them come back unticked.
2. **Author vs conflict fix.** Code written while resolving a merge conflict
   inside the PR is a different origin from the author's own commits:
   - it is drawn in a card framed with `--lane-5` (sky);
   - its header names the merge and says what each side brought;
   - `main's side` / `branch's side` show the two parents' versions.
3. **Changes vs whole file.** `Changes` shows the changed parts, with folds
   (`N unchanged lines`) that expand one at a time. `Whole file` shows the
   entire file with the changes in place.
4. **One file vs all files.** `One` pages through files, and `Viewed, next`
   ticks the file and jumps to the next unviewed one. `All` is one scrolling
   column with a header per file.
5. **Comments.** Every line has a `+`; shift-click a line number to cover a
   range. New comments are **pending** (warn-tinted) until `Finish review`,
   which submits them all with Approve, Request changes or Comment. Threads show
   inline with Reply and Resolve/Reopen. The right card lists threads as Open or
   Resolved; clicking one jumps to its line. At the bottom of the card is a
   comment box for the PR as a whole.
6. **Open in worktree.** Puts the PR head in a separate worktree (reuse
   `worktrees.rs`), so you can build and run it without touching your branch.
7. **Reading aids** (the defaults come from Settings › Reading):
   - **Focus ruler:** a warm band under the line you're on, with other chunks
     dimmed. `j`/`k` move it; clicking a line number sets it.
   - **Calm colours:** thin side markers and faint tints instead of full
     red/green rows. Removed lines are muted, not struck through.
   - **Word highlight:** the exact changed words get a soft highlight.
   - **Reading font:** `Aa` toggles the reading font and spacing for code and
     comments.

## Settings › Reading

The section is built like Appearance: note labels, chip rows and sliders.

- **Code font:** Atkinson Hyperlegible Mono, OpenDyslexic Mono, Lexend,
  JetBrains Mono, System.
- **Interface font:** System, Atkinson Hyperlegible, Lexend.
- **Sliders:** code size, line spacing, letter spacing.
- **Focus ruler:** Off / One line / Whole chunk.
- **Diff colours:** Calm / Classic, plus a `Highlight changed words` toggle.

Publish these as tokens (`--code-font`, `--code-lh`, `--code-ls`, and
`--fs-code` already exists) and apply them on Diff, Working copy and History
too, not only Review.

**Fonts:** bundle them under `assets/`, never from a CDN, because the app works
offline. All four are SIL OFL 1.1. Record them in `NOTICE`, and say why in the
handoff (AGENTS.md: no dependency without a reason).

## What the code has today (read on 2026-10-04)

| Need | State |
| --- | --- |
| PR list, "needs you" | ✅ `requests` store, `needingYou` / `waitingOnOthers` |
| Files and diffs | ⚠️ from the host's patch only (`review.rs::pull_request_files` → `parse_patch`), so there is no full file content |
| Whole file | ❌ needs local blobs: fetch `refs/pull/N/head` (GitHub) or `refs/merge-requests/N/head` (GitLab), then diff locally with `diff.rs` |
| Conflict-fix origin | ❌ nothing. For each two-parent merge in `base..head`, `git show --remerge-diff <merge>` (Git ≥ 2.36) gives exactly the resolution; tag the final diff's chunks that overlap it |
| Single-line comments, replies | ✅ GitHub only (`reply_comment`, `submit_review_with_comments`) |
| Line ranges | ❌ `DraftComment` has one `line` and no `start_line` |
| Resolve thread | ❌ `read_comments` hard-codes `resolved: false`, and `resolveComment` is local only. Needs GraphQL `resolveReviewThread` (GitHub) and discussion resolve (GitLab) |
| Comments on the whole PR | ❌ not read or written |
| Pending drafts | ⚠️ kept in `localStorage`; move to app storage and key them by PR **and** head SHA |
| GitLab review | ❌ `gitlab.rs` only lists and creates MRs; every review call builds a GitHub `/pulls` URL |
| Viewed ticks | ❌ none |
| Reading settings | ❌ only text size and zoom |

## Suggested slices

Each slice is one work item under Amendment 12 (an item, a plan, testing
documents and a branch). Assign the next free `FEAT-###` numbers when you start
each one, not before: the record test refuses identifiers that point at nothing.

1. **Review screen and inbox.** A route, a rail item, the inbox built from the
   existing store, and the preview card. Frontend only.
2. **Local PR checkout and local diff.** Fetch the PR head ref, diff locally,
   read whole-file blobs, and add `Open in worktree`.
3. **The review room, read-only.** Files with viewed ticks (per head SHA),
   `Changes`/`Whole file`, `One`/`All`, folds, the review pill, focus ruler,
   calm colours and word highlight.
4. **Conflict-fix origin.** Remerge-diff on the backend, tagging the hunks, the
   blue card, and the "each side" views.
5. **Threads done properly.** Line ranges, real thread grouping,
   Resolve/Reopen against the host, comments on the whole PR, and drafts moved
   out of `localStorage`.
6. **GitLab parity** for slices 2 to 5.
7. **Settings › Reading.** The bundled fonts, the tokens, and applying them
   app-wide.

Dependencies: 1 can start now. 3 needs 2, and 4 needs 2. 7 can run in parallel
with any of them.

## Acceptance criteria (whole feature)

- From the Review inbox, opening a PR goes straight to the first unviewed file.
- Every file can be read as its changed parts or as the whole file, one at a
  time or all scrolling.
- In a PR that merged `main` and resolved a conflict, the resolution is marked
  as a conflict fix and is never shown as the author's own work.
- A pending comment survives a restart and is sent only by `Finish review`.
- Resolving a thread in Spagitty resolves it on the host, and the reverse shows
  after a refresh.
- With Calm colours on, no diff row is a solid red or green block.
- Reading settings change Diff, Working copy, History and Review together.
- It works on GitHub and GitLab.

## Non-scope

- Splitting the review across reviewers, and assigning reviewers.
- AI summaries of the PR. The Farm may come later, separately.
- Changing the Pull requests screen.

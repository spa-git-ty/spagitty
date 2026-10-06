<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Handoff: Merger — merge any two branches, and see the result first

## Why

Raised by the author, 2026-10-06: merging today is a graph action (drag a
branch onto another, `ops::integrate`). That only merges *into the checked-out
branch*. It doesn't say what the result will be before it happens, and conflicts
only appear once Git has already stopped. The author wants one easy, attractive
place to:

- take any two branches, A and B, and merge B into A, A into B, or both into a
  new branch;
- see plainly what the result will be **before anything is written**;
- know in advance whether there will be conflicts, and where;
- resolve every conflict with every option: take A, take B, both (either
  order), pick single lines from each side, or edit by hand.

The decision is a **new Merger screen** on the rail, following the spatial
shell (FEAT-082).

## About the screens

The design is given as screenshots in `screens/`, rendered at 1440 px wide and
2× density, dark theme unless named. Build it with the repo's own components
(`Btn`, `Chip`, `Icon`, `Splitter`, `.card`, `.ornament`) and tokens. The
branches, commits and code shown are made up.

| Screen | What it shows |
| --- | --- |
| `01-plan-dark.png` | The plan: B into A, merge commit. The stage, direction switch, strategy, history preview, file list. |
| `02-plan-new-branch-rebase.png` | Into a new branch (name field), rebase strategy and its history preview. |
| `03-plan-into-B-squash.png` | Direction swapped (A into B), squash. Roles, arrows and colours follow. |
| `04-resolve-overview.png` | Resolving: Conflict 1 took A, Conflict 2 still unresolved (the "Choose what lands here" box), the files list with dots, the bottom pill. |
| `05-resolve-edit-by-hand.png` | Conflict 2 in Edit by hand, the result as a text box (purple). |
| `06-resolve-pick-lines.png` | Pick lines: checkboxes on both sides; unticked lines fade; each result line carries its A/B badge. |
| `07-resolve-both-with-base.png` | Both, A first, with the Base strip turned on. |
| `08-commit-dialog.png` | Everything resolved: the commit dialog listing each choice, with the message. |
| `09-done.png` | After committing. |
| `10-plan-light.png`, `11-resolve-light.png` | The same in the light theme. |

**Fidelity: high** for structure, copy and the meaning of every colour. Exact
pixel values come from `metrics.ts` and `app.css`, not from the screenshots. The
scrollbars and syntax colours in them already follow `app.css` (the scrollbar
block and the FEAT-064 `.tok-*` classes).

> **Do not copy the rail from the screenshots.** It was taken from
> `design_handoff_review/`, which predates TASK-045. The rail is specified in
> [The rail](#the-rail) below; `src/lib/nav.ts` is the authority.

## Where it sits in the shell

- **Route:** `/merge`. Screen code: the next free one (`1R`, unless Review has
  taken it). Record it in `docs/screens.md` in the same change, as every screen
  does.
- **Pane:** the header has no fill, like Review. The plan's three stage cards and
  the resolving columns are content in the pane, not floating layers.
- **Ornaments:** while resolving, a second glass pill floats at the bottom of the
  conflict column (previous/next conflict, Base, take A/B, Next unresolved),
  exactly as Review's pill does. The global toolbar is unchanged.
- **Performance:** as with Review, the pill is the only new blurred surface.
  Never blur a code row.

## The rail

What the author asked for, checked against `NAV_ITEMS` on 2026-10-06:

| Entry | State in `nav.ts` | What to do |
| --- | --- | --- |
| **Merger** | missing | **Add.** `{ label: 'Merger', href: '/merge', group: 'work', icon: 'merge' }`, right after Conflicts and before Branches, always shown. New icon in `icons.ts` style: `M6 3v3a6 6 0 0 0 6 6 6 6 0 0 0 6-6V3`, `M12 12v9`, `M9 18l3 3 3-3` (two lanes meeting, flowing down). Show a red dot while a merge started here has unresolved conflicts. |
| **Review** | missing | **Add** if the Review slice 1 (`design_handoff_review/`) has landed: after Pull requests, `group: 'work'`. If it hasn't, leave it to that slice. |
| Stash, Tags, Reflog | not rows; reached as tabs of **Branches** (`also`, TASK-045) | The author feels these are missing from the rail. **Ask the author** before adding them back: TASK-045 removed them on purpose. If the answer is to restore them, add them under `tools` with `shows: 'always'` and record the reversal in TASK-045's item. |
| Rebase, Log, All repositories | rows with `shows: 'open'` (only while you're on them) | Same: confirm with the author whether they should always show. |
| Badges | `shows: 'open'`, `delight: true` | Leave as is unless the author says otherwise. |
| Conflicts | `shows: 'conflicts'` | Unchanged. While a Merger merge has conflicts, the Merger row is where the dot goes. Conflicts stays for operations Git stopped by itself (see Open questions). |

## The plan: behaviour

The plan is a **dry run**, and the screen says so ("Dry run · nothing written
yet"). Nothing touches the index, the working tree or any ref until the user
commits.

1. **The stage.** Three cards in a row: branch **A** on the left, branch **B**
   on the right, and a raised **Result** card between them. Arrows run from each
   branch into the Result.
   - Each branch card has a branch picker (any local or remote-tracking branch,
     or a tag, as a source), its count of commits since the merge base, and the
     three newest commits.
   - The arrow from the branch whose commits come in is solid, in that branch's
     colour, labelled "N commits come in". The other is dashed and muted,
     labelled "continues as" (or "starts from" for a new branch).
   - The receiving card is outlined in its colour and badged **Lands here**. The
     other says **Comes in · stays as is**.
2. **Where it lands:** `Into A | Into B | Into a new branch`, plus **Swap**. A
   new branch takes a name (prefilled `merge/<a>-<b>`) and starts from A.
3. **How it lands:** Merge commit, Squash, Rebase then fast-forward, and
   Fast-forward only. Fast-forward is disabled with the reason when both sides
   have their own commits. A small lane graph ("History after") redraws for each
   choice.
4. **The forecast**, in the Result card: one plain sentence for the strategy
   chosen, three numbers (commits in, new commits, files that change), and the
   conflict box: "N conflicts in M files", a meter of conflicted against clean
   files, and where it was measured ("a dry run against <merge base>"). For a
   rebase it says it may stop once per replayed commit.
5. **What changes:** every file either branch touched, grouped by meaning:
   conflicts (red, clickable straight into resolving), both changed but merging
   cleanly, coming in from the source, or already on the target and untouched.
6. **Merge now** is live only when the dry run found no conflicts. Otherwise the
   primary action is **Resolve N conflicts**.

## Resolving: behaviour

1. **Header:** "Merging <source> into <target> · <strategy>", a resolved-count
   bar, **Abort** (returns to the plan, discards choices, writes nothing) and
   **Complete merge**, live only when everything is resolved. Under it: "Nothing
   is written to either branch until you commit."
2. **Files** (left): conflicted files with one dot per conflict (hollow red
   while unresolved, green once resolved), then the files that merge on their
   own, then a legend: A, B, and ✎ "Typed by you".
3. **Three columns per conflict:** A | Result | B, always in that order whatever
   the direction, with the headers saying which side lands and which comes in.
   Each conflict is a card with:
   - its number, location and status chip ("Unresolved", or what was chosen);
   - one sentence saying *why* it conflicts;
   - the context lines around it in all three columns, dimmed;
   - the commit on each side that introduced the change, at the foot.
4. **Every way out of a conflict**, as chips under the card:
   - **Take A** / **Take B**;
   - **Both, A first** / **Both, B first** (the badge order shows which);
   - **Pick lines:** a checkbox on every line of both sides. The result is A's
     ticked lines, then B's;
   - **Edit by hand:** the Result becomes a text box prefilled with the current
     result (or with both sides when nothing was chosen). What's typed is what
     lands;
   - **Reset**, once resolved.
   Plus, per file, **All from A** / **All from B**.
5. **Telling where every result line came from:** each result line carries a
   badge and a side marker (A blue, B amber, ✎ purple). Lines on either side
   that won't land fade to 40%. Unresolved results show a dashed red "Choose
   what lands here" box with the line counts.
6. **Base:** a toggle shows the merge base's version above each conflict ("how it
   looked before either branch"), including "nothing: both sides added lines".
7. **Line numbers** in the Result follow the choices, so later conflicts in the
   same file renumber as earlier ones change length.
8. **The pill:** previous/next conflict across files, "Conflict n of N" with the
   file name, Base, take A, take B, and **Next unresolved**.
9. **Commit dialog:** lists every conflict with what was chosen, takes the
   message (prefilled `Merge branch '<source>' into <target>`, or `Squash …`),
   and its primary button names the strategy (Create merge commit, Commit the
   squash, Finish the rebase). A rebase hides the message box, since each commit
   keeps its own. Then a done state: "<target> now includes <source>", "Open in
   Graph" and "Merge another".
10. **Syntax colours:** every code line, in all three columns and the Base strip,
    is coloured by `highlight.ts` (FEAT-064). The hand-edit box stays plain.

## Colours

| Meaning | Token |
| --- | --- |
| Branch A | `--lane-5` (blue) |
| Branch B | amber, `#a35a00` light / `#f0a64a` dark in the screenshots |
| Typed by you | `#6c4fa3` light / `#b99af0` dark in the screenshots (`--lane-4`'s family) |
| Unresolved | `--danger` |
| Resolved, clean | `--ok` |

**Open: B's amber is too close to `.tok-keyword` (`--lane-3`)** in dark mode, so
keywords on B's tinted rows lose contrast. Ask the author whether to move B to
another colour or soften keywords on side-tinted rows. Colours that must be told
apart must also differ in lightness, not hue alone.

## What the code has today (read on 2026-10-06)

| Need | State |
| --- | --- |
| Merge into the checked-out branch | ✅ `ops::integrate` → `shell::merge` (merge, `--no-ff`, `--ff-only`, rebase) |
| Merge into a branch that is **not** checked out | ❌ needs the merge to run in a worktree (reuse `worktrees.rs`), never a silent checkout |
| Squash | ❌ no `Integration::Squash` (`git merge --squash` + commit) |
| Dry run: conflicted paths and clean files without writing | ❌ `git merge-tree --write-tree --name-only --messages A B` does exactly this, but needs **Git ≥ 2.38**. The linked dev machine has 2.34.1, so a fallback is required: `merge --no-commit --no-ff` in a scratch worktree, read the result, then remove the worktree |
| Merge base and each side's commits | ⚠️ `rebase.rs::merge_base` exists but is private; the graph walk can list the commits |
| Reading conflict stages, regions | ✅ `conflicts::sides`, `regions`, `state` |
| Take a side, write the result, `git add`, continue | ✅ `resolve_region`, `resolve_all`, `take`, `write_merged`, `mark_resolved`, `continue_operation` |
| Both (ordered), pick lines | ❌ `resolve_region` knows only a `Side`; it needs `Both(first)` and a per-line pick. Edit by hand is `write_merged` |
| Choices kept while you leave and come back | ❌ keep them per (A, B, base SHA) in app storage, not `localStorage` |
| Syntax colours | ✅ `src/lib/diff/highlight.ts` (FEAT-064) |
| Scrollbars | ✅ `app.css` already styles them; nothing to do |

## Suggested slices

Each slice is one work item under Amendment 12. Assign the next free `FEAT-###`
when you start each one, not before.

1. **Merger screen, the plan, read-only.** The route, the rail row and icon, the
   stage, direction and strategy controls, the history preview, and the file
   list, built from the merge base and each side's commits. Dry run included,
   with the pre-2.38 fallback. Nothing writes.
2. **Merge without conflicts.** "Merge now" for every direction and strategy,
   into a branch that may not be checked out (worktree), including Squash.
3. **Resolving.** Three columns, every choice (A, B, both in either order, pick
   lines, edit, reset, all from a side), Base, the pill, syntax colours, choices
   kept per merge, Abort, and the commit dialog.
4. **Rebase in Merger.** Stopping once per commit, resolving each stop with the
   same columns, and Continue/Skip/Abort.
5. **Rail tidy-up**, only as the author decides (see [The rail](#the-rail)).

Dependencies: 2 needs 1; 3 needs 1 (it can start before 2); 4 needs 3.

## Acceptance criteria (whole feature)

- Choosing two branches and a direction shows, before anything is written, which
  branch receives the result, how many commits come in, which files change, and
  whether and where there will be conflicts.
- The index, the working tree and every ref are byte-for-byte unchanged until
  the user commits or clicks Merge now (a test like `conflicts.rs`'s
  modification-time check).
- Merging into a branch that isn't checked out never changes the checked-out
  branch or its working tree.
- Every conflict can be resolved as A, B, A then B, B then A, a line-by-line
  pick, or free text, and every result line says where it came from.
- Abort at any point leaves the repository as it was.
- Both themes, every palette family, and keyboard-only use work.

## Open questions for the author

1. **Merger and Conflicts.** The suggestion is that Merger owns merges it starts,
   and Conflicts stays for operations Git stopped by itself (pull, cherry-pick,
   revert, a rebase from the graph), sharing one three-column resolver
   component. Or should Conflicts be folded into Merger?
2. **B's colour** against the keyword colour (see Colours).
3. **Rail entries** that TASK-045 removed or made contextual: restore them, or
   keep them as they are?
4. Should the graph's drag-a-branch-onto-another open Merger's plan instead of
   merging straight away?

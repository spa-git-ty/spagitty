<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-087 — Automated test record

**Item:** [`agile/items/FEAT-087-the-review-screen-and-inbox.md`](../items/FEAT-087-the-review-screen-and-inbox.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/forge/github.rs` | The head commit and a review request read off the row; threads counted open and resolved, and an open thread you started that somebody else answered counted as a reply to you; a row with no threads reads as none; the two searches merge into one list, each pull request once, newest first, an issue skipped; a search with no login is refused before sending; the fragment carries every field and neither query writes. |
| `crates/spagitty-core/src/forge/gitlab.rs` | `sha`, `reviewers` and the project from `references.full` (nested group) read off a merge request. |
| `src-tauri/src/review_state.rs` | Kept under host, owner and name; a nested group becomes nested folders; any part that could climb out of the folder, or is empty, hidden or a drive, is refused; state reads back, forgetting twice is fine; a non-object is neither written nor read. |
| `src/lib/review/inbox.test.ts` | The three groups and their order; your own left out case-insensitively; a requested review stays in Needs you with replies; no empty group; the last group named by scope; size by lines and by files; the size words; progress not started, counted, and changed since; the chips; the facts in order, and none that are not known. |
| `src/lib/review/record.test.ts` | Nothing read as empty; a well-formed record kept whole; a bad field costs only itself and never the pending comments; a range that does not end after it starts dropped; keys for the open repository, a searched row in a nested group, and none without a forge. |
| `src/routes/review/page.test.ts` | Groups, your own left out, chips and the signed-in line; the preview follows the chosen card; a saved review's progress, the key it is read under, opening the room and going back; All my repos, and a row with no clone; a row whose clone is known opens that repository and its room; the host's error with the way to Settings; nothing to review. |
| `src/lib/chrome/chrome.test.ts`, `src/lib/nav.test.ts` | Review on the rail after Pull requests; its dot while a review is asked of you. |
| `src/lib/ui/flat.test.ts` | The new components read only tokens that are defined (`--read-font` and the code tokens are published in `app.css`). |

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1178 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Tests  1 failed | 3065 passed (3066)
$ cargo test -p spagitty-core
test result: ok. 547 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before this branch was cut; it is not
this item's.

In WSL (Arch), because the Tauri crate's test binary does not load on Windows:

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
```

## What is not covered automatically

Against a real host: the GraphQL fields as GitHub answers them today, GitLab's
reviewers on the author's work server, and how the inbox looks. See the sweep.

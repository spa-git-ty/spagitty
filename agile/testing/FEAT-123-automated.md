<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-123 — Automated test record

**Item:** [FEAT-123](../items/FEAT-123-merger-checks-out-what-it-merged-into-and-offers-the-push.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/plan.test.ts` — *where a push of the receiving branch goes* | To its upstream; both ends named when the upstream has another name; to the remote the upstream is on; a remote with a slash in its name read whole; to `origin` under its own name with no upstream; to the first remote when none is `origin`; a remote branch of the same name is not taken for it; an upstream that is not on a remote here falls back to the same name; nowhere with no remote. |
| `src/lib/merger/landed.test.ts` — *checking out* | Checked out after the merge has landed and not before; the repository store is told; the toolbar's name while git does it, with the dialog already saying the merge is made; nothing checked out when it is already the branch here; a new branch is checked out; the other branch when the merge went the other way; left where it is when another worktree has it; this worktree known by its path however it is spelt; a refusal keeps the merge and git's words; asked for again after a refusal; waits its turn; not done for a Farm task. |
| `src/lib/merger/landed.test.ts` — *offering the push* | Names the upstream; pushes that branch to that remote and waits for git; does not depend on the checkout; a rejection shows git's words and can be tried again; waits its turn during a pull; pushed once; nowhere to push with no remote; nothing offered or claimed when the remotes could not be read; forgotten when another merge is started; what comes back for a merge whose dialog was closed is dropped, and does not end the reading of a later one; a checkout that ends after its dialog was closed keeps the branch it moved to and writes nothing else; a pull request's own push is left as it was. |
| `src/routes/merge/page.test.ts` — *once it has landed* | The dialog says the branch is checked out and offers *Push now*, *Not now* and *Open in Graph*, with where the push goes as the button's title and no *Cancel*; pushing ends with *Pushed to* and *Merge another*; *Not now* closes and pushes nothing; a refused checkout is said in git's words and *Check out* asks again; nothing is said when the branch was already here; with no remote there is no push. |

## Results — 2026-10-12

- `bunx vitest run src/lib/merger src/routes/merge`: 124 passed.
- `bunx vitest run`: 4039 passed in 189 files. `bun run check`: zero errors, zero warnings.
- No Rust was changed, so `cargo test` was not run.

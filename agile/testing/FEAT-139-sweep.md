<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-139 — Sweep

**Item:** [FEAT-139](../items/FEAT-139-releases.md)

None of these rows was run in the application. Automated cases are named in the automated record.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT139-01 | Connected hosted remote | Open Releases, filter drafts and pre-releases, then select one | List, notes and files belong to that remote. GitLab links have no invented download count. | P1 | Not run. Mapping and mounted controls covered by tests. |
| SWEEP-FEAT139-02 | Release selected | Hide it, then Show hidden | It disappears locally and returns with Show hidden; host release remains. | P1 | Not run. Covered by command and state tests. |
| SWEEP-FEAT139-03 | Local history and prior tag | Draft a release, enter target/since, Generate from commits and Preview | Notes are grouped and keep request numbers; safe Markdown renders. | P1 | Not run. Covered by core and mounted tests. |
| SWEEP-FEAT139-04 | GitHub or GitLab account | Save draft, reopen and Publish | Each host write confirms. GitLab draft stays local until publication. | P1 | Not run. Local persistence and writes covered by tests. |
| SWEEP-FEAT139-05 | Clean checked-out target, hooks enabled, new tag | Choose Write to CHANGELOG.md and Publish | Commit runs hooks, push is separately confirmed, target movement stops publication, tag contains the new commit. | P1 | Not run. Commit path and resume covered by fixtures/state tests; live host race not driven. |
| SWEEP-FEAT139-06 | Release with files | Choose or drop files; let one fail, retry it, copy its link and remove | Per-file progress and errors; other files stay intact. Every host write confirms. | P1 | Not run. Queue and retry covered by state tests; native drop not driven. |
| SWEEP-FEAT139-07 | Finished run on target has artifacts | From pipeline, Attach, then Upload | Archive download has progress, then its upload is confirmed separately. | P1 | Not run. Queue covered by state test; real artifacts not downloaded. |
| SWEEP-FEAT139-08 | Published release | Delete with tag box off; repeat with it on | Release deletion retains tag by default. Selected tag deletion asks separately. | P1 | Not run. Covered by core and state tests. |
| SWEEP-FEAT139-09 | Unsaved notes | Select another release, cancel discard; then cancel publication | Draft stays intact. | P2 | Not run. Cancelled publication covered by mounted test. |
| SWEEP-FEAT139-10 | Narrow window and light theme | Read list, editor, preview and files | Panels and controls remain legible and reachable. | P2 | Not run. |

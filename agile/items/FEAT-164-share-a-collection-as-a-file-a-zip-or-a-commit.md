<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-164 — Share a collection as a file, a zip or a commit

**Status:** Open — built on `feature/FEAT-164-share`, stacked on FEAT-163, for release 4.0.0. Covered by Rust tests in `calls/share.rs` (every format with every kind of variable, the zip read back, a Postman round trip through the importer, the commit) and `calls/zip.rs`, by the zip case in `import/scan.rs`, and by `ShareDialog.test.ts`. The zip is written by Spagitty itself on `flate2`, which was already in the tree. Not yet looked at in the running application.
**Branch:** `feature/FEAT-164-share`.
**Screens:** 2C (the Share dialog).
**Raised by:** the author, 2026-10-25: make it easy to compact and share, as collection JSON or a zip file.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §5.5 and §7.3. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `Share.dc.html`.

## Change

*Share* writes a collection as one `.json` file (Spagitty or Postman 2.1), as a zip with its environments and example responses, or as a commit under `.spagitty/calls/`. Secrets and kept values never leave the computer.

## Acceptance criteria

- **The dialog** has three choices: *One file*, *Zip bundle* and *Commit to the repository*.
  - *What goes in*: the calls (always), each environment, the last response of each call as an example, and history.
  - A preview of the files and sizes.
  - A line that says secrets and kept values go in empty.
  - The button names what it will do.
- **One file**: *Spagitty* or *Postman 2.1*. Postman export turns keep rules into `pm.environment.set` lines, and SOAP calls into raw POSTs.
- **Zip**: `collection.json`, `env/<name>.json`, `examples/`, optional `history.json`, and a `README.md` that lists the values each person must fill in. Import reads the zip back.
- **Commit**: stages only the files the dialog lists and commits them on the current branch with the message *Calls: update <collection>*. It never pushes.
- **No secret leaves**: a test shares a collection with every kind of variable in every format and asserts that no secret or kept value appears in the output.
- **Default format**: from Settings › Calls (*Share as*).
- **Tests**:
  - Rust tests for each format.
  - A Postman round trip (export, import, compare).
  - The secret test above.
  - A component test for the dialog.

## Not in this item

- Sharing through a server, a link or an account.

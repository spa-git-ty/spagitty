<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-160 — Calls: send a REST call and keep the collection in the repository

**Status:** Open — built on `feature/FEAT-160-calls`, for release 4.0.0; the author accepted the network boundary on 2026-10-25 (see the plan). Covered by Rust tests in `calls/` and `src-tauri/src/calls.rs`, and by `src/lib/calls/*.test.ts` (helpers, model and the screen); not yet looked at in the running application.
**Branch:** `feature/FEAT-160-calls`.
**Screens:** 2C (Calls, new, at `/calls`), chrome (a rail row).
**Raised by:** the author, 2026-10-25: a tab with the idea of Postman, not Postman: a simple screen, easy to use and not cluttered.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §5 and §10. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `Main.dc.html`.

## Change

Calls sends HTTP requests the person has saved in collections under `.spagitty/calls/`, and shows the response beside them. Requests are made in Rust; the webview sends nothing.

## Acceptance criteria

- **A rail row** in Repository, after Recipes, with code 2C and the `calls` icon. FEAT-134's rail settings list it, pinned by default.
- **The screen** (§5.1):
  - The header has a chip per open call with ✕ (middle-click and `Ctrl+W` close too), `+`, the environment picker, *Import* and *Share*. Open calls are kept per repository.
  - Collections are on the left, with a filter (`Ctrl+K`) and *History* at the foot.
  - The request is above the response.
  - The variables panel is on the right.
- **The request**: method, URL, *Send* (`Ctrl+Enter`). Tabs *Params*, *Headers*, *Body* (JSON, XML, form, multipart, raw, none), *Auth* (none, bearer, basic, API key), *Keep*. Tab counts show when non-empty.
- **Variables** (§5.2):
  - `{{name}}` is a pill, and hovering shows its value.
  - Typing `{{` offers names.
  - An unknown name is drawn in `--danger` and blocks *Send* with a message naming it.
  - Built-ins `$timestamp`, `$isoNow`, `$uuid`, `$randomInt`.
- **Sending** (§5.3):
  - Requests are sent from `calls/send.rs` only. A test asserts that `ureq` is reachable from exactly `forge/http.rs` and `calls/send.rs`.
  - A send can be cancelled.
  - Responses are capped at 50 MB, with no colour over 2 MB.
  - A binary body shows its type, its size and *Save as…*.
  - `text/event-stream` streams.
- **The response**: status with a dot, time, size; tabs *Body*, *Headers*, *Cookies*; *Beautify* (§5.4), on by default, display only.
- **Storage** (§5.5):
  - Collections are `.spagitty/calls/<name>.json`, format v1, written with stable key order and two-space indent.
  - Per-person state is in application data.
- **History**:
  - The resolved request, with secrets replaced by their names, plus status, time, size, and the response up to 1 MB.
  - Kept per repository and capped by Settings.
- **Ask before sending**: an environment with it on asks once per send, naming the environment in its colour.
- **Tests**:
  - Rust tests for sending against a local test server: methods, bodies, redirects, timeouts, cancel, the size cap, and the reachability test above.
  - Round-trip tests for the file format.
  - Component tests for the screen, tabs and closing calls.

## Not in this item

- Environments' screen, secrets and kept values (FEAT-161). This item reads plain variables only.
- Import (FEAT-162), SOAP (FEAT-163), Share (FEAT-164), Settings › Calls (FEAT-165).
- Scripts of any kind.

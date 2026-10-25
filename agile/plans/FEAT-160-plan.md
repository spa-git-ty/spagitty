<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-160 — Plan

**Item:** [FEAT-160](../items/FEAT-160-calls-send-a-rest-call-and-keep-the-collection-in-the-repository.md)

The author's answers to the proposal's §15, 2026-10-25: Calls is accepted as a third network surface (`calls/send.rs`, beside `forge/http.rs` and `models/http.rs`), following redirects and allowing plain `http` to any host. The folder is `.spagitty/calls/` by default and can be changed per repository in Settings. The name is *Calls*, 2C. GraphQL, gRPC and WebSocket come later. Proxy, client certificates, TLS checking off per environment and streamed `text/event-stream` are all in 4.0.0. Everything ships in one 4.0.0 release.

1. `crates/spagitty-core/src/calls/`, a new module:
   - `model.rs`: the collection format v1 (folders, REST and SOAP calls, auth, body, keep rules), environments and `shared.json`, written with stable key order, two-space indent and a final newline.
   - `store.rs`: list, read, write, rename and delete collections and environments under the folder. A file that does not parse is listed with its error and is never written over.
   - `vars.rs`: `{{name}}` found, resolved (environment, then shared, then the collection's own) and redacted, and the built-ins `$timestamp`, `$isoNow`, `$uuid` and `$randomInt`. An unknown name is an error that names it.
   - `send.rs`: the only file in Calls that uses `ureq`. One agent per send, built from the options: timeout, redirects, TLS checking, proxy (system, none or a URL) and a client certificate. The body is read in chunks, so a send can be cancelled between them. The body is capped at 50 MB, and chunks are handed out as they arrive for an event stream.
   - `history.rs`: the per-repository history in application data. An index plus one file per response body, each up to 1 MB, trimmed to the cap.
2. `src/lib/requests/requests.test.ts`: the reachability test names three files.
3. `src-tauri/src/calls.rs`: where the folder is, the per-person state (active environment, open calls, collapsed collections, filter) and history in the data directory, and a thread per send with `calls-chunk` and `calls-done` events and `calls_cancel`.
4. Frontend, `src/lib/calls/`:
   - `types.ts` and `api.ts`.
   - `store.svelte.ts`: collections, open calls, edits saved as they are typed, sending, responses and history.
   - `vars.ts`: pills and `{{` suggestions.
   - `view/json.ts` and `view/xml.ts`: formatted lines with each value's path, for colour and, in FEAT-161, keeping.
   - Components: `CallsScreen`, `CollectionTree`, `RequestEditor`, `VarField`, `VarArea`, `PairsTable`, `ResponseView`, `BodyView`, `VariablesPanel`.
5. 2C on the rail after Recipes with a `calls` icon, the route `/calls`, a palette entry, and `docs/screens.md`, `docs/architecture.md` and `NOTICE` brought up to date.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-162 — Import calls from code and from collection files

**Status:** Open — built on `feature/FEAT-162-import`, stacked on FEAT-165, for release 4.0.0. Covered by Rust tests in `calls/import/` (Spring, WebFlux, JAX-RS and NestJS sources; OpenAPI 3 and Swagger 2; Postman 2.0 and 2.1, Insomnia, Bruno, `.http`, HAR and cURL; change detection and bringing in; a scan of a fixture repository), and by `ImportScreen.test.ts`. A Spagitty zip is read once FEAT-164 writes one; until then a Spagitty collection is read as its single `.json`. Not yet looked at in the running application.
**Branch:** `feature/FEAT-162-import`.
**Added 2026-10-26:** the author asked for NestJS too. It is read like Spring, from TypeScript: `@Controller` and its methods' `@Get` and others, `@Param`, `@Query`, `@Headers` and `@Body`; bodies from the DTO with class-validator and `@ApiProperty({ example })`; `setGlobalPrefix` and the `listen` port from `main.ts`. *NestJS* is a framework in Settings › Calls, on by default.
**Screens:** 2C (Calls › Import, at `/calls/import`; the *Found in this repository* card on Calls).
**Raised by:** the author, 2026-10-25: when there is a collection file in the repository, detect it, or select one from outside; or better, scan all endpoints and let the person choose whether to create collections or calls from them.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §8 and §7.3. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `Scan.dc.html`.

## Change

Import reads endpoints from Java source, from OpenAPI, and from collection files in the repository or from outside. The person ticks which endpoints to bring in, as calls in an existing collection or as a new one.

## Acceptance criteria

- **The screen** (§8.1):
  - Sources in three groups: *Scan the code*, *Collection files in this repository*, *From outside* (*Choose a file…*, *Paste cURL*).
  - Endpoints grouped by controller with the file. Each row has a checkbox, method, path, a hint, and a tag: *New*, *Changed* or *In <collection>*.
  - Filters: *All*, *New*, *Changed*, *Already in*; and *Scan again*.
  - *Bring in* has *As*, *Into* or *Name*, *Folders*, and three switches. Its button names the count and the target.
  - *New* and *Changed* rows are ticked by default.
- **Java** (§8.2), with the tree-sitter Java grammar already compiled in:
  - Spring MVC and WebFlux mappings, and JAX-RS, with their parameters.
  - Example bodies from the `@RequestBody` class or record, to a depth of 4, shaped by Bean Validation annotations.
  - `baseUrl` for a `local` environment from `application.yml` or `application.properties`.
- **What is read**: the working tree, read-only, skipping ignored paths and the Settings globs. The scan is in the background, streamed, cancellable and capped at 20,000 files. Nothing is built or run.
- **Files** (§8.3): Postman v2.0 and v2.1 (collections and environments), Insomnia exports, Bruno folders, OpenAPI 3 and Swagger 2, `.http` and `.rest`, HAR, cURL, and a Spagitty zip. Files found in the repository can be sent in place and are never rewritten.
- **Postman scripts** (§7.3): `pm.environment.set(…, pm.response.json().…)` and `pm.collectionVariables.set` lines become keep rules. Other lines go into the call's notes, and Import says how many calls had them.
- **Changed and new** (§8.4):
  - Endpoints match by method and normalised path.
  - *Changed* names what changed.
  - Bringing a changed call in adds and removes fields, and keeps the person's values.
- **The card on Calls** appears only when endpoints are found that are in no collection.
- **Tests**:
  - Rust tests over fixture projects for Spring, WebFlux and JAX-RS, and over each file format.
  - Tests for change detection.
  - A component test for the screen.

## Not in this item

- SOAP sources (FEAT-163).
- Micronaut, Quarkus, Express, FastAPI and other frameworks: each later, as its own item.
- Fetching remote `$ref`s without the person asking.

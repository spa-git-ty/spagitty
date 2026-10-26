<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-100 — Bruno collections in YAML do not import

**Status:** Fixed — on `fix/BUG-100-bruno-yaml-import`, in release 4.0.1. Covered by tests, and checked against the author's two collections (DXP and ESB): all 272 requests and 16 environments read. Not tried in the app; the author does the sweep.
**Branch:** `fix/BUG-100-bruno-yaml-import`.
**Screens:** Calls › Import (2C).
**Raised by:** the author, 2026-10-26: "i tried to import those collections but they didn't work ! they import fine on bruno".

## Cause

Import read Bruno's `.bru` format only. Bruno now writes the OpenCollection format: an `opencollection.yml` at the root, a `folder.yml` in each folder, one `.yml` file per request, and `environments/*.yml`. Neither the scan nor *Choose a file…* recognised any of it. A Bruno collection is also a folder, and Import could only be given one file, so even a `.bru` collection from outside the repository came in one request at a time.

## Fix

- A reader for the YAML format, `calls/import/opencollection.rs`. It reads the method, URL, headers, query and path parameters, the JSON, XML, text, GraphQL, form and multipart bodies, and bearer, basic and API-key auth. `auth: inherit` takes the auth of the folder above, up to the collection's. The collection's and the folders' headers are sent first, as Bruno does. Environments come in with secrets left empty. `res.body…` runtime variables become keep rules. Scripts and assertions go into the call's notes, because they are not run. Requests keep Bruno's order.
- The scan finds `opencollection.yml` in the repository, as it finds `bruno.json`.
- *Choose a Bruno folder…* on Import reads a whole collection, YAML or `.bru`. Choosing a collection's `opencollection.yml` or `bruno.json` with *Choose a file…* does the same. A single request `.yml` file comes in on its own.

## Known limits

- A request with no `auth` has none, even under a collection that has one: Bruno leaves `auth` out when it is *No Auth*.
- Folders come in flat and are named by their path (`cards / debit cards`), as for Postman and `.bru` collections.

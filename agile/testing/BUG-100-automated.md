<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-100 — Automated test record

**Item:** [BUG-100](../items/BUG-100-bruno-yaml-collections-do-not-import.md)

- `calls::import::opencollection` tests: inherited auth and headers, path and query parameters, a switched-off header, an empty JSON body, scripts in the notes; a folded, double-quoted XML body as Bruno writes it; multipart parts with files and switches; a request with no `auth`; environments with secrets empty; Bruno's order.
- `calls::import::scan::tests::a_bruno_yaml_collection_is_read_from_its_folder_or_its_root_file`: a folder, its `opencollection.yml` and a lone request file chosen from outside; hidden folders skipped; a folder that is not a collection refused.
- `src/lib/calls/ImportScreen.test.ts`: *Choose a Bruno folder…* opens a folder picker and imports the folder.

## Results — 2026-10-26

- `cargo test -p spagitty-core --lib calls`: 117 passed.
- `bunx vitest run src/lib/calls`: 66 passed.
- A throwaway test, not committed, read the author's collections from `Documents/collections`: DXP 227 requests in 32 folders and 8 environments, ESB 45 requests and 8 environments. That is all 272 request files.

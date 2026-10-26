<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-100 — Plan

**Item:** [BUG-100](../items/BUG-100-bruno-yaml-collections-do-not-import.md)

1. `crates/spagitty-core/src/calls/import/opencollection.rs`: read `opencollection.yml`, `folder.yml`, request `.yml` files and `environments/*.yml` with `yaml-rust2`, which the crate already uses. Resolve `auth: inherit` and the inherited headers through the folders to the collection, and keep Bruno's `seq` order.
2. `crates/spagitty-core/src/calls/import/scan.rs`: the scan finds `opencollection.yml` as it finds `bruno.json`; `read_file` takes a folder, or a collection's root file, and reads the whole collection, YAML or `.bru`; a lone request `.yml` comes in on its own.
3. `src/lib/calls/ImportScreen.svelte`: *Choose a Bruno folder…* beside *Choose a file…*.
4. Check the reader against the author's DXP and ESB collections before committing.

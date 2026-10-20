<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-154 — Automated test record

**Item:** [FEAT-154](../items/FEAT-154-usages-of-a-name.md)

- `crates/spagitty-core/src/usages.rs`: in a fixture with Rust, TypeScript, Markdown, JSON and plain text, `greet` is a function definition in Rust, a call in a method, a mention in a doc comment and in a format string, and a mention in Markdown and JSON; definitions come first. In TypeScript the two calls on one line, one inside `${…}`, are uses and the string is a mention. `greeting` and `greeter` are other names; `Greet` finds nothing; `Greeter` is a class definition. A limit of 2 says there are more; `greet(`, an empty name and `9lives` are refused. Every tags query compiles against its grammar.
- `crates/spagitty-core/src/files.rs`: the search tests still pass with the walk shared.
- `src/lib/files/usages.test.ts`: the name around a column, from its first letter to just past its last, `$` included; nothing on a space, a symbol or a number; names in any script.
- `src/lib/files/files.test.ts`: Ctrl-click on `two` asks for its usages at the resolved revision, moves the cursor and opens no card; ⌘-click off a name asks nothing. The list counts each role, folds mentions, labels a definition's kind, and opens a place with the name marked. A typed name is looked up; nothing found and a refusal are each said.

## Results — 2026-10-20

- `cargo test -p spagitty-core --lib usages`: 5 passed. `--lib files::`: 22 passed.
- `cargo clippy -p spagitty-core -p spagitty --all-targets -- -D warnings`: clean.
- `bunx vitest run src/lib/files`: 168 passed.

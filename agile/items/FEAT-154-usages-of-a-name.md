<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-154 — Usages: where a name is defined and used

**Status:** Done — on `feat/FEAT-154-usages`, stacked on `feat/FEAT-153-blame-on-click` (#85); unmerged. Covered by Rust and component tests; not looked at in the running application (the author is checking it).
**Branch:** `feat/FEAT-154-usages`.
**Screens:** 1U (Files).
**Raised by:** the author, 2026-10-20: "i would lkike to know it's impact ( and it's ) : ( funcions , classes , packages etc ..) referencng , so I click ctrl and class name or method name and we list it's usage ( like editors )", then "for the feature do what you want i trust you".

## Decision

An editor finds references with a language server that builds the project. Files reads git's objects at any branch, tag or commit and has no checked-out project to build, so a language server is not what this is. It is the search-based navigation code hosts use: every place the name stands as a whole word, case and all, and then each place read with its file's tree-sitter grammar — the grammars Files already compiles in for colour — and its tags query. It works at every revision and in every language Files colours, and it is by name, not by type; the screen says so.

## Change

- **Ctrl-click a name in a file (⌘ on a Mac).** The tree turns to a third mode, *Usages*, and lists where the name is:
  - **Defined** — where the grammar's tags query names a definition: a function, a class, a method, a module, an interface, a macro.
  - **Used** — everywhere else in code, with what the grammar calls it when it says (`call`, `implementation`, `type`).
  - **Mentions**, folded at first — in a comment, a string, or a file of data or prose (JSON, YAML, TOML, Markdown, CSS).
- A heading counts each, and the files: the impact of changing the name at a glance.
- A place opens the file at its line with the name marked.
- Holding Ctrl (⌘) underlines what a click would look up.
- *Usages* in the tree has a field: type a name and press Enter.
- Definitions and uses are told apart in Rust, TypeScript, TSX, JavaScript, Python, Go, Java, C, C++ and C#. Svelte, HTML, Bash and SQL tell comments and strings apart; a file with no grammar lists every place as a use.

## Acceptance criteria

- Ctrl-click (⌘-click) on a name lists its definitions, uses and mentions; a click off a name does nothing.
- A definition, a call, a mention in a comment, one in a string and a substitution inside a template string are each told apart in Rust and TypeScript.
- Only whole words of the same case are found.
- More than 2,000 places say there are more.
- A place opens the file at its line with the name marked.

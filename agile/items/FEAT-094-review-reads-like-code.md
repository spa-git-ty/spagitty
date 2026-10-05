<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-094 — Review reads like code

**Status:** Open — built on `feature/FEAT-094-review-reads-like-code`, not yet merged.
**Branch:** `feature/FEAT-094-review-reads-like-code`
**Screens:** 1R; the Diff, File history and Pull requests screens share its parts.
**Raised by:** the author, 2026-10-05: "a very important feature that we need
to add code syntax coloring, pans to be expandable and on this part we can
render the md text right" — the inbox preview showing dependabot's table as
rows of pipes.

## Problem

- The review room drew code in one colour. The highlighter the Diff screen
  used (FEAT-064) read each line alone, so a comment or string spanning lines
  lost its colour after the first; it knew no Kotlin, Java, Gradle, CSS,
  HTML, XML or Markdown; and on the Pomodoro palette keywords and numbers were
  the same amber, types and strings the same green.
- The inbox preview was 330 px wide and gave the description 40% of its height;
  the room's files and Conversation card were fixed too.
- `PRMarkdown` was a hand-rolled subset: no tables, no nested lists, raw HTML
  shown as text (dependabot's `<details>`), and a `[x](javascript:…)` link was
  made live in a window that can run git.

## Scope

- **Colour.** The highlighter reads a file line by line with what each line
  leaves open carried on: block comments, strings over several lines, tags
  whose attributes wrap, and a Svelte, Vue or HTML file's `<script>` and
  `<style>` in their own languages. New: Kotlin, Java, Groovy/Gradle, Swift,
  C#, Dart, PHP, Ruby, Scala, CSS/SCSS, HTML, XML, Svelte, Vue, Markdown,
  INI/properties/.env, Dockerfile; keys in TOML, YAML, JSON and INI; Rust
  lifetimes, macros and attributes; annotations and decorators. The room reads
  the old and new sides of each file apart and draws the changed words over
  the colours. The tokens get one colour each from the palette's five lanes.
- **Panes.** The inbox preview, the room's files and its Conversation card
  widen and narrow by dragging their edge (or the arrow keys on it), kept
  across restarts like every other panel. The preview's description takes the
  room there is.
- **Markdown.** `marked` (GitHub-flavoured) renders, and the result is rebuilt
  from an allow-list: tables, task lists, nested lists, line breaks,
  `<details>`, fenced code in colour. Links only if http(s) or mail, opening
  in a window of their own; images become links to them; no script, style,
  form, frame, event handler, id or foreign class. The inbox preview, the Pull
  requests screen and comments in the room all use it; the Conversation card's
  previews show the words without the marks.

## Acceptance criteria

- A Kotlin, TypeScript, Rust, TOML or Svelte file in the room is coloured by
  kind, including inside a comment or string that spans lines.
- The changed words of an edited line are still marked.
- The preview, the files and the Conversation card resize, and keep their
  width after a restart.
- A dependabot description shows its table as a table and its release notes
  as a `<details>` that opens.
- No `javascript:`, `data:` or relative link is followed; no `<script>`,
  `<iframe>`, `onerror` or `style` reaches the page.

## Dependency

`marked` 18 (MIT, no dependencies of its own). GitHub-flavoured Markdown —
tables, nested lists, HTML blocks, task lists — is a specification, not a
handful of regular expressions, and the subset it replaces got enough of it
wrong to show here. The sanitising is not delegated: DOMPurify was tried and
dropped, because under happy-dom, where the tests run, it kept `onerror` while
reporting itself supported, so its behaviour could not be tested. The rebuild
in `src/lib/ui/markdown.ts` only reads the parsed tree and only creates what
its lists name, which behaves the same in the webview and in the tests.

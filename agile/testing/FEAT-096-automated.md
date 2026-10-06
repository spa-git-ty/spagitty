<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-096 — Automated test record

**Item:** [`agile/items/FEAT-096-an-extension-host-anyone-can-build-for.md`](../items/FEAT-096-an-extension-host-anyone-can-build-for.md)

## What was written

**The contract, read by every implementation.** `schemas/extensions/fixtures/`
holds two valid manifests, fifteen invalid ones (each naming the field the
error must point at) and a review record. They are read by
`crates/spagitty-extensions/tests/contract.rs` (Rust host),
`packages/extension-sdk/src/sdk.test.ts` (TypeScript kit) and
`src/lib/extensions/boundary.test.ts` (frontend). The contract test also
checks that the published schema lists exactly the capabilities and targets
the host knows.

**Unit tests in the host** (`cargo test -p spagitty-extensions --lib`, 93):
manifest rules and every error reported at once; version ranges in both
spellings and the pre-release rule; JSON-RPC framing, including a newline
inside a value; redaction of bearer tokens, token prefixes, API-key flags and
URL userinfo; the ZIP reader refusing traversal, absolute, drive, UNC,
backslash, stream, reserved and control-character names, links of both kinds,
case collisions, lying CRCs and sizes, bombs and unknown methods; package
validation (tampering, unlisted and ghost files, missing entrypoints, programs
disguised by name, magic or mode); install, update with grant carry-over,
rollback, uninstall that never follows a link; reserved identities; review
gate decisions for every status, staleness, unknown severity and threshold;
snapshot digests changing on edit and on staging but not otherwise; a main
checkout and its worktree sharing one repository identity; history retention,
redaction and deletion; argv built from profiles refusing option-shaped,
range-shaped and shell-shaped values.

**Integration tests against real processes**
(`crates/spagitty-extensions/tests/host.rs`, 23), each starting the scripted
`spagitty-test-worker`: lazy start; grants and forged handles refused on
callbacks; six concurrent callbacks during one operation; a second completion
ignored and logged; cooperative cancellation; a worker that ignores
cancellation ended after the grace with its tree; a crash failing its
operation, redacting stderr, and three crashes keeping it stopped until a
restart; inactivity; a wrong API major refused before anything runs; a silent
handshake ended; non-protocol stdout stopping the worker; a declared tool
streaming its output; an undeclared option refused; a tool's process tree
ended on cancellation (`sleeper-survived` never written); findings validated
(duplicate id, escaping path and non-https link dropped, dispositions reset,
no location invented); a stale result when the working copy changes mid-review;
partial and billing outcomes never success; nothing running where not enabled
or consented; panels; settings validation and delivery; a packed copy
installed, run and uninstalled; a development request picked up once.

**Cross-implementation** (`tests/foreign_package.rs`, ignored by default): a
package built by `bun run ext pack examples/extensions/hello --build` is
inspected, installed, enabled, run, its panel drawn, disabled and uninstalled
by the Rust host.

**Frontend** (`src/lib/extensions/*.test.ts`, 47): markdown read into values,
hostile HTML rendered as text, only `https:` links, images reduced to alt text;
contribution placement, palette and menu flags, availability reasons, nothing
contributed by a failed, disabled or incompatible extension; the palette
replaced as a group; operation progress, failure notices, cancellation; a
review command opening the scope dialog and nothing sent until Start; farm
context worktree and base; backend confirmations; components — actions,
findings panel (result vs approval, dismissal, selection, running state, long
text), the card's consent dialog and official badge, the install dialog's
trust statement, panels from data, "not observed" never a pass; and boundary
checks that only `api.ts` invokes the backend and no source hands content to
the browser as markup, fetches, holds a token or evaluates anything.

**SDK and tools** (`packages/extension-sdk/src/sdk.test.ts`, 17;
`tools/extensions/cli.test.ts`, 7): manifest parity with the fixtures, version
rules, the worker runtime under the fake host (handshake, exactly-once
completion, thrown errors, cancellation, refusals keeping their code, reviews,
panels), ZIP round trip, packing only built targets, undeclared programs
refused, tampering found; scaffolding, the development request location per
platform, validation and usage.

## Test command and output

```
$ cargo test -p spagitty-extensions
test result: ok. 93 passed   (lib)
test result: ok.  4 passed   (contract)
test result: ok. 23 passed   (host)
$ SPAGITTY_TEST_PACKAGE=examples/extensions/hello/dist/com.example.hello-0.1.0.spagitty-extension \
    cargo test -p spagitty-extensions --test foreign_package -- --ignored
test result: ok. 1 passed
$ cargo test -p spagitty-core compare      ok. 5 passed
$ cargo test -p spagitty-process           ok. 2 passed

$ bunx vitest run src/lib/extensions packages tools/extensions
Tests  71 passed
$ bun run ext test examples/extensions/hello
2 pass — Conformant: handshake, activation, deactivation and exit.
```

Run on Windows 11 (x86_64-pc-windows-msvc) on 2026-10-06. The host tests
exercise the Windows Job Object path; the Unix process-group path runs in CI's
`farm processes` lane, which now includes this crate.

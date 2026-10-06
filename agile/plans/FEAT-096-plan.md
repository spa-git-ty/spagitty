<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-096 — Plan

**Item:** [`agile/items/FEAT-096-an-extension-host-anyone-can-build-for.md`](../items/FEAT-096-an-extension-host-anyone-can-build-for.md)

**Rules prerequisite, unresolved.** `AGENTS.md` binds work here to the shared
amendments book at `/home/maxmya/dev/agents/docs/AMENDMENTS.md`. On 2026-10-06
that path did not exist on this Windows machine, nor anywhere under its WSL
distribution (`find / -name 'AMENDMENTS*.md'` outside this repository found
nothing). This work follows the rules the repository itself records —
`agile/README.md` (Amendment 12's four documents, identifiers never reused),
`CONTRIBUTING.md` (SPDX headers, DCO, permitted licences), `docs/ci.md` (the
gates and coverage floors) and `docs/architecture.md` (Git Flow branches named
after the item) — and claims compliance with nothing it has not read.

## Dependency graph

```text
src/lib/extensions ─invoke─► src-tauri/src/extensions.rs (composition)
                                   │            │
                                   ▼            ▼
                       spagitty-extensions   spagitty-farm   (FEAT-098 declares
                         │        │            │              the supplemental
                         │        ▼            ▼              review interface)
                         │     spagitty-core ◄─┘
                         ▼
                    spagitty-process  ◄── spagitty-farm (execution::tree)
```

- `spagitty-process` is **extracted**, not written: `execution/tree.rs` moves
  there verbatim with its test, and the farm keeps `execution::tree` as a
  re-export so nothing else in the farm changes. Both the farm's agent runs and
  the extension host's workers and tool runs get Unix process groups and Windows
  Job Objects from one implementation.
- `spagitty-extensions` has no Tauri and no frontend dependency, and does not
  depend on `spagitty-farm`. It depends on `spagitty-core` for repository
  reads (status, diff, refs) and on `spagitty-process`.
- The desktop crate composes the two: it supplies the extension host's
  `HostServices` (forge reads and writes with the backend-held token, UI
  confirmations, notices) and, in FEAT-098, the farm's supplemental reviewer.

## The contract

Written once, as public documents, and implemented three times — the Rust host,
the TypeScript SDK, and the Rust CodeRabbit worker, which deliberately does
**not** link the host crate so that two independent implementations have to
agree on the wire:

| Document | Holds |
| --- | --- |
| `schemas/extensions/manifest.v1.schema.json` | The authoritative manifest schema |
| `schemas/extensions/protocol.v1.md` | Methods, messages, ordering, limits, error codes |
| `schemas/extensions/review.v1.schema.json` | The shared review model on the wire |
| `schemas/extensions/fixtures/` | Valid and invalid manifests and protocol transcripts, read by the Rust and the TypeScript tests alike |

### Decisions and why

**Process, not library.** Proposal §4. A crash is a disconnected worker, not a
dead window; nothing third-party is loaded into Spagitty's address space; a user
needs no JavaScript runtime installed to run a packaged extension.

**JSON-RPC 2.0, newline-delimited.** The cheapest framing every language
already has. Host-originated request ids are integers; worker-originated ids
are strings beginning `w`. A message with a `method` is a request or
notification; one without is a response, matched against the sender's own
pending set. A worker request is dispatched on its own thread so a callback
that takes minutes (`tools.run`) never stops the reader — the deadlock the
proposal warns about is impossible by construction rather than by care.

**Exactly one terminal result per operation.** `operation.complete` is accepted
once; anything for that operation afterwards — findings, progress, a second
completion — is dropped and logged, and the drop is what a test asserts. A late
completion cannot advance a farm task twice because the farm reads the
recorded result, not the message.

**Capabilities are checked on every callback, in the host**, against the
extension's identity, its grants for the repository the handle belongs to, the
operation the callback names, and the extension's state. The UI greying a
button is a convenience, not the check. Unknown capability names are a
manifest error.

**Repository handles are opaque and scoped.** A worker never receives a path it
can hand back to read another repository: it receives `repo:<n>` minted for
one session and one repository, and it expires when the session ends.
Repository identity — the key grants and enablement are stored under — is the
canonical path of the repository's *common* git directory, so a main checkout
and every farm worktree cut from it are one repository and need one grant.

**External tools run in the host.** `tools.execute` is not "run a command": the
manifest declares each tool's executable names and **command profiles** — a
fixed argv prefix plus typed options (`enum`, `revision`, `commit`) that map to
fixed flags. The host builds argv from a profile, never from a string, runs it
with the operation's approved working directory, streams its stdout lines back,
owns its process tree, and kills that tree on cancellation. A revision is
validated (no leading `-`, no whitespace, no `..`, at most 255 bytes) before it
reaches argv, so a branch called `--output=/etc/passwd` is refused rather than
passed.

**Executables are chosen by the user.** Detection searches `PATH` for the
declared names (with `PATHEXT` on Windows) and the user may pick another file.
The choice is user state; a repository file can neither name an executable nor
grant itself a capability.

**Contributions are data.** Commands, actions and panels carry ids, titles,
contexts and a finite list of predicates (`repositoryOpen`,
`hasWorkingChanges`, `taskSelected`, `taskHasCommit`, `pullRequestSelected`,
`forgeConnected`). No expressions, no `eval`, no components. Panels name one of
three host renderers. Markdown from an extension is rendered by an allow-list
renderer that emits no raw HTML and loads no remote media; links are inert text
until the user chooses to copy one.

**Packages are validated before extraction.** A minimal ZIP reader written for
this purpose — central directory only, stored and deflate entries only, CRC and
size checked — rejects traversal, absolute, drive and UNC paths, symlinks,
duplicate paths including case collisions, an expansion past 256 MiB or a ratio
past 200:1, executables that the manifest does not declare, a declared
entrypoint that is missing, and any file whose SHA-256 differs from the
integrity index. A hand-written reader rather than the `zip` crate because it
needs none of `zip`'s writer, encryption or compression families, and because
every rule above has to be enforced before a byte is written, which is easiest
to show in code that does nothing else. Deflate comes from `flate2` and CRC
from `crc32fast`, both already in `Cargo.lock`.

**Install layout.** `<app data>/extensions/packages/<id>/<version>/` is written
once into a staging directory, validated, then renamed into place; the
installed version is a pointer in `<app data>/extensions/state.json`, written by
rename. Update keeps the previous version until the next successful update, so
rollback is a pointer change. Uninstall removes only the validated
`<id>` directory. The bundled official package lives in the application's
resources and its identity is reserved: an import cannot replace it or claim
its badge, whatever its manifest says.

**Lifecycle.** States `installed`, `disabled`, `starting`, `active`, `stopping`,
`failed`, `incompatible`. Static contributions register from the manifest
without starting anything; the worker starts on first use. Handshake deadline
10 s, cooperative cancellation grace 5 s then the tree is killed, inactivity
120 s without a progress or heartbeat, absolute deadline 45 min (a setting).
A crash fails the running operations, unregisters the active contributions
and leaves the extension `failed`; three crashes in ten minutes keep it there
until the user restarts it. No restart is automatic, so a retry can never
repeat a billable review or a posted comment.

**The review model** is the proposal's §8 types, carried as wire fixtures that
Rust and TypeScript both parse. The gate is computed by the host — from the
result's status and completeness, the snapshot still being current, the policy
threshold and the findings — and a provider has no field it can set to pass it.
Severity mapping belongs to the provider; `unknown` blocks a required gate.

**History** is minimal records — schema version, provider and version,
snapshot identity, times, completeness, status, summary and findings with
their dispositions — under `.spagitty/extensions/reviews/`, written by rename,
last 50 per repository, deletable from the panel. No patch or file content is
stored. Provider messages are passed through `redact` (bearer tokens, `cr-`
API keys, `ghp_`/`gho_`/`github_pat_` tokens, URL userinfo) before storage,
events and diagnostics.

**Developer kit.** `packages/extension-sdk/` (types, transport, handler
registration, typed host callbacks, a fake host) and `tools/extensions/cli.ts`
(`create`, `dev`, `validate`, `test`, `pack`, `inspect`), runnable with
`bun run ext <action>`. `pack` produces native executables with
`bun build --compile` for the targets asked for; only the targets that were
actually built are written into the packaged manifest.

## Alternatives considered

**WASM.** Real containment, but no WASM runtime is in the dependency tree,
every review tool worth integrating is a native CLI that WASM could not spawn
anyway, and the proposal defers it.

**Extension JavaScript in the webview.** Would hand every extension the
window, the command bridge and everything the user can see. Rejected by §4.

**`semver` for ranges.** Used: it is already in `Cargo.lock` (through Tauri's
build tooling), MIT/Apache-2.0, and its `VersionReq` is the Cargo dialect.
The schema documents that dialect (`>=0.9.0, <1.0.0`, `^1.0`), and the TS SDK
implements the same subset with tests over the shared fixtures.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-process/` | `ProcessTree`, moved from the farm with its test |
| `crates/spagitty-extensions/src/{manifest,version,capabilities,protocol,host,operations,tools,package,zip,registry,storage,review,redact,snapshot}.rs` | The host |
| `crates/spagitty-extensions/src/bin/spagitty-test-worker.rs` | A scripted worker the host's tests drive |
| `crates/spagitty-extensions/tests/` | Lifecycle, package, callback and grant tests against real processes and real repositories |
| `src-tauri/src/extensions.rs` | Commands, events, `HostServices`, confirmations |
| `src/lib/extensions/` | Bridge, store, contribution registration, renderers, settings section |
| `src/lib/settings/`, `src/routes/{changes,farm,requests,settings}` | Entry points |
| `packages/extension-sdk/`, `tools/extensions/`, `examples/extensions/hello/` | Developer kit and example |
| `schemas/extensions/`, `docs/extensions/` | Contract and guides |

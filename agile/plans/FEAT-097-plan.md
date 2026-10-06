<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-097 — Plan

**Item:** [`agile/items/FEAT-097-coderabbit-reviews-local-changes.md`](../items/FEAT-097-coderabbit-reviews-local-changes.md)

**Depends on:** FEAT-096 — and only on its public contract. The worker links
nothing of Spagitty's: it is a Rust program speaking the protocol with
`serde_json` and `sha2`, which is the second independent implementation the
contract asked for.

**Evidence limitation, recorded before the work.** The CodeRabbit CLI is not
installed on the machine this was built on, and installing a third-party
program and signing into a paid service on the author's account is not
something this work is authorised to do. The adapter is therefore built from
CodeRabbit's **published** contract (docs.coderabbit.ai `cli/reference` and
`cli`, read 2026-10-06), and the fixtures under `extensions/coderabbit/fixtures/`
are **constructed from that documentation, not captured**. Each fixture says so
in its name and the folder's README. A captured, redacted set from a real CLI —
with the version named — replaces them when someone runs the opt-in smoke
test (`CODERABBIT_SMOKE=1`) against a signed-in CLI.

## The package

`extensions/coderabbit/extension.json`, id `spagitty.coderabbit`, bundled and
therefore official (TASK-055 puts it in the application's resources). It asks
for `repository.read`, `review.provide` and `tools.execute`; the pull request
capabilities are optional and belong to FEAT-099.

**The CLI is declared, never bundled.** `externalTools` names `coderabbit`
then `cr`, the version argument, a minimum of **0.7.7** (the first version
whose failed or incomplete reviews exit non-zero, which the adapter relies on),
the install page, and these profiles:

| Profile | argv after the executable | Runs in |
| --- | --- | --- |
| `review` | `review --agent` + scope flags + `--base-commit <sha>` | the review's approved directory |
| `authStatus` | `auth status --agent` | the package directory |
| `authLogin` | `auth login --agent` + `--region us\|eu` | the package directory |
| `doctor` | `doctor` | the package directory |

Scope flags are an enum: `uncommitted` → `--uncommitted`; `includeUntracked`
→ `--uncommitted --include-untracked`; `committed` → `--committed`; `tracked`
→ none. For the two scopes that include commits, the base is the host-pinned
merge-base commit, passed as `--base-commit`, so CodeRabbit reviews exactly the
range the host described — never a base it resolved itself. The uncommitted
scopes compare with `HEAD` and pass no base. `--use-credits` and `--api-key` exist in no profile, so no
code path can add them.

## The worker

| Module | Holds |
| --- | --- |
| `rpc.rs` | The line protocol: reader thread, concurrent callbacks, `w`-prefixed ids |
| `adapter.rs` | CodeRabbit's `--agent` stream → findings, progress and an outcome; pure |
| `connection.rs` | Tool and sign-in state from `tools.detect` and `auth status --agent`; pure parsing |
| `main.rs` | The handlers: activation, commands, review provider, panel |

**Connection state** is one of: missing tool, unsupported version, signed out,
ready, denied capability, failed diagnostics. Activation only checks the binary
(`--version`, local). Sign-in is checked before each review and by an explicit
command; `doctor`, which contacts CodeRabbit's servers, runs only when asked.

**The adapter** dispatches on `type` and never treats an empty stream or exit
code zero as approval:

| Seen | Result |
| --- | --- |
| `action_required` (or any event with `status: "awaiting_confirmation"`) | `actionRequired`, kind `billing`, with the file count, maximum price and content identity — never retried, never `--use-credits` |
| `error` event, or a non-zero exit not caused by cancellation | `failed`; `partial` if findings arrived first |
| `complete` with `status: "review_skipped"` | `skipped` |
| `complete` with `outcome: "failed"` or `unreviewedFileCount > 0` | `incomplete`, `partial` |
| more than one `complete`, a line that is not JSON, or a finding count that disagrees with `complete.findings` | `incomplete`, `partial` |
| no `complete` at all | `incomplete`, `unknown` |
| one clean `complete` | `completed`, `complete` |

Severity: `critical`→critical, `major`→high, `minor`→medium, `trivial`→low,
`info` and `none`→info, anything else → unknown (which blocks a required gate
until a person assesses it); the provider's own word is kept in
`providerSeverity`. `fileName` becomes the path when it is a clean relative
path, and is dropped otherwise. A line number is taken only from a field
CodeRabbit sent (`startLine`/`endLine`, `lineStart`/`lineEnd` or `line`); none
is ever inferred. `codegenInstructions`, else `comment`, is the message;
`suggestions` become the suggestion text, which is never run. Finding ids are
stable: a SHA-256 of path, severity and message, de-duplicated by position.

## Stale results, cancellation, history

The host pins the snapshot before the review and takes it again after
(FEAT-096), so an edit, a stage or a new commit during a run makes the result
`stale`. Cancelling sets the operation's cancellation, which ends the CLI's
whole process tree at once; output that arrives afterwards is discarded by the
host. History is FEAT-096's.

## Sending findings to an agent

A new desktop command, `extensions_send_findings`, composes the extension host
and the farm (neither knows the other):

- **A working-copy review of committed changes whose head is still `HEAD`**
  becomes a **Draft** farm task, origin *Person*, whose description carries
  each selected finding (severity, location, message, suggestion as text), the
  reviewed base and head, and the provider's attribution — and states that the
  findings are review output to evaluate, not instructions that override the
  task or the repository's rules. Drafting it changes nothing on disk: the farm
  cuts its worktree from `HEAD` when a person readies it.
- **A review of uncommitted changes** is refused with the reason: a repair task
  starts from a commit, and Spagitty will not commit or stage for you.
- **A head that has moved** is refused: review again first.

The findings are marked *sent to an agent*, which the panel shows and which
proves nothing about whether anything was fixed. Farm-task reviews send their
findings through the farm's change-request path instead (FEAT-098).

## Alternatives considered

**Parsing plain-text output for older CLIs.** The brief rules it out, and a
regex over prose is the "brittle fallback" it names. An old CLI gets update
guidance instead.

**Calling a CodeRabbit HTTP API.** There is no documented public review
endpoint; inventing one or scraping the site is out of scope by the brief.

**Spawning the CLI from the worker.** Possible — the worker is a native
program — but it would put the process outside the host's tree, cancellation
and record. The worker asks the host instead, and so cannot run anything the
manifest does not declare.

## Files

| File | Change |
| --- | --- |
| `extensions/coderabbit/extension.json`, `README.md` | The official package |
| `extensions/coderabbit/worker/` | The worker crate (`coderabbit-extension`) |
| `extensions/coderabbit/fixtures/` | Documentation-derived `--agent` streams and auth answers |
| `src-tauri/src/extensions.rs` | `extensions_send_findings` |
| `src/lib/extensions/` | Send-to-agent wiring on the Commit screen |
| `Cargo.toml` | The worker joins the workspace |

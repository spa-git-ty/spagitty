<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Spagitty extension protocol, API version 1

This document is the standard. The TypeScript SDK in `packages/extension-sdk`
is one implementation of it and the official CodeRabbit worker is another; an
extension in any other language implements this page, not either of them.

- **API version:** `1.0.0`. The host accepts a worker that answers with any
  `1.x.y`. A worker that answers with another major version is stopped before
  it is given anything to do.
- **Manifest:** `manifest.v1.schema.json` beside this page.
- **Review model:** `review.v1.schema.json` beside this page.

## Trust

An extension is **trusted native software running with your operating-system
privileges.** Running it as a separate process means a crash cannot take
Spagitty down with it; capabilities mean Spagitty's own API will only do for an
extension what it was granted. Neither is a sandbox: a worker can read any file
your account can read and open its own network connections. Install only
extensions you would run from a terminal.

## Transport

- The host starts the entrypoint for the current target with **no arguments**,
  the package directory as its working directory, and the environment of the
  application plus `SPAGITTY_EXTENSION_ID` and `SPAGITTY_EXTENSION_API=1`.
- **stdout is protocol only.** One JSON-RPC 2.0 message per line: compact UTF-8
  JSON, no embedded newline, terminated by `\n` (a preceding `\r` is
  tolerated). Anything on stdout that is not a JSON-RPC message is a protocol
  violation and stops the worker.
- **stdin** carries the host's messages in the same framing.
- **stderr** is diagnostics. The host keeps the last 64 KiB, redacted, for the
  Diagnostics view. It is never parsed.
- A message may not exceed **1 MiB**. A longer line stops the worker.

## Messages

JSON-RPC 2.0 exactly: `{"jsonrpc":"2.0", ...}` with `method`/`params`/`id` for a
request, `method`/`params` without `id` for a notification, and `id` with
`result` or `error` for a response.

**Ids.** The host's request ids are integers. A worker's request ids are
strings beginning with `w` (`"w1"`, `"w2"`, …). A response is matched against
the requests *its receiver* sent, so the two sides never confuse each other's
ids, and either side may have many requests outstanding at once.

**Concurrency.** The host answers a worker's requests concurrently and keeps
reading while it does, so a worker may wait on a callback (for example a tool
run) while continuing to send notifications. A worker should do the same: keep
reading stdin while it waits on the host.

## Lifecycle

```text
host                                worker
 │── extension.initialize ──────────►│   negotiate; grants; limits; settings
 │◄──────────────────────── result ──│
 │── extension.activate ────────────►│   once, before the first operation
 │◄──────────────────────── result ──│
 │── command.execute / review.start / panel.resolve … (any order, concurrently)
 │── settings.changed (notification)
 │── operation.cancel (notification)
 │── extension.deactivate ──────────►│   then stdin closes
 │◄──────────────────────── result ──│   then the worker exits
```

### `extension.initialize` (request, host → worker)

```json
{"jsonrpc":"2.0","id":1,"method":"extension.initialize","params":{
  "apiVersions":["1.0.0"],
  "host":{"name":"Spagitty","version":"0.9.0"},
  "extension":{"id":"com.example.hello","version":"1.0.0"},
  "session":"s-8f3a…",
  "capabilities":["repository.read","ui.notify","storage"],
  "locale":"en-GB",
  "platform":"x86_64-pc-windows-msvc",
  "limits":{"maxMessageBytes":1048576,"maxConcurrentOperations":4,
            "inactivityMs":120000,"cancelGraceMs":5000},
  "settings":{"greeting":"Hello"}
}}
```

`capabilities` is what is **granted** for this session, which can be less than
the manifest asked for. The worker answers:

```json
{"jsonrpc":"2.0","id":1,"result":{"apiVersion":"1.0.0"}}
```

The handshake must complete within **10 seconds**.

### `extension.activate` (request)

`params`: `{"reason":"command"|"reviewProvider"|"panel"}`.
`result`: `{}` — or `{"unavailable":[{"id":"<contribution>","reason":"…"}]}`
to report contributions that cannot work right now (a missing tool, say). The
host shows the reason beside each.

### `extension.deactivate` (request)

`params`: `{}`. The worker releases whatever it holds and answers `{}`; the host
then closes stdin and waits up to the cancellation grace for the process to
exit before ending its process tree.

### `settings.changed` (notification)

`params`: `{"settings":{…}}` — the complete, validated settings object.

## Operations

Every long-running piece of work is an **operation** with an id the host
mints. A request that starts one is answered as soon as the worker has
accepted it; the outcome arrives later as exactly one `operation.complete`.

Every operation-scoped message carries `operationId`. Messages for an operation
the host does not know, or one that has already completed, are dropped and
logged.

### `command.execute` (request)

```json
{"jsonrpc":"2.0","id":7,"method":"command.execute","params":{
  "operationId":"op-12","command":"sayHello",
  "context":{"kind":"workingCopy","repository":"repo:1"}}}
```

`context.kind` is `global`, `workingCopy`, `farmTask` (adds `taskId`) or
`pullRequest` (adds `pullRequest`: `{"number":412,"headSha":"…"}`).
`repository` is an opaque handle, absent for `global` with no repository open.

`result`: `{"accepted":true}`.

### `review.start` (request)

```json
{"jsonrpc":"2.0","id":8,"method":"review.start","params":{
  "operationId":"op-13","provider":"review","reviewId":"rv-…",
  "repository":"repo:1",
  "snapshot":{ …ReviewSnapshot… },
  "workdir":"wd:1"}}
```

`snapshot` is the host's description of exactly what is to be reviewed (see the
review model). `workdir` is a handle for the directory a tool run for this
review must use. `result`: `{"accepted":true}`.

### `panel.resolve` (request)

`params`: `{"panel":"<id>","context":{…as for command.execute…}}`.
`result`: the panel's data for its renderer (see Panels). Not an operation:
answered directly, within 30 seconds.

### `operation.cancel` (notification, host → worker)

`params`: `{"operationId":"op-13"}`. The worker stops and sends
`operation.complete` with `status: "cancelled"`. After **5 seconds** the host
ends the operation as cancelled itself, kills any tool runs it owns, and — if
the worker has still not answered — ends the worker's process tree.

### `operation.progress` (notification, worker → host)

`params`: `{"operationId":"op-13","message":"Reviewing 12 files","phase":"review"}`.
Any progress or heartbeat resets the inactivity timer. Leaving out `message`
makes it a heartbeat.

### `review.findings` (notification, worker → host)

`params`: `{"operationId":"op-13","findings":[ …ReviewFinding without reviewId… ]}`.
Findings arrive in any number of batches. The host assigns `reviewId` and
rejects a finding whose `id` repeats one already received for the review.

### `operation.complete` (notification, worker → host)

The terminal message. Exactly one is accepted per operation.

```json
{"jsonrpc":"2.0","method":"operation.complete","params":{
  "operationId":"op-13",
  "status":"completed",
  "message":"Review completed with 3 findings",
  "review":{"status":"completed","completeness":"complete",
            "summary":"…","providerVersion":"0.8.1"}}}
```

`status` is `completed`, `failed` or `cancelled`. For a review, `review.status`
is one of the review model's run statuses and `review.completeness` one of
`complete`, `partial`, `unknown`. A provider cannot set a gate: the host
computes it.

`actionRequired` may be added to `review`:
`{"kind":"billing","message":"…","detail":{…}}` with
`review.status: "actionRequired"`.

### `log` (notification, worker → host)

`params`: `{"level":"debug"|"info"|"warn"|"error","message":"…"}`. Redacted
and kept with the diagnostics; at most 200 lines are retained.

## Host services (worker → host requests)

Every call is checked when it arrives: the extension must be active, the
capability granted for the repository the handle belongs to, the handle
current, and — where the call names one — the operation running and owned by
this extension. A refusal is a JSON-RPC error (codes below), never a silent
empty answer.

| Method | Capability | Params | Result |
| --- | --- | --- | --- |
| `repository.describe` | `repository.read` | `repository` | `{name, branch, head, remotes:[{name,forge}], forge}` |
| `repository.changes` | `repository.read` | `repository`, `snapshot` (id) | `{files:[{path, previousPath?, status}], truncated}` |
| `review.snapshot` | `review.provide` | `operationId` | the snapshot the review was started with |
| `tools.detect` | `tools.execute` | `tool` | `{found, path?, version?, compatible?, reason?}` |
| `tools.run` | `tools.execute` | `operationId?`, `tool`, `profile`, `options`, `workdir?` | `{exitCode, signalled, durationMs}` once the process ends; its stdout lines arrive first as `tools.output` |
| `forge.pullRequest.snapshot` | `forge.pullRequest.read` | `repository`, `number` | a pull request snapshot (FEAT-099), or error `-32010` unsupported |
| `forge.pullRequest.comment` | `forge.pullRequest.comment` | `repository`, `number`, `body` | `{posted:true, url?}`; the user is shown the exact body first and may refuse (`-32011`) |
| `storage.get` | — | `key` | `{value}` |
| `storage.set` | — | `key`, `value` (≤ 64 KiB) | `{}` |
| `ui.notify` | — | `level`, `message` | `{}` |

`tools.output` (notification, host → worker):
`{"runId":"…","operationId":"op-13","stream":"stdout","line":"…"}`. Lines are
delivered in order before the `tools.run` response. stderr of a tool is kept
by the host for diagnostics and not forwarded.

A tool run inherits its operation's cancellation: cancelling the operation ends
the tool's process tree.

## Error codes

| Code | Meaning |
| --- | --- |
| `-32700` `-32600` `-32601` `-32602` `-32603` | JSON-RPC's own |
| `-32001` | Capability not granted |
| `-32002` | Unknown or expired handle |
| `-32003` | Operation unknown, finished, or not this extension's |
| `-32004` | Extension not active |
| `-32005` | Tool, profile or option not declared, or an option value refused |
| `-32006` | Tool not found or not compatible |
| `-32007` | Limit exceeded (size, concurrency, rate) |
| `-32008` | Cancelled |
| `-32010` | Not supported for this repository's host |
| `-32011` | The user declined |
| `-32012` | Delivery uncertain — the request may have reached the service; refresh before trying again |

## Panels

A panel's data is drawn by the host. An extension supplies values, never markup.

**`summary`** — `{"title"?: string, "rows": [{"label","value"}], "text"?: markdown}`

**`reviewFindings`** — drawn from the host's own review history for the
extension's review provider; `panel.resolve` is not called.

**`reviewStatus`** —
```json
{"state":"notObserved"|"requested"|"running"|"completed"|"stale"|"unavailable",
 "headline":"…", "summary":"markdown", "revision":"<sha>",
 "items":[{"kind":"comment"|"finding"|"check","author":"…","authorType":"bot"|"user",
           "title":"…","body":"markdown","path"?:"…","line"?:12,"url"?:"…","state"?:"…"}],
 "links":[{"title":"…","url":"https://…"}],
 "complete": true}
```

Markdown is rendered with an allow-list: paragraphs, emphasis, code, lists,
quotes, headings and `https:` links. Raw HTML is never interpreted: comments
are dropped and tags removed, leaving their text. Images are not loaded; their
alt text stands in. A link is never opened by rendering it — choosing one
copies its address.

## Limits

| Limit | Value |
| --- | --- |
| Handshake | 10 s |
| Message | 1 MiB |
| stderr kept | 64 KiB |
| Concurrent operations per extension | 4 |
| Inactivity (no progress or heartbeat) | 120 s |
| Absolute operation deadline | 45 min, a setting |
| Cancellation grace | 5 s |
| Findings per review | 2000 |
| Storage value | 64 KiB, 256 keys |
| Concurrent host callbacks per worker | 8 |

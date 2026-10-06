<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Writing an extension in any language

The protocol, not the TypeScript SDK, is the standard. The official CodeRabbit
worker (`extensions/coderabbit/worker`) is a Rust program that implements it
with `serde_json` alone and links nothing of Spagitty's.

What a worker must do:

1. **Read stdin line by line.** Each line is one JSON-RPC 2.0 message. Keep
   reading while you work; the host may send a cancellation or answer your
   callback at any time.
2. **Write only protocol to stdout**, one compact JSON object per line,
   flushed. Write diagnostics to stderr. A message may not exceed 1 MiB.
3. **Answer `extension.initialize`** within ten seconds with
   `{"apiVersion": "1.0.0"}` (any 1.x you implement).
4. **Answer `extension.activate`** with `{}`, or with `unavailable` entries for
   contributions that cannot work right now.
5. **Accept operations** — `command.execute`, `review.start` — by answering
   `{"accepted": true}` at once, doing the work, and sending exactly one
   `operation.complete` notification for the `operationId` you were given.
6. **Number your own requests** with string ids beginning with `w`, and match
   the host's answers by id. Several may be in flight at once.
7. **On `operation.cancel`**, stop and send `operation.complete` with
   `status: "cancelled"` within five seconds.
8. **On `extension.deactivate`**, answer `{}`; then exit when stdin closes.

Treat `repository`, `workdir` and other handles as opaque strings: pass them
back, never parse them. Treat every string the host gives you as data.

A worker in another language can still use the TypeScript fake host to test
itself: `FakeHost.spawn(['./my-worker'])` from a Bun script, or
`bun run ext test <dir>` if the folder has a `src/main.ts` that execs it.

The protocol's full text is
[`schemas/extensions/protocol.v1.md`](../../schemas/extensions/protocol.v1.md).

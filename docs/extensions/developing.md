<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Writing an extension

The kit lives in this repository: `packages/extension-sdk` (TypeScript SDK and
fake host) and `tools/extensions/cli.ts` (run as `bun run ext`). Nothing is
published to a registry; a scaffolded extension depends on the SDK by path.
You need [Bun](https://bun.sh) 1.1 or newer to use the kit. **People who
install your extension need nothing** — the package carries a compiled program.

## The path, end to end

```sh
bun run ext create com.example.hello hello    # 1. scaffold
cd hello && bun install && bun test           # 2. your tests
bun run ext test .                            # 3. protocol conformance
bun run ext dev .                             # 4. try it in Spagitty
bun run ext pack . --build                    # 5. build and package
bun run ext inspect dist/com.example.hello-0.1.0.spagitty-extension
```

Then **Settings › Extensions › Install from file…** on any Spagitty build that
supports extension API 1. `examples/extensions/hello` is exactly this path,
committed, and its package is installed and run by the host's own test
(`crates/spagitty-extensions/tests/foreign_package.rs`).

## The scaffold

```text
extension.json      the manifest
src/main.ts         the worker
test/               tests against the SDK's fake host
package.json        bun scripts, the SDK by path
```

## The manifest

`extension.json` is validated against
[`manifest.v1.schema.json`](../../schemas/extensions/manifest.v1.schema.json)
with the same rules Spagitty uses. `bun run ext validate .` reports every
problem at once, each with the field it is about.

- **`id`** — reverse-DNS, two to five lowercase labels (`com.example.hello`).
  Contribution ids are qualified with it, so `hello` in your manifest is
  `com.example.hello/hello` in Spagitty.
- **`engines`** — `spagitty` and `extensionApi` version requirements in the
  Cargo dialect (`>=0.9.0, <1.0.0`; a space instead of the comma is accepted).
- **`runtime.entrypoints`** — one program per target triple. Listing a target
  claims the package runs there; `ext pack` drops targets it did not build.
- **`capabilities`** — `required` ones are granted when the user turns the
  extension on; `optional` ones the user turns on separately. Unknown names are
  errors.
- **`externalTools`** — programs your extension asks Spagitty to run. You never
  pass a command line: you declare executable names (looked up on `PATH`, or
  chosen by the user), and **profiles** — fixed arguments plus typed options
  (`enum` values that map to fixed flags; `revision` and `commit` values that
  are validated before they reach argv; options are appended in the order of
  their names). Spagitty runs the tool in the
  directory it approved for the operation and ends its whole process tree on
  cancellation.
- **`contributes`** — `commands` (with a `context` and finite `when`
  conditions), `reviewProviders`, `panels` (drawn by one of three host
  renderers: `reviewFindings`, `reviewStatus`, `summary`) and `settings`
  (`boolean`, `enum`, `text`, `number`, `executable`). Secrets are not
  settings.

## The worker

```ts
import { defineExtension, run } from '@spagitty/extension-sdk';

export const extension = defineExtension({
	commands: {
		async hello(ctx) {
			const repo = await ctx.host.describeRepository(ctx.context.repository!);
			await ctx.host.notify(`Hello from ${repo.branch}`);
			return { message: 'Said hello' };
		}
	},
	reviewProviders: {
		async review(ctx) {
			ctx.progress('Reviewing');
			const result = await ctx.host.runTool(
				{ operationId: ctx.operationId, tool: 'lint', profile: 'check', options: { scope: 'all' }, workdir: ctx.workdir },
				(line) => { /* parse each line; call ctx.findings([...]) */ }
			);
			return { status: result.exitCode === 0 ? 'completed' : 'failed', completeness: 'complete', providerVersion: '1.0.0' };
		}
	},
	panels: {
		about: () => ({ title: 'Hello', rows: [{ label: 'Version', value: '1.0.0' }] })
	}
});

if (import.meta.main) run(extension);
```

What the SDK guarantees: one `operation.complete` per operation whatever your
handler does (a thrown error is `failed`, an aborted one `cancelled`);
`ctx.signal` fires when Spagitty asks you to stop — stop within five seconds or
your process tree is ended; `console.log` goes to stderr, because stdout is the
protocol's alone.

What the host guarantees: it computes the review's snapshot and identity before
you start, decides the gate itself from your result (you cannot pass a gate by
saying so), drops findings with a repeated id or a path outside the repository,
never invents a line number you did not send, and renders your markdown as text
— no HTML, no images loaded, links copied rather than opened.

## Testing

`@spagitty/extension-sdk/testing` exports `FakeHost`. `FakeHost.inMemory(...)`
runs your definition in-process for fast tests; `FakeHost.spawn([...])` runs
your real program. Both record `violations` — non-protocol output, a request id
that does not start with `w`, a second completion, findings after completion.
`bun run ext test .` runs your tests and then a conformance pass against your
real worker: handshake, activation, deactivation and exit when stdin closes.

## Developing against a running Spagitty

`bun run ext dev .` leaves a request in Spagitty's data directory; the next
time Settings › Extensions is opened, the folder is attached as a Development
extension (or use **Attach folder…** there). Spagitty runs the program at the
folder's entrypoint for this computer, so build first: `bun run ext build .`.
Detaching leaves the folder untouched. A development folder cannot take the
identity of an installed or official extension.

## Packaging

`bun run ext pack . --build [--target <triple>]…` compiles `src/main.ts` with
`bun build --compile` for each target asked (this computer's by default) and
writes `dist/<id>-<version>.spagitty-extension`: a ZIP with the manifest,
`integrity.json` (SHA-256 of every file), the programs, and everything else in
the folder except sources, tests and tooling. **Checksums prove the files are
the ones you packed; they say nothing about who you are.**

Tested targets for compiled Bun workers: `x86_64-pc-windows-msvc` (built and
run on Windows 11, 2026-10-06). The other targets Bun names
(`bun-linux-x64`, `bun-linux-arm64`, `bun-darwin-x64`, `bun-darwin-arm64`) are
built by the same command but were not run as part of this work; a compiled
Bun program is around 90 MB. Windows on Arm has no Bun target.

## Versioning

The manifest version, the extension API version and the application version
move independently. Spagitty refuses a package whose `engines` it does not
satisfy, and refuses a worker whose handshake answers another API major
version. Additive protocol fields may appear within API 1; ignore the ones you
do not know.

## Persisted review gates and forge receipts

Implement the optional SDK `checkReview` handler (protocol `review.check`) if your
provider's saved results can be used by Required farm policy. Return explicit
`ready`, a reason when unavailable, and the same provider version as your review.
This is a local availability/authentication check, not a new uploaded review.
Unsupported or unprovable readiness blocks a required gate after restart.

The SDK's PR snapshot preserves platform actor/app IDs, revision attribution,
nullable resolution and per-list completeness. PR comment callbacks return
`status: posted | uncertain`, never a review approval. Pass your operation ID
to `commentOnPullRequest` so cancellation also covers the host confirmation.
Store uncertainty before sending, including crash recovery, and never retry an
ambiguous write automatically. The CodeRabbit worker demonstrates this path.

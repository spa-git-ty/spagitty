<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Extensions

Spagitty runs extensions: separate programs that add commands, actions,
settings, panels and review providers to the application through a published,
versioned contract (FEAT-096).

| Document | For |
| --- | --- |
| [Using extensions](using.md) | Installing, turning on, configuring, updating and removing them |
| [Writing an extension](developing.md) | The SDK, the `ext` tool, testing and packaging |
| [Any other language](other-languages.md) | Implementing the protocol without the TypeScript SDK |
| [Protocol](../../schemas/extensions/protocol.v1.md) | The standard: messages, methods, limits, error codes |
| [Manifest schema](../../schemas/extensions/manifest.v1.schema.json) | `extension.json`, authoritatively |
| [Review model](../../schemas/extensions/review.v1.schema.json) | Snapshots, findings, results |

**Versions supported by this build:** manifest version 1, extension API
1.x (1.0.0 implemented).

## The trust model, plainly

An extension is a program that runs on your computer **with your
permissions**. Running it in its own process means a crash cannot take
Spagitty down with it. Capabilities mean Spagitty's own API will only do for
it what you granted — read this repository, run the tools it declared, post a
comment you approved. **Neither is a sandbox.** An extension can read any file
you can and open its own network connections. Install only extensions you
would be willing to run from a terminal.

The official CodeRabbit extension is trusted because it ships inside
Spagitty's own release, not because of anything its manifest says; an
extension installed from a file never gets the Official badge.

<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Hello — the example extension

A Spagitty extension (`com.example.hello`).

```sh
bun install
bun test                       # your tests
bun run ext test .             # protocol conformance against a fake host
bun run ext dev .              # attach this folder to a running Spagitty
bun run ext pack . --build     # build this computer's program and package it
```

Extensions are programs that run on the user's computer with the user's
permissions. Spagitty only does for an extension what it is granted, but it is
not a sandbox — say plainly what yours does.

This example is independent of Spagitty's source: it uses only the public
schema, the SDK and the `ext` tool, and runs on an unchanged build.

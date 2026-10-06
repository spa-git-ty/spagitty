# {{name}}

A Spagitty extension (`{{id}}`).

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

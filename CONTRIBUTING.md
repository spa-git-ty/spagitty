# Contributing to Spagitty

Thanks for wanting to help.

## License of contributions

Spagitty is licensed **GPL-3.0-or-later**. By contributing, you agree that your
contribution is licensed under GPL-3.0-or-later. There is no CLA and no
copyright assignment — you keep the copyright to your work.

Every new source file gets an SPDX header as its first line:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

```svelte
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
```

## Developer Certificate of Origin

We use the [DCO](https://developercertificate.org/) instead of a CLA. Sign off
every commit:

```sh
git commit -s
```

That appends a line to your commit message:

```
Signed-off-by: Your Name <you@example.com>
```

The name and email must be real and must match the commit author. Signing off
certifies the DCO, reproduced in full:

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

## Dependencies

Before adding a dependency, check its license. Acceptable: MIT, Apache-2.0,
BSD, ISC, MPL-2.0, LGPL, GPL-2.0-with-linking-exception, CC0, OFL (fonts).

**Not acceptable:** proprietary, SSPL, BSL, or any "source available"
non-free license. Apache-2.0-only code must not be linked into GPL-2.0-only
code.

New dependencies must show up in the generated `THIRD-PARTY-LICENSES.md`.

## Design rules that are not negotiable

These come from the design handoff and hold across every screen:

- **Standard git terminology only.** fetch, push, stage, hunk, `stash@{n}`,
  pick/squash/reword/drop, ours/theirs. No invented vocabulary, no friendly
  renaming of git concepts.
- **Structural numbers are shared constants, never literals.** The 26px commit
  row pitch, 186px rail, 270px detail panel, and 24px lane pitch each have
  exactly one definition (`src/lib/metrics.ts`, mirrored in
  `crates/spagitty-core/src/graph.rs`). If you find yourself typing `26`, use the
  constant.
- **Never block on walking the whole history.** The graph streams in windows and
  paints progressively.
- **Destructive actions are reversible.** Rebase previews before executing;
  conflict resolution writes nothing until finished; Undo maps to reflog
  restore.
- **Nothing leaves the machine unless you ask.** Repositories are read straight
  from disk; credentials go in the OS keychain; the one HTTP client talks only to
  forges you connected. An extension you turn on is a program with your
  permissions and can talk to services of its own — one that sends code
  somewhere says where, and asks before it is turned on (FEAT-096).

## Code style

- Rust: `cargo fmt` and `cargo clippy` clean.
- Frontend: `bun run check` (svelte-check) clean.
- Don't reimplement in Rust what `shell.rs` should shell out to `git` for
  (interactive rebase execution, hooks, LFS, submodule recursion, credential
  helpers). If you need a new one of those, add it to `shell.rs` with a comment
  explaining why gix isn't the right tool.

<!--
  Spagitty — a local-first desktop Git client.
  Copyright (C) 2026 The Spagitty Authors
  Licensed under the GNU General Public License v3.0 or later. See LICENSE.
-->

<div align="center">

<img src="assets/brand/brand-mark.png" alt="Spagitty" width="128" height="128">

# Spagitty

**A local-first desktop Git client for repositories where people and coding agents work side by side.**

[![Release](https://img.shields.io/github/v/release/spa-git-ty/spagitty?sort=semver&label=release&color=EEB04D)](https://github.com/spa-git-ty/spagitty/releases/latest)
[![Gates](https://github.com/spa-git-ty/spagitty/actions/workflows/gates.yml/badge.svg)](https://github.com/spa-git-ty/spagitty/actions/workflows/gates.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

[Download](https://github.com/spa-git-ty/spagitty/releases/latest) · [Screens](docs/screens.md) · [Architecture](docs/architecture.md) · [Changelog](CHANGELOG.md)

</div>

---

Spagitty is a desktop Git client built with Tauri, Rust and Svelte. It covers
the everyday work of a Git client: history, diffs, staging, rebasing, conflicts
and pull requests. It can also run the coding agents already installed on your
machine against a goal: one task per agent, each on its own branch and worktree.

Spagitty ships no model and no API key, and it has no account or server of its
own. Repositories are read from disk. The network is used only for actions you
trigger, against the services you connected.

## Features

- **Commit graph.** Branch lanes and tags, with hide, solo and pin so large
  histories stay readable.
- **Working copy.** Stage and unstage by file, hunk or line. Discard shows exactly
  what will be lost.
- **Diffs.** Syntax highlighting, image comparison, binary size deltas, and
  search inside patches.
- **History.** File history that follows renames, line-by-line blame, and log
  search by author, message, path, date and diff content.
- **Merger.** Preview any merge, squash, rebase or fast-forward before it is
  written, including where it would conflict. Resolve conflicts side by side,
  and step through a rebase one commit at a time.
- **Pull requests.** Read, review, comment on, create and merge pull requests on
  GitHub, GitLab and Bitbucket Cloud. The Review screen sorts your inbox by
  what each pull request needs from you.
- **Everyday Git.** Branches, tags, stashes, reflog, worktrees, submodules,
  remotes, clone, fetch, pull and push. The git command behind each action can
  be shown.
- **Identity and signing.** Named profiles for author identity and signing key.
  Signing status is read from your Git configuration.
- **Agent farm.** Split a goal into tasks and run them in parallel with Claude
  Code, Codex, Cursor or any command-line agent. Each task gets its own
  worktree. Your repository's checks and a review by a second agent are
  required before a merge.
- **Extensions.** Out-of-process extensions add commands, panels and reviews,
  with permissions you approve. The bundled CodeRabbit extension reviews
  changes with your own CodeRabbit CLI.

A tour of every screen is in [`docs/screens.md`](docs/screens.md).

## The agent farm

The farm runs coding agents you already have, as your own user. It is a
supervisor, not a model.

- **Isolated work.** Each task runs on its own branch (`spagitty-farm/<task>/<provider>`)
  in its own worktree. Your working copy is never touched, and deleting a task
  keeps its commits.
- **Verification you control.** A task is done only after your repository's own
  commands pass and a different agent has reviewed the change. No agent can mark
  its own work done.
- **Autonomy levels.** Five levels, from *Manual*, where nothing runs on its
  own, to *Unattended*.
- **Dependencies.** Tasks form a dependency graph. Up to four run at once.
- **Repository rules.** `AGENTS.md` is attached to every prompt.
- **Crash-safe state.** Farm state lives under `.spagitty/` and is never
  committed: atomically written JSON plus an append-only event log.

The orchestration lives in [`crates/spagitty-farm`](crates/spagitty-farm). The
design is recorded in [`FEAT-073`](agile/items/FEAT-073-agent-farm.md).

## Install

Download a build from the [latest release](https://github.com/spa-git-ty/spagitty/releases/latest).

| Platform | Package |
| --- | --- |
| Linux | AppImage, `.deb`, `.rpm` |
| Windows | Installer (`.exe`, `.msi`) or a portable `.exe` |
| macOS | `.dmg` for Apple silicon and Intel, on the [pre-release builds](https://github.com/spa-git-ty/spagitty/releases) |

The macOS builds are signed ad hoc and are not notarized, so Gatekeeper asks
before the first launch. [`docs/BUILD_MACOS.md`](docs/BUILD_MACOS.md) explains
how to open the app, and how to build it yourself.

Spagitty follows [Semantic Versioning](https://semver.org). Release notes are
in [`CHANGELOG.md`](CHANGELOG.md).

## Building from source

Requirements: Rust 1.77+, Bun 1.4, and the platform WebView (WebKitGTK on
Linux).

```sh
git clone https://github.com/spa-git-ty/spagitty.git
cd spagitty
bun install
bun run tauri dev      # run the desktop app
```

Checks run in CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run check          # type checks
bun run coverage       # frontend tests with the coverage floor
```

Before plain `cargo` builds of `src-tauri`, stage the bundled extension with
`bun tools/extensions/bundle.ts --debug`. `bun run tauri dev` and
`bun run tauri build` do this for you.

## Architecture

```
src/                     SvelteKit UI, one store per screen
  └─ invoke ──────────► src-tauri/              Tauri commands and workers
                          ├─ crates/spagitty-core/        Git, via gitoxide and git
                          ├─ crates/spagitty-farm/        agent orchestration
                          └─ crates/spagitty-extensions/  extension host
```

Each layer depends only on the layers below it. Only `src/lib/api.ts` calls the
backend, and `spagitty-core` never imports Tauri, so the core can be tested
without a window. Details are in [`docs/architecture.md`](docs/architecture.md).

## Privacy

- No telemetry, no analytics, and no Spagitty account or server.
- Forge tokens are stored in the OS keychain and never reach the web view.
- Forge requests go only to the accounts you connected, and only when you act.
- Extensions run as your user and may use the network. Review their permissions
  before you enable one.
- CodeRabbit sends the selected code to its service, through your own CLI and
  account, and only after you consent for that repository.

## Documentation

| Document | Contents |
| --- | --- |
| [`docs/architecture.md`](docs/architecture.md) | Layers, boundaries and data flow |
| [`docs/screens.md`](docs/screens.md) | Every screen and what it is for |
| [`docs/extensions/`](docs/extensions/) | Using and writing extensions |
| [`docs/testing.md`](docs/testing.md) | Test strategy and coverage |
| [`docs/ci.md`](docs/ci.md) | CI gates and release lanes |
| [`docs/BUILD_MACOS.md`](docs/BUILD_MACOS.md) | macOS builds and signing |
| [`docs/branding.md`](docs/branding.md) | Name, mark and palette |
| [`agile/`](agile/) | Work items, plans and test records |

## Contributing

Bug reports and pull requests are welcome. Please read
[`CONTRIBUTING.md`](CONTRIBUTING.md) first. In short:

- Sign off your commits (DCO).
- Reference a work item from [`agile/`](agile/) in the branch name.
- Add a changelog entry under `Unreleased` in the same change.

## License

Spagitty is licensed under the [GNU General Public License v3.0 or later](LICENSE).
See [`NOTICE`](NOTICE) for third-party attributions.

The Sora typeface is licensed under the SIL Open Font License 1.1. Spagitty is
not affiliated with the Git project. Git and the Git logo are trademarks of
Software Freedom Conservancy.

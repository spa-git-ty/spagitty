<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Testing

How Spagitty is checked: what runs headless, how the app is driven for a visual
sweep, and how the fixture repository the sweeps assume is built.

## Headless

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run check          # svelte-kit sync && svelte-check
bun run test           # vitest, run through bun
bun run coverage       # vitest with the coverage floor enforced
```

Coverage of first-party code is measured on both sides and both must clear their
floor — 70% for Rust, 65% for the frontend:

```sh
cargo llvm-cov --workspace --ignore-filename-regex '(fixture|testing)\.rs' --summary-only
bun run coverage
```

What counts, what does not, and which gate runs each command is in
[ci.md](ci.md).

## Fixtures in the Rust tests

Anything that reads a repository is tested against a real one.
`crates/spagitty-core/src/fixture.rs` builds them with the `git` binary — not
with `gix`, because a fixture built by the library under test would agree with
it by construction. `Fixture::empty()` is an initialised repository with no
commits, `Fixture::woven()` the standard history below with a clean working
copy, and `Fixture::dirty()` the same with work in progress. Each lives in a
temporary directory that is removed with it.

## Fixtures in the Tauri layer's tests

`src-tauri` tests the graph worker, the filesystem watcher's debounce and the
session against a real repository too. Two things make that possible.

Everything in `src-tauri` that touches the application is generic over
`R: tauri::Runtime` (TASK-003), so `tauri::test::mock_app` can supply an
`AppHandle<MockRuntime>` where the real one supplies a Wry handle. And
`spagitty-core`'s fixtures are `pub` behind a `fixture` feature, which
`src-tauri` enables as a dev-dependency — one fixture builder for the workspace
rather than two things building repositories with the `git` binary. Nothing
enables the feature outside tests, so a release build carries neither the
fixtures nor `tempfile`.

`src-tauri/src/testing.rs` holds what an event test needs beyond that: a mock
application managing the real `AppState`, an `Emitted<T>` collector with
`at_least(n)` and `no_more_than(n)`, and `finishes_promptly`, which puts a
deadline on a drop. The last one matters — dropping a worker joins its thread,
so a worker that ignored its shutdown would hang the runner rather than fail.

It also points `HOME` and the platform's other configuration variables at a
temporary directory, once for the whole test binary. `open_repo` writes to the
recent-repositories list, and a test run must not add rows to the list of
repositories you have opened.

## The one test that crosses the language boundary

`ROW_PITCH` exists twice: in `src/lib/metrics.ts`, which is the source of truth,
and in `crates/spagitty-core/src/graph.rs`, which describes lane elbows in row
units and so needs the same number. Two copies of a constant in two languages
drift, and in FEAT-029 they did — the geometry moved on one side while doc
comments on both sides claimed a test was holding them together. There was no
such test.

`graph::tests::row_pitch_matches_the_frontend` is that test. It resolves
`src/lib/metrics.ts` from `CARGO_MANIFEST_DIR`, reads the `export const
ROW_PITCH` declaration out of the file, and asserts it against the crate's
constant, failing with both values named so the message says which side moved.

It is the only test here that reads another language's source, and it earns that
by being the only constant duplicated across the boundary. Anything else needing
to cross should go through the existing IPC types rather than growing a second
file-reading test.

## Fixtures for the frontend tests

Components are mounted for real against happy-dom; the helpers are in
`src/testing/`. `mount.ts` renders a component and returns query handles;
`graph-store.svelte.ts` and `repo-store.svelte.ts` are reactive stand-ins for
the two stores the chrome reads. They hold `$state` rather than plain fields
on purpose — a stub built from ordinary properties renders once and never
updates, so every test about a change would pass by rendering the initial
value.

## The fixture repository

The manual sweeps in `agile/testing/*-sweep.md` assume a repository with a merge,
a branch that is merged and one that is not, tags of both kinds, stash entries,
a binary file, a dotfile, a deeply nested path, and a dirty working copy with
both staged and unstaged changes.

Build it in a scratch directory — **never inside this repository**:

```sh
FIX=/tmp/spagitty-fixture
mkdir -p "$FIX" && cd "$FIX"
git init -q -b main
git config user.name "Ada Lovelace"
git config user.email ada@example.com
git config commit.gpgsign false

seq 1 40 | sed 's/^/line /' > core.txt
printf 'alpha\nbeta\ngamma\n' > notes.md
printf '# fixture\n' > README.md
mkdir -p src/deep/nested
printf 'fn main() {}\n' > src/deep/nested/main.rs
printf '\x00\x01\x02binary\x00data\n' > logo.bin
printf '.cache/\n' > .gitignore
git add -A && git commit -q -m "Initial import"

for i in 1 2 3 4 5; do
  printf 'entry %s\n' "$i" >> notes.md
  git commit -q -am "Add note $i"
done

git switch -q -c feature/split-view
sed -i 's/^line 3$/LINE THREE/' core.txt
git commit -q -am "Rewrite line 3 in core"
printf 'split view work\n' > split.txt
git add split.txt && git commit -q -m "Start the split view"

git switch -q main
sed -i 's/^line 38$/LINE THIRTY-EIGHT/' core.txt
git commit -q -am "Rewrite line 38 in core"
git merge -q --no-ff feature/split-view -m "Merge feature/split-view"

git switch -q -c chore/tooling main
printf 'tooling\n' > tools.txt
git add tools.txt && git commit -q -m "Add a tooling note"

git switch -q main
git branch merged/already-in-main main~1
git tag -a v0.1.0 -m "First tag" main~2      # annotated
git tag v0.2.0                                # lightweight

printf 'work in progress\n' >> notes.md
git stash push -q -u -m "wip on notes"
printf 'second stash\n' >> README.md
git stash push -q -m "wip on readme"

printf 'uncommitted change\n' >> core.txt
printf 'brand new file\n' > untracked.txt
printf 'staged change\n' >> notes.md
git add notes.md
```

That leaves 11 commits, 4 local branches, 2 tags, 2 stash entries, one staged
file, one unstaged file and one untracked file.

### A conflicted fixture

The Conflicts screen needs a repository stopped mid-merge, which is a state the
other screens should not be tested against. Build it separately:

```sh
CFL=/tmp/spagitty-conflict
mkdir -p "$CFL" && cd "$CFL"
git init -q -b main
git config user.name "Ada Lovelace"
git config user.email ada@example.com
printf 'one\ntwo\nthree\n' > shared.txt
git add -A && git commit -q -m "Base"

git switch -q -c theirs
printf 'one\nTHEIRS\nthree\n' > shared.txt
git commit -q -am "Their change"

git switch -q main
printf 'one\nOURS\nthree\n' > shared.txt
git commit -q -am "Our change"

git merge theirs || true    # stops with a conflict, which is the point
```

## Visual sweep

The application is driven for real, not screenshotted from a mock.

```sh
bun run tauri dev -- -- -- /tmp/spagitty-fixture
```

The trailing path is passed to the binary and read by the `launch_path`
command, so the app opens straight onto the fixture with no dialog.

`WEBKIT_DISABLE_DMABUF_RENDERER` is left to the environment (FEAT-055): the
binary used to set it to `1` on Linux, which cost every host the GPU path, and
now it sets nothing. If the webview comes up blank with `Failed to create GBM
buffer` on stderr, export `WEBKIT_DISABLE_DMABUF_RENDERER=1` before launching —
that is the BUG-004 workaround, now opt-in. The binary does set
`WEBKIT_FORCE_COMPOSITING_MODE=1` unless the environment already has an opinion.

### Driving it without a desk

A human tester works the window directly. An automated sweep on a Wayland
session cannot, for two reasons that both look like bugs in the application and
are not:

- **The pointer cannot be moved.** XWayland refuses `XWarpPointer`, so
  `xdotool mousemove` silently does nothing and every click lands wherever the
  real pointer happens to be.
- **An unfocused window never repaints.** The compositor only paints what it
  shows, so `import -window` returns whatever was in the backing store — usually
  the first frame, before the stylesheet's custom properties were applied. The
  capture looks like a broken layout and is a stale image of a correct one.

A plain X server has neither problem. Run the app against `Xvfb`, which is also
the polite option — nothing steals focus from whatever is on screen:

```sh
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp &

env -u WAYLAND_DISPLAY DISPLAY=:99 GDK_BACKEND=x11 \
bun run tauri dev -- -- -- /tmp/spagitty-fixture
```

`-u WAYLAND_DISPLAY` is the part that is easy to miss: with it set, GTK prefers
the Wayland backend and connects to the real session regardless of `DISPLAY`,
and the window opens on the desktop instead of the virtual screen.

Then `xdotool mousemove`, `xdotool click`, `xdotool type` and
`import -window` all behave, against `DISPLAY=:99`.

Findings go into the item's `agile/testing/<ID>-sweep.md` as filled-in results,
not into a commit message.

## Development stylesheet regression

`bun run test tools/dev-styles.test.ts` checks the installed Vite/Svelte pipeline
against a stylesheet request arriving before its component (BUG-028). Development
injects scoped styles with component JavaScript through `emitCss: false`;
production continues extracting CSS assets. This avoids the v5 plugin's cold-cache
fallback serving Svelte source as CSS. Build success alone does not test this
development request order, and this headless test does not replace a visual sweep.

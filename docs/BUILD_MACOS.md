<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Building Spagitty for macOS

Two ways, one result: a `.dmg` for Apple silicon, Intel, or both, **ad-hoc
signed** — a real signature over the app's bytes, with no Apple identity behind
it. No Apple Developer account and no secrets are needed. It is **not
notarized**: the first time it is opened, macOS asks for it to be allowed once
(see [Opening it](#opening-it-on-a-mac)).

A Mac app cannot be built on Windows or Linux — `codesign`, `hdiutil` and
Apple's SDK exist only on macOS — so from those, use GitHub.

## A. On GitHub (recommended)

The **draft release** workflow, [`.github/workflows/draft-release.yml`](../.github/workflows/draft-release.yml),
builds both Mac architectures on GitHub's Mac runners (and Linux and Windows
beside them), signs them ad-hoc, checks them, and leaves a **draft** release.
It creates no tag and publishes nothing.

### 1. Start it

Commit, then push the commit you want built to any branch under `draft/`:

```bash
git push origin HEAD:draft/macos
```

Pushing to the same `draft/…` branch again builds again, and cancels a run of
that branch still going.

Or from the website: **Actions** → **draft release** → **Run workflow** →
choose the branch under *Use workflow from* → **Run workflow**.

### 2. Watch it

<https://github.com/spa-git-ty/spagitty/actions/workflows/draft-release.yml> —
the newest run, named after your branch. Each Mac job notes what it checked,
for example `Spagitty_0.8.1_aarch64.dmg: container verified, signature ad-hoc,
architecture arm64, notarized=false`.

### 3. Download it

Either:

- the run's page → **Artifacts** → `draft-macos-arm64` (Apple silicon) or
  `draft-macos-x86_64` (Intel). Each is a zip holding the `.dmg` and its
  `SHA256SUMS-macos-*.txt`. From a terminal:

  ```bash
  gh run download --repo spa-git-ty/spagitty -n draft-macos-arm64
  ```

- or **Releases** → the draft **Spagitty v`<version>`-alpha.`<n>`**, which
  carries every platform's files:
  `Spagitty_<version>_aarch64-macos-arm64.dmg` and
  `Spagitty_<version>_x64-macos-x86_64.dmg`. A draft is visible only to the
  repository's maintainers until someone presses **Publish release**.

`<version>` is the one in `src-tauri/tauri.conf.json`. Releases that go to
everyone are cut elsewhere: `dev` publishes alphas signed the same way, and
`main`'s release gate wants a notarizing Apple certificate and fails without
one — [`docs/ci.md`](ci.md).

## B. On a Mac

Once per Mac:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://bun.sh/install | bash -s "bun-v1.4.0"
```

Open a new terminal so `cargo` and `bun` are on the `PATH`, then:

```bash
git clone https://github.com/spa-git-ty/spagitty.git
cd spagitty
bun install --frozen-lockfile
chmod +x scripts/build-macos.sh
./scripts/build-macos.sh arm64
```

`intel` builds for Intel Macs and `all` builds both; either works on either
kind of Mac. The script adds the Rust target it needs, builds, checks, and
ends by printing the paths:

```text
target/aarch64-apple-darwin/release/bundle/dmg/Spagitty_<version>_aarch64.dmg
target/x86_64-apple-darwin/release/bundle/dmg/Spagitty_<version>_x64.dmg
```

## How the signature is checked

Both ways run the same checks, and stop with an error before a `.dmg` is
reported or uploaded if any fails:

| Check | What it answers |
| --- | --- |
| `codesign --verify --deep --strict --verbose=2 Spagitty.app` | Does the signature match every byte of the app, nested code included? |
| `codesign -dv --verbose=4 Spagitty.app` | What signed it — `Signature=adhoc` here. Printed for the record. |
| `file` on the app's executable | Is it the architecture its name says? |
| `hdiutil verify Spagitty_….dmg` | Is the disk image intact? |

On GitHub that is [`.github/actions/macos-verify`](../.github/actions/macos-verify/action.yml),
which also mounts the image first. It runs `spctl` too and prints what
Gatekeeper says, but an ad-hoc build is *expected* to be turned down there, so
that verdict is not a failure.

## Opening it on a Mac

The first open says Apple cannot check the app. Allow it once: **System
Settings → Privacy & Security → Open Anyway** (or right-click the app →
**Open**). macOS remembers.

If macOS instead calls the app **damaged**, the download is not the file that
was built: compare it with `SHA256SUMS-macos-*.txt` before anything else.
Don't run `xattr -cr` or turn Gatekeeper off — that hides the problem rather
than fixing it. A Mac managed by an organisation may not allow unnotarized
apps at all.

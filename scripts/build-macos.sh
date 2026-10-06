#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Build Spagitty's macOS .dmg on a Mac, ad-hoc signed, and check it (TASK-054).
#
#   ./scripts/build-macos.sh arm64   Apple silicon  (aarch64-apple-darwin)
#   ./scripts/build-macos.sh intel   Intel          (x86_64-apple-darwin)
#   ./scripts/build-macos.sh all     both
#
# The signing and the checks are the draft-release workflow's
# (.github/actions/macos-signing, .github/actions/macos-verify): an ad-hoc
# signature, `APPLE_SIGNING_IDENTITY=-` — a real seal over the app's bytes with
# no identity behind it. It is not notarization, and macOS will ask the person
# who opens the app to allow it once. See docs/BUILD_MACOS.md.
#
# Either architecture builds on either kind of Mac: Apple's toolchain
# cross-compiles between the two.

set -euo pipefail

if [ "$(uname -s)" != "Darwin" ]; then
  echo "build-macos.sh needs a Mac: codesign, hdiutil and Apple's SDK exist only on macOS." >&2
  echo "From any other machine, build it on GitHub instead: see docs/BUILD_MACOS.md." >&2
  exit 1
fi

case "${1:-}" in
  arm64) targets=(aarch64-apple-darwin) ;;
  intel) targets=(x86_64-apple-darwin) ;;
  all) targets=(aarch64-apple-darwin x86_64-apple-darwin) ;;
  *)
    echo "usage: $0 arm64|intel|all" >&2
    exit 2
    ;;
esac

for tool in bun rustup cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is not installed; docs/BUILD_MACOS.md says how to get it." >&2
    exit 1
  fi
done

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

# The Rust targets, added only when missing.
installed=$(rustup target list --installed)
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "==> Adding the Rust target $target"
    rustup target add "$target"
  fi
done

if [ ! -d node_modules ]; then
  echo "==> Installing dependencies"
  bun install --frozen-lockfile
fi

# `-` is codesign's spelling of an ad-hoc signature. Tauri's bundler signs the
# .app with whatever this names.
export APPLE_SIGNING_IDENTITY="-"

# The first file named `$2` under either place Tauri has put a bundle: the
# workspace's `target/`, or `src-tauri/target/` (.github/actions/release-assets).
first() {
  find "$root/target/$1" "$root/src-tauri/target/$1" -maxdepth 1 -name "$2" -print 2>/dev/null | head -n 1 || true
}

built=()
for target in "${targets[@]}"; do
  case "$target" in
    aarch64-apple-darwin) arch=arm64 ;;
    x86_64-apple-darwin) arch=x86_64 ;;
  esac
  bundle="$target/release/bundle"

  # Last time's .dmg would otherwise be found and reported as this one.
  rm -rf "$root/target/$bundle/dmg" "$root/target/$bundle/macos" \
    "$root/src-tauri/target/$bundle/dmg" "$root/src-tauri/target/$bundle/macos"

  echo "==> Building $target"
  # One `--` for bun and none for cargo: `bun run tauri build -- --bundles`
  # hands `--bundles` to cargo, which refuses it.
  bun run tauri -- build --bundles dmg --target "$target"

  app=$(first "$bundle/macos" '*.app')
  dmg=$(first "$bundle/dmg" '*.dmg')
  if [ -z "$app" ] || [ -z "$dmg" ]; then
    echo "the build left no .app or no .dmg under target/$bundle" >&2
    exit 1
  fi

  echo "==> Checking the signature: $app"
  codesign --verify --deep --strict --verbose=2 "$app"
  codesign -dv --verbose=4 "$app"

  # The architecture the bundle holds, not the one asked for: a build that
  # quietly took the host's would fail on the other kind of Mac.
  executable=$(plutil -extract CFBundleExecutable raw "$app/Contents/Info.plist")
  description=$(file "$app/Contents/MacOS/$executable")
  if ! grep -q "$arch" <<<"$description"; then
    echo "asked for $arch and built: $description" >&2
    exit 1
  fi

  echo "==> Checking the disk image: $dmg"
  hdiutil verify "$dmg"

  built+=("$dmg")
done

echo
echo "Built, ad-hoc signed and verified:"
for dmg in "${built[@]}"; do
  echo "  $dmg"
done

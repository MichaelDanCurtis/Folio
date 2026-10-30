#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
project="$PWD"
cargo build --release --manifest-path rust/Cargo.toml
# Build outside synced folders: Finder metadata added to frameworks can break signing.
stage="${FOLIO_BUILD_STAGE:-$(mktemp -d /private/tmp/folio-native-build.XXXXXX)}"
mkdir -p "$stage"
rsync -a --exclude build --exclude .dart_tool --exclude /rust --exclude /runtime --exclude scripts --exclude release --exclude .git --exclude macos/Flutter/ephemeral ./ "$stage/"
cd "$stage"
flutter pub get
flutter build macos --release --no-pub
app="$stage/build/macos/Build/Products/Release/Folio Native.app"
runtime="${FOLIO_CODEX_RUNTIME:-$project/runtime/package/vendor/aarch64-apple-darwin}"
if [[ ! -x "$runtime/bin/codex" && -z "${FOLIO_CODEX_RUNTIME:-}" ]]; then
  existing="$project/../folio-desktop/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin"
  if [[ -x "$existing/bin/codex" ]]; then runtime="$existing"; else python3 "$project/scripts/fetch_codex.py"; fi
fi
if [[ ! -x "$runtime/bin/codex" ]]; then
  echo 'Set FOLIO_CODEX_RUNTIME to the Codex 0.154.0 aarch64-apple-darwin runtime directory.' >&2
  exit 1
fi
"$runtime/bin/codex" --version | grep -q '0.154.0'
cp "$project/rust/target/release/libfolio_core.dylib" "$app/Contents/Frameworks/"
mkdir -p "$app/Contents/Resources/third_party"
ditto --noextattr --norsrc "$project/third_party" "$app/Contents/Resources/third_party"
mkdir -p "$app/Contents/Resources/codex-runtime"
ditto --noextattr --norsrc "$runtime" "$app/Contents/Resources/codex-runtime"
codesign --force --deep --sign - --entitlements "$project/macos/Runner/Release.entitlements" "$app"
codesign --verify --deep --strict "$app"
release="${FOLIO_RELEASE_DIR:-$project/release}"
mkdir -p "$release"
release="$(cd "$release" && pwd -P)"
ditto -c -k --sequesterRsrc --keepParent "$app" "$release/Folio Native.zip"
# ditto merges into an existing bundle, so files from an older build would stay
# inside and break the signature. Remove the previous app first, and only that
# exact path inside the release directory.
release_app="$release/Folio Native.app"
if [[ -L "$release_app" ]]; then
  echo "Refusing to replace $release_app: it is a symbolic link." >&2
  exit 1
fi
if [[ -e "$release_app" ]]; then
  if [[ "$(cd "$(dirname "$release_app")" && pwd -P)" != "$release" ]]; then
    echo "Refusing to remove $release_app: it is not inside $release." >&2
    exit 1
  fi
  rm -rf -- "$release_app"
fi
ditto --noextattr --norsrc "$app" "$release_app"
# Synced folders can attach Finder metadata during the copy. Remove only the
# two metadata types prohibited by code signing; retain quarantine and all
# other extended attributes. The file provider can add these again later; the
# portable ZIP is verified after a fresh extraction by release_manifest.py.
/usr/bin/xattr -r -d com.apple.FinderInfo "$release_app" 2>/dev/null || true
/usr/bin/xattr -r -d com.apple.ResourceFork "$release_app" 2>/dev/null || true
if ! codesign --verify --deep --strict "$release_app"; then
  echo "The copied app at $release_app fails 'codesign --verify --deep --strict'." >&2
  echo "The staged build at $app verified, so the copy changed it (leftover files or Finder metadata)." >&2
  exit 1
fi
python3 "$project/scripts/release_manifest.py" "$release/Folio Native.zip"

mkdir -p "$project/build"
printf '%s\n' "$stage" > "$project/build/last-staging-path.txt"
echo "Built $release/Folio Native.app"

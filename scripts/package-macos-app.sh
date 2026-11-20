#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Build, sign, notarize and package Veyron.app (macos/Veyron).
#
#   ./scripts/package-macos-app.sh build            universal Veyron.app, ad-hoc signed (local testing)
#   ./scripts/package-macos-app.sh release          build, then sign + notarize + DMG on the Mac mini
#   ./scripts/package-macos-app.sh sign-notarize    (runs ON the signing Mac) Developer ID sign, notarize, staple, DMG
#
# Signing Mac settings (override with env):
#   VEYRON_SIGN_HOST     ssh alias of the signing Mac            (default: mini)
#   VEYRON_SIGN_IDENTITY Developer ID Application identity       (default: the ZYVORAI LABS one)
#   VEYRON_NOTARY_PROFILE notarytool keychain profile            (default: Macora-Notary)
# The notary API key stays in the signing Mac's keychain; this script never reads or copies it.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG="$ROOT/macos/Veyron"
OUT="${VEYRON_MACOS_OUT:-$ROOT/dist/macos}"
APP="$OUT/Veyron.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
BUILD="$(git -C "$ROOT" rev-list --count HEAD 2>/dev/null || echo 1)"
SIGN_HOST="${VEYRON_SIGN_HOST:-mini}"
IDENTITY="${VEYRON_SIGN_IDENTITY:-Developer ID Application: ZYVORAI LABS PRIVATE LIMITED (NX667LTDQ2)}"
PROFILE="${VEYRON_NOTARY_PROFILE:-Macora-Notary}"
DMG="$OUT/Veyron-$VERSION.dmg"

say() { printf '\n── %s\n' "$*"; }

make_icns() {
  local png="$PKG/Resources/AppIcon-1024.png" set
  set="$(mktemp -d)/AppIcon.iconset"
  mkdir -p "$set"
  for s in 16 32 128 256 512; do
    sips -z "$s" "$s" "$png" --out "$set/icon_${s}x${s}.png" >/dev/null
    sips -z "$((s * 2))" "$((s * 2))" "$png" --out "$set/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "$set" -o "$APP/Contents/Resources/AppIcon.icns"
}

build_app() {
  say "Testing VeyronCore"
  (cd "$PKG" && swift test 2>&1 | grep -E "Executed [0-9]+ tests" | tail -1)
  say "Building universal binary (arm64 + x86_64)"
  (cd "$PKG" && swift build -c release --arch arm64 --arch x86_64 2>&1 | tail -2)
  local bin
  bin="$(cd "$PKG" && swift build -c release --arch arm64 --arch x86_64 --show-bin-path 2>/dev/null)/Veyron"
  [[ -x "$bin" ]] || { echo "missing $bin" >&2; exit 1; }
  rm -rf "$APP"
  mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
  cp "$bin" "$APP/Contents/MacOS/Veyron"
  sed -e "s/@VERSION@/$VERSION/" -e "s/@BUILD@/$BUILD/" "$PKG/Resources/Info.plist.in" >"$APP/Contents/Info.plist"
  make_icns
  plutil -lint "$APP/Contents/Info.plist" >/dev/null
  say "Veyron.app $VERSION ($BUILD): $(lipo -archs "$APP/Contents/MacOS/Veyron")"
}

cmd_build() {
  mkdir -p "$OUT"
  build_app
  codesign --force --sign - --options runtime "$APP"
  codesign --verify --deep --strict "$APP"
  echo "ad-hoc signed (local testing only): $APP"
}

notarize() {
  local file="$1"
  say "Notarizing $(basename "$file") (waits for Apple)"
  xcrun notarytool submit "$file" --keychain-profile "$PROFILE" --wait 2>&1 | tee "$OUT/notary.log"
  grep -q "status: Accepted" "$OUT/notary.log" || {
    local id
    id="$(sed -n 's/^  id: //p' "$OUT/notary.log" | head -1)"
    echo "notarization was not accepted" >&2
    [[ -n "$id" ]] && xcrun notarytool log "$id" --keychain-profile "$PROFILE" >&2 || true
    exit 1
  }
}

cmd_sign_notarize() {
  mkdir -p "$OUT"
  build_app
  say "Signing Veyron.app with Developer ID (hardened runtime)"
  codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
  local zip="$OUT/Veyron-app.zip"
  ditto -c -k --keepParent "$APP" "$zip"
  notarize "$zip"
  xcrun stapler staple "$APP"
  spctl -a -vvv -t exec "$APP"

  say "Packaging $(basename "$DMG")"
  local stage
  stage="$(mktemp -d)"
  cp -R "$APP" "$stage/"
  ln -s /Applications "$stage/Applications"
  rm -f "$DMG"
  hdiutil create -volname "Veyron" -srcfolder "$stage" -ov -format UDZO "$DMG" >/dev/null
  codesign --force --timestamp --sign "$IDENTITY" "$DMG"
  notarize "$DMG"
  xcrun stapler staple "$DMG"
  spctl -a -vvv -t open --context context:primary-signature "$DMG"
  xcrun stapler validate "$DMG"
  (cd "$OUT" && shasum -a 256 "$(basename "$DMG")" | tee "$(basename "$DMG").sha256")
}

cmd_release() {
  local remote="veyron-macos-build"
  say "Sending sources to $SIGN_HOST"
  ssh "$SIGN_HOST" "rm -rf ~/$remote && mkdir -p ~/$remote/macos ~/$remote/scripts"
  rsync -a --delete --exclude .build "$PKG" "$SIGN_HOST:$remote/macos/"
  rsync -a "$ROOT/Cargo.toml" "$SIGN_HOST:$remote/"
  rsync -a "$ROOT/scripts/package-macos-app.sh" "$SIGN_HOST:$remote/scripts/"
  say "Signing and notarizing on $SIGN_HOST"
  ssh "$SIGN_HOST" "cd ~/$remote && VEYRON_BUILD_NUMBER=$BUILD ./scripts/package-macos-app.sh sign-notarize"
  mkdir -p "$OUT"
  rsync -a "$SIGN_HOST:$remote/dist/macos/Veyron-$VERSION.dmg" "$SIGN_HOST:$remote/dist/macos/Veyron-$VERSION.dmg.sha256" "$OUT/"
  say "Done: $DMG"
}

case "${1:-}" in
  build) cmd_build ;;
  release) cmd_release ;;
  sign-notarize) BUILD="${VEYRON_BUILD_NUMBER:-$BUILD}" cmd_sign_notarize ;;
  *) sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac

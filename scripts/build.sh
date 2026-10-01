#!/bin/bash
# Builds build/BijoyBangla.app (ad-hoc signed).
set -euo pipefail
cd "$(dirname "$0")/.."

swift build -c release --product BijoyBangla
BIN="$(swift build -c release --show-bin-path)/BijoyBangla"

[ -f Resources/icon.tiff ] || swift scripts/make_icon.swift Resources/icon.tiff

APP=build/BijoyBangla.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/BijoyBangla"
cp Resources/Info.plist "$APP/Contents/Info.plist"
cp Resources/icon.tiff "$APP/Contents/Resources/icon.tiff"

# A real certificate keeps the Accessibility permission across rebuilds (ad-hoc signing
# changes identity every build, so macOS would ask again).
SIGN_ID="${SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development|Developer ID Application/ {print $2; exit}')}"
codesign --force --sign "${SIGN_ID:--}" "$APP"
echo "Signed with: ${SIGN_ID:-ad-hoc}"
echo "Built $APP"

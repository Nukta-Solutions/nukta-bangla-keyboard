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

codesign --force --sign - "$APP"
echo "Built $APP"

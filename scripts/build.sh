#!/bin/bash
# Builds build/NuktaBangla.app for this Mac, or with --universal for Apple Silicon and Intel.
set -euo pipefail
cd "$(dirname "$0")/.."

ARCHS=()
[ "${1:-}" = "--universal" ] && ARCHS=(--arch arm64 --arch x86_64)
swift build -c release ${ARCHS[@]+"${ARCHS[@]}"} --product NuktaBangla
BIN="$(swift build -c release ${ARCHS[@]+"${ARCHS[@]}"} --show-bin-path)/NuktaBangla"

[ Resources/icon.tiff -nt Resources/icon.svg ] || swift scripts/make_icon.swift Resources/icon.tiff
[ -f Resources/AppIcon.icns ] || swift scripts/make_icon.swift --app Resources/AppIcon.icns

APP=build/NuktaBangla.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/NuktaBangla"
cp Resources/Info.plist "$APP/Contents/Info.plist"
cp Resources/icon.tiff Resources/AppIcon.icns "$APP/Contents/Resources/"

# A real certificate keeps the Accessibility permission across rebuilds (ad-hoc signing
# changes identity every build, so macOS would ask again).
SIGN_ID="${SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development|Developer ID Application/ {print $2; exit}')}"
codesign --force --sign "${SIGN_ID:--}" "$APP"
echo "Signed with: ${SIGN_ID:-ad-hoc}"
echo "Built $APP"

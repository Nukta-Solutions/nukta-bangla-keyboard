#!/bin/bash
# Builds build/NuktaBangla.app for this Mac, or with --universal for Apple Silicon and Intel.
# --release signs it for distribution: Developer ID, hardened runtime and a secure timestamp.
set -euo pipefail
cd "$(dirname "$0")/.."

ARCHS=()
RELEASE=
for arg in "$@"; do
    case "$arg" in
        --universal) ARCHS=(--arch arm64 --arch x86_64) ;;
        --release) RELEASE=1 ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done

scripts/build_riti.sh

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

# MPL 2.0 asks that every copy of the app says where riti's source is: here it ships inside the
# app, with the licences of the Rust crates riti uses.
LICENSES="$APP/Contents/Resources/Licenses"
mkdir -p "$LICENSES"
cp Licenses/MPL-2.0.txt Licenses/THIRD-PARTY-NOTICES.txt "$LICENSES/"
zip -qr -X "$LICENSES/MPL-source.zip" riti-bridge/riti riti-bridge/riti.patch -x '*.DS_Store'

if [ -n "$RELEASE" ]; then
    # Gatekeeper and notarization need a Developer ID signature with the hardened runtime.
    SIGN_ID="${SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Developer ID Application/ {print $2; exit}')}"
    [ -n "$SIGN_ID" ] || { echo "No Developer ID Application certificate in the keychain" >&2; exit 1; }
    codesign --force --options runtime --timestamp --sign "$SIGN_ID" "$APP"
    codesign --verify --strict --verbose=2 "$APP"
else
    # A real certificate keeps the same signing identity across rebuilds (ad-hoc signing
    # changes it every build).
    SIGN_ID="${SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development|Developer ID Application/ {print $2; exit}')}"
    codesign --force --sign "${SIGN_ID:--}" "$APP"
fi
echo "Signed with: ${SIGN_ID:-ad-hoc}"
echo "Built $APP"

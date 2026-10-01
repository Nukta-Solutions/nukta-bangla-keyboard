#!/bin/bash
# Builds and installs Nukta Bangla (নুকতা বাংলা) into ~/Library/Input Methods.
set -euo pipefail
cd "$(dirname "$0")/.."

scripts/build.sh

DEST="$HOME/Library/Input Methods"
mkdir -p "$DEST"
killall NuktaBangla 2>/dev/null || true
# Remove the build from before the rename to Nukta Bangla.
killall BijoyBangla 2>/dev/null || true
rm -rf "$DEST/BijoyBangla.app"
rm -rf "$DEST/NuktaBangla.app"
cp -R build/NuktaBangla.app "$DEST/"

echo "Installed to $DEST/NuktaBangla.app"
echo "First time: System Settings → Keyboard → Input Sources → Edit… → + → Bangla → নুকতা বাংলা"
echo "(If it isn't listed, log out and back in.)"

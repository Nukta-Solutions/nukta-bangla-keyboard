#!/bin/bash
# Builds and installs Bijoy Bangla into ~/Library/Input Methods.
set -euo pipefail
cd "$(dirname "$0")/.."

scripts/build.sh

DEST="$HOME/Library/Input Methods"
mkdir -p "$DEST"
killall BijoyBangla 2>/dev/null || true
rm -rf "$DEST/BijoyBangla.app"
cp -R build/BijoyBangla.app "$DEST/"

echo "Installed to $DEST/BijoyBangla.app"
echo "First time: System Settings → Keyboard → Input Sources → Edit… → + → Bangla → Bijoy Bangla"
echo "(If it isn't listed, log out and back in.)"

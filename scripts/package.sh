#!/bin/bash
# Builds build/NuktaBangla-<version>.pkg to share: installs into /Library/Input Methods for
# Apple Silicon and Intel Macs. Not notarized, so the first open needs "Open Anyway" (see INSTALL.txt).
set -euo pipefail
cd "$(dirname "$0")/.."

scripts/build.sh --universal

VERSION="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' Resources/Info.plist)"
PKG="build/NuktaBangla-$VERSION.pkg"
WORK=build/pkg
rm -rf "$WORK" "$PKG"
mkdir -p "$WORK/root" "$WORK/scripts"
cp -R build/NuktaBangla.app "$WORK/root/"

# Without this, the installer "upgrades" any other copy of the app it finds (e.g. build/) instead.
pkgbuild --analyze --root "$WORK/root" "$WORK/components.plist"
/usr/libexec/PlistBuddy -c 'Set :0:BundleIsRelocatable false' "$WORK/components.plist"

# Stop the running old version so the new one loads.
cat > "$WORK/scripts/postinstall" <<'SH'
#!/bin/bash
killall NuktaBangla 2>/dev/null || true
exit 0
SH
chmod +x "$WORK/scripts/postinstall"

pkgbuild --root "$WORK/root" --component-plist "$WORK/components.plist" \
    --scripts "$WORK/scripts" --install-location "/Library/Input Methods" \
    --identifier com.asifmahmud.inputmethod.NuktaBangla.pkg --version "$VERSION" "$PKG"
rm -rf "$WORK"
cp INSTALL.txt build/
echo "Built $PKG (send it with build/INSTALL.txt)"

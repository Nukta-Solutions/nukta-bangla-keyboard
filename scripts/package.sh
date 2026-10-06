#!/bin/bash
# Builds build/NuktaBangla-<version>.pkg to share: installs into /Library/Input Methods for
# Apple Silicon and Intel Macs. Signed with Developer ID, notarized and stapled, so it opens
# without a Gatekeeper warning.
#
# Notarization uses the keychain profile in $NOTARY_PROFILE (default "nukta"). Create it once with
#   xcrun notarytool store-credentials nukta --apple-id <Apple ID> --team-id Y2466L4CFL
# and an app-specific password from account.apple.com. --no-notarize skips it for a quick test build.
set -euo pipefail
cd "$(dirname "$0")/.."

NOTARIZE=1
for arg in "$@"; do
    case "$arg" in
        --no-notarize) NOTARIZE= ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done
NOTARY_PROFILE="${NOTARY_PROFILE:-nukta}"

INSTALLER_ID="${INSTALLER_ID:-$(security find-identity -v 2>/dev/null | awk -F'"' '/Developer ID Installer/ {print $2; exit}')}"
[ -n "$INSTALLER_ID" ] || { echo "No Developer ID Installer certificate in the keychain" >&2; exit 1; }

scripts/build.sh --universal --release

VERSION="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' Resources/Info.plist)"
PKG="build/NuktaBangla-$VERSION.pkg"
WORK=build/pkg
rm -rf "$WORK" "$PKG"
mkdir -p "$WORK/root" "$WORK/scripts"
cp -R build/NuktaBangla.app "$WORK/root/"

# Without this, the installer "upgrades" any other copy of the app it finds (e.g. build/) instead.
pkgbuild --analyze --root "$WORK/root" "$WORK/components.plist"
/usr/libexec/PlistBuddy -c 'Set :0:BundleIsRelocatable false' "$WORK/components.plist"

# The installer won't replace a bundle with a different identifier: it would put the new app in
# NuktaBangla.localized/ beside the old com.asifmahmud build. Remove that build (and any such
# side-by-side copy) first.
cat > "$WORK/scripts/preinstall" <<'SH'
#!/bin/bash
DIR="/Library/Input Methods"
ID="$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$DIR/NuktaBangla.app/Contents/Info.plist" 2>/dev/null)"
if [ -n "$ID" ] && [ "$ID" != "com.nuktasolutions.inputmethod.NuktaBangla" ]; then
    killall NuktaBangla 2>/dev/null || true
    rm -rf "$DIR/NuktaBangla.app"
fi
rm -rf "$DIR/NuktaBangla.localized"
exit 0
SH
chmod +x "$WORK/scripts/preinstall"

# Stop the running old version so the new one loads.
cat > "$WORK/scripts/postinstall" <<'SH'
#!/bin/bash
killall NuktaBangla 2>/dev/null || true
exit 0
SH
chmod +x "$WORK/scripts/postinstall"

pkgbuild --root "$WORK/root" --component-plist "$WORK/components.plist" \
    --scripts "$WORK/scripts" --install-location "/Library/Input Methods" \
    --identifier com.nuktasolutions.inputmethod.NuktaBangla.pkg --version "$VERSION" \
    --sign "$INSTALLER_ID" --timestamp "$PKG"
rm -rf "$WORK"

if [ -n "$NOTARIZE" ]; then
    xcrun notarytool submit "$PKG" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$PKG"
    spctl --assess --type install --verbose=2 "$PKG"
else
    echo "Not notarized (--no-notarize): Gatekeeper will warn on other Macs."
fi
cp INSTALL.txt build/
echo "Built $PKG (send it with build/INSTALL.txt)"

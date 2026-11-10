#!/bin/bash
# Adds NuktaBangla.dmg to a release from this Mac, for when the Release workflow skipped macOS
# (no Apple signing secrets in the repository). Builds, signs and notarizes with scripts/package.sh
# and the "nukta" notarytool profile, uploads the .dmg under its fixed name, and publishes the
# release once the Windows and Linux files the workflow built are attached too, then points the
# Homebrew cask in Nukta-Solutions/homebrew-tap at it.
#
#   scripts/release-mac.sh v1.0.2
#
# Run it from a checkout of the tag (git switch --detach v1.0.2), after the workflow has finished.
set -euo pipefail
cd "$(dirname "$0")/.."

TAG="${1:?Usage: scripts/release-mac.sh <tag>, e.g. v1.0.2}"
VERSION="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' Resources/Info.plist)"
[ "$TAG" = "v$VERSION" ] || { echo "Tag $TAG does not match Info.plist ($VERSION): check out the tag first" >&2; exit 1; }
[ "$(git rev-parse HEAD)" = "$(git rev-parse "$TAG^{commit}")" ] \
    || { echo "HEAD is not $TAG: run git switch --detach $TAG first" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "Uncommitted changes: commit or stash them first" >&2; exit 1; }
gh release view "$TAG" >/dev/null || { echo "No release $TAG yet: push the tag and let the Release workflow run" >&2; exit 1; }

scripts/package.sh

# The website links to releases/latest/download/NuktaBangla.dmg: the name never changes.
cp "build/NuktaBangla-$VERSION.dmg" build/NuktaBangla.dmg
gh release upload "$TAG" build/NuktaBangla.dmg --clobber

ASSETS="$(gh release view "$TAG" --json assets -q '.assets[].name')"
for name in NuktaBangla.dmg NuktaBangla-Setup.exe nukta-bangla-linux.sh; do
    if ! grep -qx "$name" <<< "$ASSETS"; then
        echo "Uploaded NuktaBangla.dmg, but $name is not attached yet: $TAG stays a draft." >&2
        echo "Once it is, publish with: gh release edit $TAG --draft=false --latest" >&2
        exit 1
    fi
done
gh release edit "$TAG" --draft=false --latest
echo "Published $TAG: $(gh release view "$TAG" --json url -q .url)"

# Points the Homebrew cask at it, as the workflow's publish job does when it publishes: the version
# and the sha256 of what users actually download, from the public URL the cask uses. Committed with
# gh as whoever runs this, so it needs push access to the tap, not HOMEBREW_TAP_TOKEN.
TAP=Nukta-Solutions/homebrew-tap
CASK=Casks/nukta-bangla.rb
SHA256="$(curl -fsSL --retry 5 --retry-all-errors \
    "https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/download/$TAG/NuktaBangla.dmg" \
    | shasum -a 256 | cut -d' ' -f1)"
FILE_SHA="$(gh api "repos/$TAP/contents/$CASK" -q .sha)"
OLD="$(gh api "repos/$TAP/contents/$CASK" -q .content | base64 -d)"
NEW="$(sed -E "s/^(  version )\"[^\"]*\"/\1\"$VERSION\"/; s/^(  sha256 )\"[^\"]*\"/\1\"$SHA256\"/" <<< "$OLD")"
if ! grep -q "^  version \"$VERSION\"$" <<< "$NEW" || ! grep -q "^  sha256 \"$SHA256\"$" <<< "$NEW"; then
    echo "Could not update version/sha256 in $TAP $CASK: bump it by hand" >&2
    exit 1
fi
if [ "$NEW" = "$OLD" ]; then
    echo "Cask already at $VERSION"
else
    gh api -X PUT "repos/$TAP/contents/$CASK" -f message="nukta-bangla $VERSION" \
        -f content="$(printf '%s\n' "$NEW" | base64)" -f sha="$FILE_SHA" >/dev/null
    echo "Cask $TAP $CASK: $VERSION, sha256 $SHA256"
fi

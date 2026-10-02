#!/usr/bin/env bash
# Builds the files you hand to someone else:
#
#   build/nukta-bangla-<version>-linux.sh      one self-extracting installer — `bash <file>` and done
#   build/nukta-bangla-<version>-linux.tar.gz  the same tree as a plain tarball
#
# Both carry everything the engine needs, so neither git nor access to this repository is required
# on the other machine. This is the Linux counterpart of scripts/package.sh and its .pkg.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

VERSION=$(python3 -c 'import nukta_bangla; print(nukta_bangla.__version__)')
NAME="nukta-bangla-$VERSION"
BUILD="../build"
STAGE="$BUILD/$NAME"

echo "Running engine tests…"
python3 -m unittest discover -s tests -q

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp -R nukta_bangla icons tests install.sh uninstall.sh nukta-bangla.xml.in README.md "$STAGE/"
rm -rf "$STAGE/nukta_bangla/__pycache__" "$STAGE/tests/__pycache__"
chmod +x "$STAGE/install.sh" "$STAGE/uninstall.sh"
cp ibus-engine-nukta-bangla "$STAGE/"
chmod +x "$STAGE/ibus-engine-nukta-bangla"

TARBALL="$BUILD/$NAME-linux.tar.gz"
tar czf "$TARBALL" -C "$BUILD" "$NAME"

# Self-extracting installer: this header, then the tarball appended after the marker line.
INSTALLER="$BUILD/$NAME-linux.sh"
cat > "$INSTALLER" <<HEADER
#!/usr/bin/env bash
# নুকতা বাংলা (Nukta Bangla) $VERSION — Bijoy Bangla keyboard for Linux (IBus).
#
#   bash $NAME-linux.sh              install, asking before it installs IBus and the Python bindings
#   bash $NAME-linux.sh --with-deps  install those without asking (for a scripted setup)
#   bash $NAME-linux.sh --user       install into ~/.local, without root
#
# Everything below the marker is a gzipped tar of the engine; nothing is downloaded.
set -euo pipefail

SELF=\$(readlink -f "\${BASH_SOURCE[0]}")
[[ -f "\$SELF" ]] || { echo "Save this installer to a file and run: bash $NAME-linux.sh" >&2; exit 1; }

TMP=\$(mktemp -d)
trap 'rm -rf "\$TMP"' EXIT
PAYLOAD=\$(awk '/^__NUKTA_PAYLOAD__\$/ { print NR + 1; exit }' "\$SELF")
tail -n +"\$PAYLOAD" "\$SELF" | tar xz -C "\$TMP"

echo "নুকতা বাংলা $VERSION"
exec "\$TMP/$NAME/install.sh" "\$@"
__NUKTA_PAYLOAD__
HEADER
cat "$TARBALL" >> "$INSTALLER"
chmod +x "$INSTALLER"

echo
echo "Built:"
ls -lh "$INSTALLER" "$TARBALL" | awk '{print "  " $9 "  " $5}'
cat <<MSG

Send the .sh file. On the other machine, one line installs it:

    bash $NAME-linux.sh

It asks before installing IBus, the Python bindings and a Bangla font, then installs the engine and
restarts IBus. The person still adds নুকতা বাংলা in their keyboard settings once (the installer
prints how, per desktop).
MSG

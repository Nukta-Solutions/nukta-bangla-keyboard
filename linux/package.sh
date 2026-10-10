#!/usr/bin/env bash
# Builds the files you hand to someone else:
#
#   build/nukta-bangla-<version>-linux.sh      one self-extracting installer — `bash <file>` and done
#   build/nukta-bangla-<version>-linux.tar.gz  the same tree as a plain tarball
#
# Both carry everything the engine needs — including riti, built here, so phonetic typing works on
# the other machine without Rust — so neither git nor access to this repository is required there.
# This is the Linux counterpart of scripts/package.sh and its .pkg.
#
#   ./package.sh                 both layouts: build riti and bake it in
#   ./package.sh --no-phonetic   the Bijoy layout only, for a smaller package
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

PHONETIC=yes
[[ ${1:-} == --no-phonetic ]] && PHONETIC=no

VERSION=$(python3 -c 'import nukta_bangla; print(nukta_bangla.__version__)')
NAME="nukta-bangla-$VERSION"
BUILD="../build"
STAGE="$BUILD/$NAME"

# riti (OpenBangla's engine, Rust) is what phonetic typing needs, and the whole point of baking it
# in is that the other machine needs no Rust. Build it before the tests, so the phonetic corpus in
# tests/ runs against the library that is about to be packaged.
if [[ $PHONETIC == yes ]]; then
    ./build_riti.sh
fi

echo "Running engine tests…"
python3 -m unittest discover -s tests -q

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp -R nukta_bangla icons tests install.sh uninstall.sh nukta-bangla.xml.in README.md "$STAGE/"
cp build_riti.sh "$STAGE/"   # so a recipient without the library can still build it
cp ../LICENSE "$STAGE/"      # MIT: the notice travels with every copy

if [[ $PHONETIC == yes ]]; then
    # riti is MPL-2.0: the library ships with its licence notices and its source, the same files
    # the macOS app carries in Contents/Resources/Licenses.
    mkdir -p "$STAGE/lib" "$STAGE/licenses"
    cp ../riti-bridge/lib/libnukta_riti.so "$STAGE/lib/"
    cp ../Licenses/MPL-2.0.txt ../Licenses/THIRD-PARTY-NOTICES.txt "$STAGE/licenses/"
    tar czf "$STAGE/licenses/riti-source.tar.gz" -C .. riti-bridge/riti riti-bridge/riti.patch
fi
rm -rf "$STAGE/nukta_bangla/__pycache__" "$STAGE/tests/__pycache__"
chmod +x "$STAGE/install.sh" "$STAGE/uninstall.sh" "$STAGE/build_riti.sh"
cp ibus-engine-nukta-bangla "$STAGE/"
chmod +x "$STAGE/ibus-engine-nukta-bangla"

TARBALL="$BUILD/$NAME-linux.tar.gz"
tar czf "$TARBALL" -C "$BUILD" "$NAME"

# Self-extracting installer: this header, then the tarball appended after the marker line.
INSTALLER="$BUILD/$NAME-linux.sh"
cat > "$INSTALLER" <<HEADER
#!/usr/bin/env bash
# নুকতা বাংলা (Nukta Bangla) $VERSION — Bijoy and phonetic Bangla keyboards for Linux (IBus).
#
#   bash $NAME-linux.sh              install, asking before it installs IBus and the Python bindings
#   bash $NAME-linux.sh --with-deps  install those without asking (for a scripted setup)
#   bash $NAME-linux.sh --user       install into ~/.local, without root
#
# Everything below the marker is a gzipped tar of the engine, riti included; nothing is downloaded.
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
prints how, per desktop) — both layouts are listed there: নুকতা বাংলা - বিজয় and - ফোনেটিক.
MSG
[[ $PHONETIC == yes ]] || echo "Built without phonetic typing (--no-phonetic): the package offers Bijoy only."

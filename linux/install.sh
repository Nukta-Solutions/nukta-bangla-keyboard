#!/usr/bin/env bash
# Installs নুকতা বাংলা (Nukta Bangla) as an IBus engine.
#
#   ./install.sh           system-wide into /usr (asks for sudo) — recommended
#   ./install.sh --user    into ~/.local, no root, needs IBUS_COMPONENT_PATH (printed at the end)
#   PREFIX=/usr ./install.sh   for packagers; DESTDIR is honoured too
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

MODE=system
[[ "${1:-}" == "--user" ]] && MODE=user

# IBus only scans $datadir/ibus/component (that is /usr/share on every distro) plus whatever
# IBUS_COMPONENT_PATH lists, so a system install goes to /usr, not /usr/local.
if [[ $MODE == user ]]; then
    PREFIX="${PREFIX:-$HOME/.local}"
    SUDO=()
else
    PREFIX="${PREFIX:-/usr}"
    SUDO=()
    # Staged builds (DESTDIR) and root need no sudo; anyone else does.
    if [[ -z "${DESTDIR:-}" && $(id -u) -ne 0 ]]; then
        command -v sudo >/dev/null || { echo "Need root to write $PREFIX/share; run as root or install sudo." >&2; exit 1; }
        SUDO=(sudo)
    fi
fi

DATADIR="${DESTDIR:-}$PREFIX/share"
LIBDIR="$DATADIR/ibus-nukta-bangla"
COMPONENTDIR="$DATADIR/ibus/component"
VERSION=$(python3 -c 'import nukta_bangla; print(nukta_bangla.__version__)')

# The layout logic is the same corpus the macOS engine is tested against: never install a build
# that types differently.
echo "Running engine tests…"
python3 -m unittest discover -s tests -q

for cmd in ibus python3; do
    command -v "$cmd" >/dev/null || echo "Warning: $cmd not found — see linux/README.md for the packages to install." >&2
done
python3 -c 'import gi; gi.require_version("IBus", "1.0"); from gi.repository import IBus' 2>/dev/null \
    || echo "Warning: Python IBus bindings missing (install python3-gi / python-gobject + ibus). See linux/README.md." >&2

echo "Installing to $LIBDIR…"
"${SUDO[@]}" install -d "$LIBDIR/nukta_bangla" "$LIBDIR/icons" "$COMPONENTDIR"
"${SUDO[@]}" install -m 644 nukta_bangla/*.py "$LIBDIR/nukta_bangla/"
"${SUDO[@]}" install -m 644 icons/nukta-bangla.svg "$LIBDIR/icons/"
"${SUDO[@]}" install -m 755 ibus-engine-nukta-bangla "$LIBDIR/"
# Stale .pyc from an older install would shadow the new sources.
"${SUDO[@]}" rm -rf "$LIBDIR/nukta_bangla/__pycache__"

XML=$(mktemp)
trap 'rm -f "$XML"' EXIT
sed -e "s|@EXEC@|$PREFIX/share/ibus-nukta-bangla/ibus-engine-nukta-bangla|" \
    -e "s|@ICON@|$PREFIX/share/ibus-nukta-bangla/icons/nukta-bangla.svg|" \
    -e "s|@COMPONENTDIR@|$PREFIX/share/ibus/component|" \
    -e "s|@VERSION@|$VERSION|" \
    nukta-bangla.xml.in > "$XML"
"${SUDO[@]}" install -m 644 "$XML" "$COMPONENTDIR/nukta-bangla.xml"

if [[ -z "${DESTDIR:-}" ]] && command -v ibus >/dev/null; then
    ibus write-cache >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
fi

echo
echo "Installed নুকতা বাংলা $VERSION."
if [[ $MODE == user ]]; then
    cat <<MSG

A user install lives outside the directory IBus scans, so tell IBus where to look — add this to
~/.config/environment.d/nukta-bangla.conf (systemd sessions: GNOME, KDE, most distros):

    IBUS_COMPONENT_PATH=$PREFIX/share/ibus/component

then log out and back in. Without it IBus will not see the engine.
MSG
fi
cat <<'MSG'

Then add the keyboard:
  GNOME  Settings → Keyboard → Input Sources → + → Bangla → নুকতা বাংলা
  KDE    System Settings → Keyboard → Virtual Keyboard / Input Method → IBus → add Bangla - নুকতা বাংলা
  Any    ibus-setup → Input Method → Add → Bangla → নুকতা বাংলা

Switch layouts with Super+Space. If the engine is not listed, run `ibus restart` (or log out and
back in). Run ./install.sh again after any code change.
MSG

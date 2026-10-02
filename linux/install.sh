#!/usr/bin/env bash
# Installs নুকতা বাংলা (Nukta Bangla) as an IBus engine.
#
#   ./install.sh             system-wide into /usr (asks for sudo) — recommended
#   ./install.sh --user      into ~/.local, no root, needs IBUS_COMPONENT_PATH (printed at the end)
#   ./install.sh --with-deps also installs IBus and the Python bindings with the distro's package
#                            manager, without asking first (what the one-liner installer uses)
#   ./install.sh --print-deps   just say what is missing and the command that would install it
#   PREFIX=/usr ./install.sh    for packagers; DESTDIR is honoured too
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

MODE=system
DEPS=ask          # ask | yes | print
for arg in "$@"; do
    case "$arg" in
        --user) MODE=user ;;
        --with-deps) DEPS=yes ;;
        --print-deps) DEPS=print ;;
        -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
        *) echo "Unknown option: $arg (try --help)" >&2; exit 1 ;;
    esac
done

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

# --- dependencies: IBus itself, its Python bindings, and a Bangla font ------------------------
# Package names differ per distro; the font package is a best effort and never fatal.
case "$(. /etc/os-release 2>/dev/null && echo "${ID_LIKE:-${ID:-}}")" in
    *debian*|*ubuntu*) MANAGER=(apt-get install -y); PKG_IBUS=ibus; PKG_GI=python3-gi; PKG_FONT=fonts-beng ;;
    *fedora*|*rhel*)   MANAGER=(dnf install -y);     PKG_IBUS=ibus; PKG_GI=python3-gobject; PKG_FONT=google-noto-sans-bengali-fonts ;;
    *arch*)            MANAGER=(pacman -S --needed --noconfirm); PKG_IBUS=ibus; PKG_GI=python-gobject; PKG_FONT=noto-fonts ;;
    *suse*)            MANAGER=(zypper install -y); PKG_IBUS=ibus; PKG_GI=python3-gobject; PKG_FONT=noto-sans-bengali-fonts ;;
    *)                 MANAGER=() ;;
esac

MISSING=()
command -v ibus >/dev/null || MISSING+=("${PKG_IBUS:-ibus}")
python3 -c 'import gi; gi.require_version("IBus", "1.0"); from gi.repository import IBus' 2>/dev/null \
    || MISSING+=("${PKG_GI:-python3-gi}")
# No Bangla font means boxes instead of letters, so offer it alongside, but only if none is there.
if command -v fc-list >/dev/null && ! fc-list :lang=bn 2>/dev/null | grep -q .; then
    MISSING+=("${PKG_FONT:-fonts-beng}")
fi

install_deps() {
    (( ${#MISSING[@]} )) || return 0
    if (( ${#MANAGER[@]} == 0 )); then
        echo "Install these yourself, then run this again: ${MISSING[*]} (see linux/README.md)." >&2
        return 1
    fi
    local prefix=()
    [[ $(id -u) -ne 0 ]] && prefix=(sudo)
    echo "+ ${prefix[*]} ${MANAGER[*]} ${MISSING[*]}"
    "${prefix[@]}" "${MANAGER[@]}" "${MISSING[@]}"
}

deps_command() {
    (( ${#MANAGER[@]} )) && echo "sudo ${MANAGER[*]} ${MISSING[*]}" || echo "${MISSING[*]}"
}

if [[ $DEPS == print ]]; then
    (( ${#MISSING[@]} )) && echo "Missing: $(deps_command)" || echo "Everything নুকতা বাংলা needs is already installed."
    exit 0
fi

if (( ${#MISSING[@]} )); then
    if [[ $DEPS == yes ]]; then
        install_deps || true
    elif [[ -t 0 ]]; then
        echo "নুকতা বাংলা needs: ${MISSING[*]}"
        read -r -p "Install them now? [Y/n] " answer
        [[ ${answer:-y} =~ ^[Nn] ]] || install_deps || true
    else
        echo "Warning: missing ${MISSING[*]} — install with: $(deps_command)" >&2
    fi
fi

# The layout logic is the same corpus the macOS engine is tested against: never install a build
# that types differently.
echo "Running engine tests…"
python3 -m unittest discover -s tests -q

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

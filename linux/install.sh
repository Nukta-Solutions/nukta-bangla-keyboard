#!/usr/bin/env bash
# Installs নুকতা বাংলা (Nukta Bangla) as an IBus engine.
#
#   ./install.sh             system-wide into /usr (asks for sudo) — recommended
#   ./install.sh --user      into ~/.local, no root, needs IBUS_COMPONENT_PATH (printed at the end)
#   ./install.sh --with-deps also installs IBus and the Python bindings with the distro's package
#                            manager, without asking first (what the one-liner installer uses)
#   ./install.sh --print-deps   just say what is missing and the command that would install it
#   ./install.sh --with-phonetic  build riti for phonetic typing without asking (needs Rust)
#   ./install.sh --no-phonetic    install the Bijoy layout only
#   PREFIX=/usr ./install.sh    for packagers; DESTDIR is honoured too
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

MODE=system
DEPS=ask          # ask | yes | print
PHONETIC=ask      # ask | yes | no
for arg in "$@"; do
    case "$arg" in
        --user) MODE=user ;;
        --with-deps) DEPS=yes; PHONETIC=yes ;;
        --print-deps) DEPS=print ;;
        --with-phonetic) PHONETIC=yes ;;
        --no-phonetic) PHONETIC=no ;;
        -h|--help) sed -n '2,11p' "$0"; exit 0 ;;
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
# Read without Python: on a minimal system python3 is one of the dependencies installed below.
VERSION=$(sed -n 's/^__version__ = "\(.*\)"/\1/p' nukta_bangla/__init__.py)

# --- dependencies: IBus itself, its Python bindings, and a Bangla font ------------------------
# Package names differ per distro; the font package is a best effort and never fatal.
case "$(. /etc/os-release 2>/dev/null && echo "${ID_LIKE:-${ID:-}}")" in
    *debian*|*ubuntu*) MANAGER=(apt-get install -y); PKG_PY=python3; PKG_IBUS=ibus; PKG_GI=python3-gi; PKG_FONT=fonts-beng ;;
    *fedora*|*rhel*)   MANAGER=(dnf install -y);     PKG_PY=python3; PKG_IBUS=ibus; PKG_GI=python3-gobject; PKG_FONT=google-noto-sans-bengali-fonts ;;
    *arch*)            MANAGER=(pacman -S --needed --noconfirm); PKG_PY=python; PKG_IBUS=ibus; PKG_GI=python-gobject; PKG_FONT=noto-fonts ;;
    *suse*)            MANAGER=(zypper install -y); PKG_PY=python3; PKG_IBUS=ibus; PKG_GI=python3-gobject; PKG_FONT=noto-sans-bengali-fonts ;;
    *)                 MANAGER=() ;;
esac

MISSING=()
command -v python3 >/dev/null || MISSING+=("${PKG_PY:-python3}")
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
    # apt can't find packages until its lists are fetched (a new system) or refreshed (an old one).
    [[ ${MANAGER[0]} == apt-get ]] && { "${prefix[@]}" apt-get update -qq || true; }
    echo "+ ${prefix[*]} ${MANAGER[*]} ${MISSING[*]}"
    "${prefix[@]}" "${MANAGER[@]}" "${MISSING[@]}"
}

deps_command() {
    (( ${#MANAGER[@]} )) && echo "sudo ${MANAGER[*]} ${MISSING[*]}" || echo "${MISSING[*]}"
}

if [[ $DEPS == print ]]; then
    (( ${#MISSING[@]} )) && echo "Missing: $(deps_command)" || echo "Everything নুকতা বাংলা needs is already installed."
    # Phonetic typing is the one piece that may be missing without a package to install.
    if [[ -f lib/libnukta_riti.so || -f ../riti-bridge/lib/libnukta_riti.so ]]; then
        echo "Phonetic typing: riti is ready (libnukta_riti.so)."
    elif command -v cargo >/dev/null || [[ -f "$HOME/.cargo/env" ]]; then
        echo "Phonetic typing: riti is not built yet — install.sh can build it (Rust is installed)."
    else
        echo "Phonetic typing: needs riti, so either Rust (https://rustup.rs) or an installer that carries it."
    fi
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

command -v python3 >/dev/null || { echo "নুকতা বাংলা needs Python 3: install python3, then run this again." >&2; exit 1; }

# --- phonetic typing: riti ----------------------------------------------------------------------
# The Bijoy layout is pure Python, but phonetic typing uses riti (OpenBangla's engine, Rust) as a
# shared library. A package built by package.sh carries it in lib/; a clone can build it with
# linux/build_riti.sh if Rust is installed. Without it only Bijoy is installed, and the phonetic
# input source is left out of the XML so nobody can pick a layout that cannot type.
RITI_SO=""
RITI_SOURCE=""      # riti's own source, for the MPL notice that travels with the library

find_riti() {
    local candidate
    for candidate in lib/libnukta_riti.so ../riti-bridge/lib/libnukta_riti.so; do
        [[ -f $candidate ]] && { RITI_SO=$candidate; return 0; }
    done
    return 1
}

have_cargo() {
    command -v cargo >/dev/null && return 0
    [[ -f "$HOME/.cargo/env" ]] || return 1
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
    command -v cargo >/dev/null
}

if [[ $PHONETIC != no ]] && ! find_riti; then
    if [[ -x ./build_riti.sh && -d ../riti-bridge ]] && have_cargo; then
        build=no
        if [[ $PHONETIC == yes ]]; then
            build=yes
        elif [[ -t 0 ]]; then
            echo "Phonetic typing (Avro Phonetic) needs riti built from source — a few minutes with Rust,"
            echo "which is already installed. The Bijoy layout does not need it."
            read -r -p "Build it now? [Y/n] " answer
            [[ ${answer:-y} =~ ^[Nn] ]] || build=yes
        fi
        if [[ $build == yes ]]; then
            ./build_riti.sh || echo "Warning: riti did not build; installing the Bijoy layout only." >&2
            find_riti || true
        fi
    elif [[ $PHONETIC == yes ]]; then
        echo "Warning: cannot build riti here (needs Rust from https://rustup.rs and linux/build_riti.sh)." >&2
    fi
fi

if [[ -n $RITI_SO ]]; then
    # riti is MPL-2.0: every copy of the library says where its source is, so the notices and the
    # source travel with it, exactly as they do inside the macOS app.
    for licenses in licenses ../Licenses; do
        [[ -d $licenses ]] && { RITI_SOURCE=$licenses; break; }
    done
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
# MIT: ship the notice with the installed copy. ../LICENSE in a clone, ./LICENSE in a package.
for license in ../LICENSE LICENSE; do
    [[ -f $license ]] && { "${SUDO[@]}" install -m 644 "$license" "$LIBDIR/LICENSE"; break; }
done
# Stale .pyc from an older install would shadow the new sources.
"${SUDO[@]}" rm -rf "$LIBDIR/nukta_bangla/__pycache__"

# riti, if we have it: the library beside the Python (nukta_bangla/riti.py looks for it there),
# and its licences and source next to it. A library left over from an earlier install with
# phonetic typing is removed, so the XML and what is on disk always agree.
"${SUDO[@]}" rm -rf "$LIBDIR/lib" "$LIBDIR/licenses"
if [[ -n $RITI_SO ]]; then
    "${SUDO[@]}" install -d "$LIBDIR/lib" "$LIBDIR/licenses"
    "${SUDO[@]}" install -m 644 "$RITI_SO" "$LIBDIR/lib/libnukta_riti.so"
    for notice in MPL-2.0.txt THIRD-PARTY-NOTICES.txt; do
        [[ -f "$RITI_SOURCE/$notice" ]] && "${SUDO[@]}" install -m 644 "$RITI_SOURCE/$notice" "$LIBDIR/licenses/"
    done
    if [[ -f "$RITI_SOURCE/riti-source.tar.gz" ]]; then
        "${SUDO[@]}" install -m 644 "$RITI_SOURCE/riti-source.tar.gz" "$LIBDIR/licenses/"
    elif [[ -d ../riti-bridge/riti ]]; then
        # A clone: pack riti's source as the package would have.
        SOURCE_TAR=$(mktemp)
        tar czf "$SOURCE_TAR" -C .. riti-bridge/riti riti-bridge/riti.patch
        "${SUDO[@]}" install -m 644 "$SOURCE_TAR" "$LIBDIR/licenses/riti-source.tar.gz"
        rm -f "$SOURCE_TAR"
    fi
fi

XML=$(mktemp)
trap 'rm -f "$XML"' EXIT
# No riti: drop the phonetic engine between the markers, so only Bijoy is offered.
PHONETIC_FILTER=(-e "/PHONETIC-\(BEGIN\|END\)/d")
[[ -n $RITI_SO ]] || PHONETIC_FILTER=(-e "/PHONETIC-BEGIN/,/PHONETIC-END/d")
sed -e "s|@EXEC@|$PREFIX/share/ibus-nukta-bangla/ibus-engine-nukta-bangla|" \
    -e "s|@ICON@|$PREFIX/share/ibus-nukta-bangla/icons/nukta-bangla.svg|" \
    -e "s|@COMPONENTDIR@|$PREFIX/share/ibus/component|" \
    -e "s|@VERSION@|$VERSION|" \
    "${PHONETIC_FILTER[@]}" \
    nukta-bangla.xml.in > "$XML"
"${SUDO[@]}" install -m 644 "$XML" "$COMPONENTDIR/nukta-bangla.xml"

if [[ -z "${DESTDIR:-}" ]] && command -v ibus >/dev/null; then
    # A running engine has the old Python already loaded, and `ibus restart` restarts the daemon
    # without touching it: without this the session keeps typing with the code we just replaced.
    # It belongs to this user's daemon, so no sudo — and IBus respawns it on the next keystroke.
    pkill -f "$LIBDIR/ibus-engine-nukta-bangla" >/dev/null 2>&1 || true
    ibus write-cache >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
fi

echo
if [[ -n $RITI_SO ]]; then
    echo "Installed নুকতা বাংলা $VERSION with both layouts: Bijoy and phonetic."
else
    echo "Installed নুকতা বাংলা $VERSION (Bijoy layout only)."
fi
if [[ $MODE == user ]]; then
    cat <<MSG

A user install lives outside the directory IBus scans, so tell IBus where to look — add this to
~/.config/environment.d/nukta-bangla.conf (systemd sessions: GNOME, KDE, most distros):

    IBUS_COMPONENT_PATH=$PREFIX/share/ibus/component

then log out and back in. Without it IBus will not see the engine.
MSG
fi
if [[ -z $RITI_SO ]]; then
    cat <<'MSG'

Phonetic typing (Avro Phonetic) is not installed: it needs riti, a compiled library. To add it,
install Rust from https://rustup.rs, then run linux/build_riti.sh and linux/install.sh again —
or use an installer built with phonetic typing included.
MSG
fi
cat <<'MSG'

Then add the keyboard:
  GNOME  Settings → Keyboard → Input Sources → + → Bangla → নুকতা বাংলা - বিজয়
  KDE    System Settings → Keyboard → Virtual Keyboard / Input Method → IBus → add Bangla - নুকতা বাংলা
  Any    ibus-setup → Input Method → Add → Bangla → নুকতা বাংলা - বিজয়

Both layouts are listed under Bangla: **নুকতা বাংলা - বিজয়** and **নুকতা বাংলা - ফোনেটিক**.
Add either or both; Super+Space switches between whatever you added, and each layout's own menu
can switch to the other. Phonetic settings live in ~/.config/nukta-bangla/settings.json
(linux/README.md lists them).

If an engine is not listed, run `ibus restart` (or log out and back in). Run ./install.sh again
after any code change.
MSG

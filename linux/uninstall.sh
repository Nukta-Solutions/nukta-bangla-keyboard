#!/usr/bin/env bash
# Removes নুকতা বাংলা (Nukta Bangla). Use --user if it was installed with ./install.sh --user.
set -euo pipefail

MODE=system
[[ "${1:-}" == "--user" ]] && MODE=user
if [[ $MODE == user ]]; then PREFIX="${PREFIX:-$HOME/.local}"; else PREFIX="${PREFIX:-/usr}"; fi

SUDO=()
[[ -w "$PREFIX/share" ]] || { command -v sudo >/dev/null && SUDO=(sudo); }

"${SUDO[@]}" rm -rf "$PREFIX/share/ibus-nukta-bangla"
"${SUDO[@]}" rm -f "$PREFIX/share/ibus/component/nukta-bangla.xml"

if command -v ibus >/dev/null; then
    # The engine process survives `ibus restart`, and would keep running from the files just
    # deleted. Kill it before the daemon restarts, so nothing respawns it.
    pkill -f "$PREFIX/share/ibus-nukta-bangla/ibus-engine-nukta-bangla" >/dev/null 2>&1 || true
    ibus write-cache >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
fi
echo "Removed নুকতা বাংলা from $PREFIX. Remove the input source from your keyboard settings too."

#!/usr/bin/env bash
# নুকতা বাংলা (Nukta Bangla) — one-line installer for Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/Nukta-Solutions/nukta-bangla-keyboard/main/linux/bootstrap.sh | bash
#
# Downloads the current source, checks it, and runs linux/install.sh. Arguments reach install.sh:
#   … | bash -s -- --user        install into ~/.local, without root
#   … | bash -s -- --with-deps   install IBus and the Python bindings without asking first
#
# Overrides, for a fork or a tagged release:
#   NUKTA_REPO=owner/name  NUKTA_REF=v1.0.0  NUKTA_URL=https://…/something.tar.gz
set -euo pipefail

REPO="${NUKTA_REPO:-Nukta-Solutions/nukta-bangla-keyboard}"
REF="${NUKTA_REF:-main}"
case "$REF" in
    v[0-9]*) REF_PATH="refs/tags/$REF" ;;
    *)       REF_PATH="refs/heads/$REF" ;;
esac
URL="${NUKTA_URL:-https://codeload.github.com/$REPO/tar.gz/$REF_PATH}"

command -v tar >/dev/null || { echo "This installer needs tar." >&2; exit 1; }
if command -v curl >/dev/null; then
    FETCH=(curl -fsSL --retry 3 -o)
elif command -v wget >/dev/null; then
    FETCH=(wget -qO)
else
    echo "This installer needs curl or wget." >&2; exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "নুকতা বাংলা — downloading $REPO ($REF)…"
"${FETCH[@]}" "$TMP/source.tar.gz" "$URL"
tar xzf "$TMP/source.tar.gz" -C "$TMP"

# The tarball unpacks to one directory, named after the repository and ref.
SRC=$(find "$TMP" -mindepth 1 -maxdepth 1 -type d | head -1)
[[ -x "$SRC/linux/install.sh" ]] || { echo "That archive does not look like নুকতা বাংলা (no linux/install.sh)." >&2; exit 1; }

# Piped into bash, stdin is the pipe, so install.sh could not ask about missing packages. Give it
# back the terminal where there is one; without a terminal it falls back to printing the command.
# /dev/tty can exist and still refuse to open (a session with no controlling terminal), so try it
# in a subshell first rather than killing the install on the real redirect.
if [[ ! -t 0 ]] && (exec </dev/tty) 2>/dev/null; then
    exec < /dev/tty
fi

exec "$SRC/linux/install.sh" "$@"

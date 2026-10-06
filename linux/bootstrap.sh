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
#   NUKTA_REPO=owner/name  NUKTA_REF=v1.0.1  NUKTA_URL=https://…/something.tar.gz
#   NUKTA_CONNECT_TIMEOUT=30            longer, for a slow link
set -euo pipefail

REPO="${NUKTA_REPO:-Nukta-Solutions/nukta-bangla-keyboard}"
REF="${NUKTA_REF:-main}"
case "$REF" in
    v[0-9]*) REF_PATH="refs/tags/$REF" ;;
    *)       REF_PATH="refs/heads/$REF" ;;
esac
URL="${NUKTA_URL:-https://codeload.github.com/$REPO/tar.gz/$REF_PATH}"

if [[ "${1:-}" == "--diagnose" ]]; then
    echo "নুকতা বাংলা — checking what this network can reach"
    for host in raw.githubusercontent.com codeload.github.com github.com api.github.com; do
        printf '  %-28s ' "$host"
        curl -sS -o /dev/null -m 25 --connect-timeout 10 --speed-limit 1024 --speed-time 15 \
            -w 'HTTP %{http_code}  connect %{time_connect}s  total %{time_total}s\n' \
            "https://$host/" 2>&1 | tail -1
    done
    printf '  %-28s ' "tarball download"
    curl -sS -o /dev/null -m 60 --connect-timeout 10 --speed-limit 1024 --speed-time 20 \
        -w 'HTTP %{http_code}  %{size_download} bytes  %{time_total}s\n' \
        "https://codeload.github.com/${NUKTA_REPO:-Nukta-Solutions/nukta-bangla-keyboard}/tar.gz/refs/heads/main" 2>&1 | tail -1
    printf '  %-28s ' "git clone"
    if command -v git >/dev/null; then
        GIT_TERMINAL_PROMPT=0 timeout 40 git ls-remote --heads \
            "https://github.com/${NUKTA_REPO:-Nukta-Solutions/nukta-bangla-keyboard}.git" main >/dev/null 2>&1 \
            && echo "works" || echo "failed or timed out"
    else
        echo "git not installed"
    fi
    exit 0
fi

command -v tar >/dev/null || { echo "This installer needs tar." >&2; exit 1; }

# Every GitHub tarball URL — /archive, the API's /tarball — redirects to codeload.github.com, so a
# second URL is no fallback at all when codeload is what a network blocks or stalls on. Cloning
# talks to github.com instead, which is the one genuinely different route. Each attempt is bounded:
# a stalled connection must fail and move on, not hang silently the way an untimed curl does.
# --connect-timeout only covers making the connection. A network that blocks codeload by accepting
# the connection and then sending nothing sails past it, which is why --speed-time matters: it gives
# up when no data has moved for that long. Without it the only limit is --max-time, and a long one
# looks exactly like a hang.
CONNECT_TIMEOUT="${NUKTA_CONNECT_TIMEOUT:-10}"
STALL_TIMEOUT="${NUKTA_STALL_TIMEOUT:-20}"
MAX_TIME="${NUKTA_MAX_TIME:-300}"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

download() {   # download <url> [extra curl/wget args…]
    local url=$1; shift
    if command -v curl >/dev/null; then
        # A progress bar on a terminal; just errors when the output is a log or a pipe, where the
        # bar is unreadable noise.
        local progress=(-sS); [[ -t 2 ]] && progress=(--progress-bar)
        curl -fL --connect-timeout "$CONNECT_TIMEOUT" --max-time "$MAX_TIME" \
             --speed-limit 1024 --speed-time "$STALL_TIMEOUT" \
             --retry 1 --retry-connrefused "${progress[@]}" "$@" -o "$TMP/source.tar.gz" "$url"
    elif command -v wget >/dev/null; then
        local progress=(-nv); [[ -t 2 ]] && progress=(-q --show-progress)
        wget --connect-timeout="$CONNECT_TIMEOUT" --read-timeout="$STALL_TIMEOUT" --tries=2 \
             "${progress[@]}" "$@" -O "$TMP/source.tar.gz" "$url"
    else
        return 127
    fi
}

echo "নুকতা বাংলা — downloading $REPO ($REF)…"
echo "  (if this pauses, codeload.github.com is blocked here; it gives up after ${STALL_TIMEOUT}s and clones instead)"
SRC=""
if download "$URL"; then
    tar xzf "$TMP/source.tar.gz" -C "$TMP"
elif [[ -z "${NUKTA_URL:-}" ]] && command -v curl >/dev/null && download "$URL" -4; then
    # Broken IPv6 is the other common way this stalls; -4 forces IPv4.
    echo "  (IPv6 did not work here; used IPv4)"
    tar xzf "$TMP/source.tar.gz" -C "$TMP"
elif [[ -z "${NUKTA_URL:-}" ]] && command -v git >/dev/null; then
    echo "  codeload.github.com did not answer — cloning from github.com instead…"
    GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=true \
    git clone --quiet --depth 1 --branch "$REF" "https://github.com/$REPO.git" "$TMP/clone" \
        || { echo "Could not reach GitHub. Download the installer from
  https://github.com/$REPO/releases and run: bash nukta-bangla-<version>-linux.sh" >&2; exit 1; }
    SRC="$TMP/clone"
else
    echo "Download failed, and git is not installed to fall back on. Get the installer from
  https://github.com/$REPO/releases and run: bash nukta-bangla-<version>-linux.sh" >&2
    exit 1
fi

# A tarball unpacks to one directory, named after the repository and ref.
[[ -n $SRC ]] || SRC=$(find "$TMP" -mindepth 1 -maxdepth 1 -type d | head -1)
[[ -x "$SRC/linux/install.sh" ]] || { echo "That archive does not look like নুকতা বাংলা (no linux/install.sh)." >&2; exit 1; }

# Piped into bash, stdin is the pipe, so install.sh could not ask about missing packages. Give it
# back the terminal where there is one; without a terminal it falls back to printing the command.
# /dev/tty can exist and still refuse to open (a session with no controlling terminal), so try it
# in a subshell first rather than killing the install on the real redirect.
if [[ ! -t 0 ]] && (exec </dev/tty) 2>/dev/null; then
    exec < /dev/tty
fi

exec "$SRC/linux/install.sh" "$@"

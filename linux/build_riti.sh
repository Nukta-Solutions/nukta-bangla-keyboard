#!/usr/bin/env bash
# Builds riti (OpenBangla's phonetic engine) as a shared library for Linux:
#
#   riti-bridge/lib/libnukta_riti.so
#
# The Linux engine is Python, so it loads riti at run time with ctypes
# (linux/nukta_bangla/riti.py) instead of linking it the way the macOS app does. Phonetic typing
# is the only thing that needs this; the Bijoy layout is pure Python and never touches it.
#
#   ./build_riti.sh            build for this machine
#   ./build_riti.sh --quiet    only complain if something goes wrong (install.sh uses this)
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/../riti-bridge"

QUIET=0
[[ ${1:-} == --quiet ]] && QUIET=1
say() { (( QUIET )) || echo "$@"; }

command -v cargo >/dev/null || [ ! -f "$HOME/.cargo/env" ] || source "$HOME/.cargo/env"
command -v cargo >/dev/null || {
    echo "Rust is missing: install it from https://rustup.rs, then run linux/build_riti.sh again." >&2
    exit 1
}

say "Building riti (this takes a few minutes the first time)…"
cargo build --release --locked 2>&1 \
    | grep -vE '^\s*(Compiling|Fresh|Finished|Updating|Locking|Downloaded|Downloading)' || true
[[ -f target/release/libnukta_riti.so ]] || { echo "riti build failed: no target/release/libnukta_riti.so" >&2; exit 1; }

mkdir -p lib
install -m 644 target/release/libnukta_riti.so lib/libnukta_riti.so
# Debug symbols are a third of the file and nothing reads them here.
strip --strip-unneeded lib/libnukta_riti.so 2>/dev/null || true
say "Built riti-bridge/lib/libnukta_riti.so ($(du -h lib/libnukta_riti.so | cut -f1))"

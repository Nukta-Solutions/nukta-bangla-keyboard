#!/bin/bash
# Builds riti (OpenBangla's phonetic engine) as riti-bridge/lib/libnukta_riti.a for Apple Silicon and
# Intel. Run once before `swift build` / `swift test`; scripts/build.sh runs it for you.
set -euo pipefail
cd "$(dirname "$0")/../riti-bridge"

command -v cargo >/dev/null || source "$HOME/.cargo/env" 2>/dev/null || true
command -v cargo >/dev/null || { echo "Rust is missing: install it from https://rustup.rs" >&2; exit 1; }

# Match the app's minimum macOS, or the linker warns about every object file.
export MACOSX_DEPLOYMENT_TARGET=13.0
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
rustup target add "${TARGETS[@]}" >/dev/null 2>&1 || true
for target in "${TARGETS[@]}"; do
    cargo build --release --locked --target "$target" 2>&1 | grep -vE '^\s*(Compiling|Fresh|Finished|Updating|Locking|Downloaded|Downloading)' || true
    [ -f "target/$target/release/libnukta_riti.a" ] || { echo "riti build failed for $target" >&2; exit 1; }
done

mkdir -p lib
lipo -create $(printf 'target/%s/release/libnukta_riti.a ' "${TARGETS[@]}") -output lib/libnukta_riti.a
echo "Built riti-bridge/lib/libnukta_riti.a"

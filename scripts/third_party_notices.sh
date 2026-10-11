#!/bin/bash
# Writes the licence of every Rust crate linked into the app:
#   Licenses/THIRD-PARTY-NOTICES.txt          riti (macOS and Linux)
#   Licenses/THIRD-PARTY-NOTICES-WINDOWS.txt  the Windows input method: riti, plus what it adds
# Run again after changing riti-bridge/Cargo.lock or windows/Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || source "$HOME/.cargo/env"

META="$(mktemp)"
trap 'rm -f "$META"' EXIT

# notices <crate directory> <platform> <output file> <heading>
notices() {
    (cd "$1" && cargo metadata --format-version 1 --locked --filter-platform "$2") > "$META"
    python3 - "$META" "$3" "$4" <<'PY'
import json, os, sys

meta = json.load(open(sys.argv[1]))
output, heading = sys.argv[2], sys.argv[3]
resolved = {node["id"] for node in meta["resolve"]["nodes"]}
# Where a crate offers a choice, the MIT text: the shortest one that applies.
preferred = ["LICENSE-MIT", "LICENSE", "LICENSE.TXT", "LICENSE.md", "COPYING", "LICENSE-APACHE", "LICENSE-BSD"]
rule = "=" * 78
out = [
    heading,
    "riti itself is under the Mozilla Public License 2.0 (MPL-2.0.txt).",
    "",
]
count = 0
for pkg in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
    if pkg["id"] not in resolved or pkg["name"] in ("nukta-riti", "riti", "nukta-bangla-windows"):
        continue
    folder = os.path.dirname(pkg["manifest_path"])
    files = [f for f in preferred if os.path.exists(os.path.join(folder, f))]
    out += [rule, f"{pkg['name']} {pkg['version']} ({pkg['license']})"]
    if pkg.get("repository"):
        out.append(pkg["repository"])
    out.append(rule)
    if files:
        out.append(open(os.path.join(folder, files[0]), encoding="utf-8").read().strip())
    unicode = os.path.join(folder, "LICENSE-UNICODE")
    if "Unicode" in (pkg["license"] or "") and os.path.exists(unicode):
        out += ["", open(unicode, encoding="utf-8").read().strip()]
    out.append("")
    count += 1
open(output, "w", encoding="utf-8").write("\n".join(out))
print(f"Wrote {output} ({count} crates)")
PY
}

notices riti-bridge aarch64-apple-darwin Licenses/THIRD-PARTY-NOTICES.txt \
    "Third-party software in Nukta Bangla's phonetic engine: the Rust crates riti uses."
notices windows x86_64-pc-windows-msvc Licenses/THIRD-PARTY-NOTICES-WINDOWS.txt \
    "Third-party software in Nukta Bangla for Windows: the Rust crates it and riti use."

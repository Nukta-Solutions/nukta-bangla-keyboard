#!/bin/bash
# Writes Licenses/THIRD-PARTY-NOTICES.txt: the licence of every Rust crate linked into riti.
# Run again after changing riti-bridge/Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || source "$HOME/.cargo/env"

META="$(mktemp)"
trap 'rm -f "$META"' EXIT
(cd riti-bridge && cargo metadata --format-version 1 --locked --filter-platform aarch64-apple-darwin) > "$META"

python3 - "$META" <<'PY'
import json, os, sys

meta = json.load(open(sys.argv[1]))
resolved = {node["id"] for node in meta["resolve"]["nodes"]}
# Where a crate offers a choice, the MIT text: the shortest one that applies.
preferred = ["LICENSE-MIT", "LICENSE", "LICENSE.TXT", "LICENSE.md", "COPYING", "LICENSE-APACHE", "LICENSE-BSD"]
rule = "=" * 78
out = [
    "Third-party software in Nukta Bangla's phonetic engine: the Rust crates riti uses.",
    "riti itself is under the Mozilla Public License 2.0 (MPL-2.0.txt).",
    "",
]
count = 0
for pkg in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
    if pkg["id"] not in resolved or pkg["name"] in ("nukta-riti", "riti"):
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
open("Licenses/THIRD-PARTY-NOTICES.txt", "w", encoding="utf-8").write("\n".join(out))
print(f"Wrote Licenses/THIRD-PARTY-NOTICES.txt ({count} crates)")
PY

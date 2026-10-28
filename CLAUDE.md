# নুকতা বাংলা

## Release rules
- Every GitHub release must carry all three assets, with exactly these names, because the website links
  to `releases/latest/download/<name>`:
  - `NuktaBangla.dmg` (macOS): https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/NuktaBangla.dmg
  - `NuktaBangla-Setup.exe` (Windows): https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/NuktaBangla-Setup.exe
  - `nukta-bangla-linux.sh` (Linux): https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/nukta-bangla-linux.sh
- Never rename these files, put a version in their names, or publish a release missing one of them.
  Keep releases as drafts until all three are attached.
- The version lives in three places and they must match the tag: `Resources/Info.plist`
  (`CFBundleShortVersionString`, plus bump `CFBundleVersion`), `windows/Cargo.toml`, and
  `linux/nukta_bangla/__init__.py`.
- Pushing a `v*` tag runs `.github/workflows/release.yml`: it checks the three versions against the tag,
  builds all three assets, refuses to publish unless all three are attached, then publishes and
  updates the Homebrew cask. Full steps and the secrets it needs are in README.md under "Releasing".
- The macOS job only runs when the Apple signing secrets are set. Until then the user signs the Mac
  build on their own Mac (Developer ID certificates and the `nukta` notarytool profile are in its
  keychain): after the workflow leaves a draft with the .exe and .sh, `scripts/release-mac.sh <tag>`
  builds and notarizes the .dmg, uploads it, and publishes.

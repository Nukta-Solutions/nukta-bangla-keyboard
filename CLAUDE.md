# নুকতা বাংলা

## Team
- Asif writes all the code, the macOS Swift code included (on Windows, where it can't be compiled),
  and releases Windows and Linux.
- Harun's only part is the Mac release: on his Mac mini (Developer ID certificates and the `nukta`
  notarytool profile in its keychain) he runs `scripts/release-mac.sh <tag>`, which builds and
  notarizes the .dmg, uploads it and publishes. Keep his side to that one command.

## Release rules
- Platforms are released separately, on one version line: a release carries only the platforms in
  it. A version number is used once; the next release of any platform takes the next free number.
- The files never change name and never carry a version:
  - `NuktaBangla.dmg` (macOS), `NuktaBangla-Setup.exe` (Windows), `nukta-bangla-linux.sh` (Linux).
- The version lives in three places, bumped together on every release (even for a platform not in
  it) and matching the tag: `Resources/Info.plist` (`CFBundleShortVersionString`, plus bump
  `CFBundleVersion`), `windows/Cargo.toml` (and its Cargo.lock), and `linux/nukta_bangla/__init__.py`.
- Each platform's update check offers only the newest published release with its own file, so a
  Windows-only release never prompts Mac users. Builds before this (Windows 1.0.3–1.0.4, Linux up to
  1.0.4, macOS 1.0.2) still prompt for every latest release.
- Create the draft release with Bangla `- ` notes (they become the update dialog's "নতুন যা আছে"; never
  mention Avro) before pushing the tag; the workflow's generated notes are English PR lines.
- Pushing a `v*` tag runs `.github/workflows/release.yml`: it checks the three versions against the
  tag, builds Windows and Linux into the draft, and builds macOS only when the Apple signing secrets
  are set (they aren't). The draft is then published by hand for a Windows/Linux-only release, or by
  Harun's `scripts/release-mac.sh <tag>` for one with macOS. Full steps are in README.md, "Releasing".
- `releases/latest/download/<name>` only works for platforms in the latest release: the website's
  button for a platform left out links to that platform's last release (macOS: v1.0.2).

## CI rules
- GitHub Actions run only for `main`: `windows.yml` and `macos.yml` (an unsigned build and test of
  the Swift code, so a broken Mac build is caught before Harun's release) on pushes to main
  (including merged pull requests), `release.yml` on `v*` tags, and it refuses tags whose commit is
  not on main. Never add triggers for other branches or for pull requests.

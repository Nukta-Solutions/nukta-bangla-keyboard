import Foundation

/// Update checks: the counterpart of `windows/src/update.rs` and `linux/nukta_bangla/updates.py`,
/// with the same rules.
///
/// - The latest release comes from GitHub (`/releases/latest`): its tag is the version, and the
///   description's bullet lines (`- …` or `* …`, at most five) are what the dialog shows under
///   "নতুন যা আছে".
/// - Switching to the keyboard asks GitHub at most once a day, while `Settings.autoUpdateCheck` is
///   on. The dialog then waits a day before asking again ("পরে"), and a skipped version is never
///   offered again ("বাদ দিন").
/// - Settings' "এখনই খুঁজুন" asks straight away and always answers.
final class Updates {
    static let shared = Updates()

    static let apiURL = URL(string: "https://api.github.com/repos/Nukta-Solutions/nukta-bangla-keyboard/releases/latest")!
    /// The keyboard's page, which has every platform's installer.
    static let website = URL(string: "https://www.nuktasolutions.com/nukta-bangla-keyboard")!
    /// The macOS installer's name in every release (CLAUDE.md: never renamed).
    static let installerAsset = "NuktaBangla.dmg"
    /// How long between automatic checks, and how long "পরে" waits.
    static let day: TimeInterval = 24 * 60 * 60
    /// Bullet lines shown in the dialog; the rest are on the release page.
    static let maxNotes = 5

    /// A published release.
    struct Release: Equatable {
        /// Without the tag's "v": 1.0.3.
        var version: String
        var notes: [String]
        /// The release's page on GitHub ("সব পরিবর্তন দেখুন").
        var url: URL
        /// The release's .dmg, if it has one: what ডাউনলোড fetches.
        var download: URL?
    }

    enum CheckResult {
        case found(Release), latest, failed
    }

    /// This build's version.
    static var current: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0"
    }

    private let defaults = UserDefaults.standard
    /// A request to GitHub is in flight.
    private var checking = false

    // MARK: Checking

    /// The user switched to the keyboard: asks GitHub if a day has passed since it last answered, or
    /// else shows what an earlier check found. Nothing at all while update checks are off.
    func checkIfDue() {
        guard Settings.autoUpdateCheck, !checking else { return }
        let now = Date()
        if let last = lastCheck, last <= now, now.timeIntervalSince(last) < Self.day {
            showIfPending()
            return
        }
        fetch { [weak self] release in
            guard let self else { return }
            if let release { self.record(release) }
            self.showIfPending()
        }
    }

    /// "এখনই খুঁজুন": asks GitHub now, whatever the daily check last did.
    func checkNow(_ done: @escaping (CheckResult) -> Void) {
        fetch { [weak self] release in
            guard let self, let release else { return done(.failed) }
            self.record(release)
            done(Self.isNewer(release.version, than: Self.current) ? .found(release) : .latest)
        }
    }

    /// Opens the update dialog if there is a new version the user hasn't skipped or put off. It
    /// opens without taking the keyboard, so typing in the app underneath goes on.
    private func showIfPending() {
        guard Settings.autoUpdateCheck, let release = pending(now: Date()) else { return }
        UpdateWindow.shared.show(release, activate: false)
    }

    private func fetch(_ done: @escaping (Release?) -> Void) {
        checking = true
        var request = URLRequest(url: Self.apiURL, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 15)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("NuktaBangla/\(Self.current) (macOS)", forHTTPHeaderField: "User-Agent")
        URLSession.shared.dataTask(with: request) { data, response, _ in
            let ok = (response as? HTTPURLResponse)?.statusCode == 200
            let release = ok ? data.flatMap(Self.parseRelease) : nil
            DispatchQueue.main.async {
                self.checking = false
                done(release)
            }
        }.resume()
    }

    // MARK: What was found and what the user chose (UserDefaults)

    /// When GitHub last answered.
    private var lastCheck: Date? {
        defaults.object(forKey: "updateLastCheck") as? Date
    }

    var latest: Release? {
        guard let version = defaults.string(forKey: "updateLatestVersion") else { return nil }
        let url = defaults.string(forKey: "updateLatestURL").flatMap(URL.init(string:)) ?? Self.website
        return Release(version: version, notes: defaults.stringArray(forKey: "updateLatestNotes") ?? [], url: url,
                       download: defaults.string(forKey: "updateLatestDownload").flatMap(URL.init(string:)))
    }

    private func record(_ release: Release) {
        defaults.set(Date(), forKey: "updateLastCheck")
        defaults.set(release.version, forKey: "updateLatestVersion")
        defaults.set(release.notes, forKey: "updateLatestNotes")
        defaults.set(release.url.absoluteString, forKey: "updateLatestURL")
        defaults.set(release.download?.absoluteString, forKey: "updateLatestDownload")
    }

    /// The release worth telling the user about, if any: newer than this build, not skipped, and
    /// not put off.
    func pending(now: Date) -> Release? {
        guard let release = latest, Self.isNewer(release.version, than: Self.current),
              defaults.string(forKey: "updateSkippedVersion") != release.version else { return nil }
        let remindAfter = defaults.object(forKey: "updateRemindAfter") as? Date ?? .distantPast
        return now >= remindAfter ? release : nil
    }

    /// The dialog is on screen: not again for a day, whatever is clicked.
    func markShown() {
        defaults.set(Date().addingTimeInterval(Self.day), forKey: "updateRemindAfter")
    }

    func skip(_ version: String) {
        defaults.set(version, forKey: "updateSkippedVersion")
    }

    // MARK: Reading GitHub's answer

    /// GitHub's answer for the latest release, or nil if it isn't one.
    static func parseRelease(_ data: Data) -> Release? {
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let tag = object["tag_name"] as? String else { return nil }
        var version = tag.trimmingCharacters(in: .whitespaces)
        if version.first == "v" || version.first == "V" { version.removeFirst() }
        guard !versionParts(version).isEmpty else { return nil }
        let url = (object["html_url"] as? String).flatMap(URL.init(string:)) ?? website
        let assets = object["assets"] as? [[String: Any]] ?? []
        let download = assets.first { $0["name"] as? String == installerAsset }
            .flatMap { $0["browser_download_url"] as? String }
            .flatMap(URL.init(string:))
        return Release(version: version, notes: notes(from: object["body"] as? String ?? ""), url: url,
                       download: download)
    }

    /// The bullet lines of a release description, without their bullets or Markdown.
    static func notes(from body: String) -> [String] {
        var notes: [String] = []
        for line in body.components(separatedBy: .newlines) where notes.count < maxNotes {
            let line = line.trimmingCharacters(in: .whitespaces)
            guard line.hasPrefix("- ") || line.hasPrefix("* ") else { continue }
            let text = plain(String(line.dropFirst(2)))
            if !text.isEmpty { notes.append(text) }
        }
        return notes
    }

    /// A line of Markdown as plain text: **bold**, __bold__ and `code` lose their marks, and
    /// [a link](url) keeps only its text.
    static func plain(_ line: String) -> String {
        line.replacingOccurrences(of: #"\[([^\]]*)\]\([^)]*\)"#, with: "$1", options: .regularExpression)
            .replacingOccurrences(of: "**", with: "")
            .replacingOccurrences(of: "__", with: "")
            .replacingOccurrences(of: "`", with: "")
            .trimmingCharacters(in: .whitespaces)
    }

    /// 1.0.3 → [1, 0, 3]. Anything after a "-" or "+" (1.1.0-beta) is ignored; so is a leading "v".
    static func versionParts(_ version: String) -> [Int] {
        var core = Substring(version.trimmingCharacters(in: .whitespaces))
        if core.first == "v" || core.first == "V" { core = core.dropFirst() }
        core = core.split(whereSeparator: { $0 == "-" || $0 == "+" }).first ?? ""
        var parts: [Int] = []
        for part in core.split(separator: ".", omittingEmptySubsequences: false) {
            guard let number = Int(part) else { return [] }
            parts.append(number)
        }
        return parts
    }

    /// True if `candidate` is a later version than `current` (1.0.10 is later than 1.0.9).
    static func isNewer(_ candidate: String, than current: String) -> Bool {
        var a = versionParts(candidate), b = versionParts(current)
        guard !a.isEmpty, !b.isEmpty else { return false }
        let length = max(a.count, b.count)
        a += Array(repeating: 0, count: length - a.count)
        b += Array(repeating: 0, count: length - b.count)
        return b.lexicographicallyPrecedes(a)
    }

    /// A version in Bangla digits (১.০.৩).
    static func banglaDigits(_ text: String) -> String {
        String(String.UnicodeScalarView(text.unicodeScalars.map { (scalar: Unicode.Scalar) -> Unicode.Scalar in
            guard (0x30...0x39).contains(scalar.value) else { return scalar }
            return Unicode.Scalar(0x09E6 + scalar.value - 0x30) ?? scalar
        }))
    }
}

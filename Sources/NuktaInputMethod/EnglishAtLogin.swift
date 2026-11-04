import Cocoa
import Carbon

/// Every login starts in the Mac's English keyboard, not in Bangla: when the Mac logs out,
/// restarts or shuts down with নুকতা বাংলা selected, it switches to the English keyboard, which
/// macOS then brings back at the next login. Switching back to নুকতা বাংলা brings the layout last
/// picked (Settings.layout). Done on the way out, not at login, so a user who picks নুকতা বাংলা
/// straight after logging in is never switched away.
enum EnglishAtLogin {
    static func install() {
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.willPowerOffNotification, object: nil, queue: .main
        ) { _ in
            selectEnglish()
        }
    }

    private static func selectEnglish() {
        guard let current = TISCopyCurrentKeyboardInputSource()?.takeRetainedValue(),
              string(current, kTISPropertyBundleID) == Bundle.main.bundleIdentifier,
              // The English layout the Mac uses for shortcuts: ABC, U.S. …
              let english = TISCopyCurrentASCIICapableKeyboardLayoutInputSource()?.takeRetainedValue()
        else { return }
        let status = TISSelectInputSource(english)
        log.notice("logging out: English keyboard selected (\(status))")
    }

    private static func string(_ source: TISInputSource, _ key: CFString) -> String? {
        guard let value = TISGetInputSourceProperty(source, key) else { return nil }
        return Unmanaged<CFString>.fromOpaque(value).takeUnretainedValue() as String
    }
}

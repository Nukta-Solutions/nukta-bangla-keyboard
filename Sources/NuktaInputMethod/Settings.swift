import Foundation
import NuktaPhonetic

/// The keyboard layout, shared by every app and kept across restarts.
enum Layout: String {
    case bijoy, phonetic
}

/// Where the suggestion list opens.
enum PopupPosition: String {
    case below, above
}

/// How the suggestions are laid out.
enum PopupDirection: String {
    case vertical, horizontal
}

/// User settings, stored in UserDefaults. Changing one posts `.nuktaSettingsChanged`.
enum Settings {
    private static let defaults = UserDefaults.standard

    static var layout: Layout {
        get { value("layout", default: .bijoy) }
        set { set("layout", newValue.rawValue) }
    }

    static var typingMode: TypingMode {
        get { value("typingMode", default: .phoneticFirst) }
        set { set("typingMode", newValue.rawValue) }
    }

    static var autocorrect: Bool {
        get { bool("autocorrect") }
        set { set("autocorrect", newValue) }
    }

    static var emoji: Bool {
        get { bool("emoji") }
        set { set("emoji", newValue) }
    }

    static var englishWord: Bool {
        get { bool("englishWord") }
        set { set("englishWord", newValue) }
    }

    static var colonIsBisarga: Bool {
        get { defaults.bool(forKey: "colonIsBisarga") }
        set { set("colonIsBisarga", newValue) }
    }

    static var popupPosition: PopupPosition {
        get { value("popupPosition", default: .below) }
        set { set("popupPosition", newValue.rawValue) }
    }

    static var popupDirection: PopupDirection {
        get { value("popupDirection", default: .vertical) }
        set { set("popupDirection", newValue.rawValue) }
    }

    static var phoneticOptions: PhoneticOptions {
        var options = PhoneticOptions()
        options.mode = typingMode
        options.autocorrect = autocorrect
        options.emoji = emoji
        options.englishWord = englishWord
        options.colonIsBisarga = colonIsBisarga
        return options
    }

    /// Where riti and the pick memory keep what they learn.
    static var dataDirectory: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Nukta Bangla")
    }

    /// Brings over, once, the settings saved under the old com.asifmahmud bundle ID.
    static func migrateFromOldBundleID() {
        let old = "com.asifmahmud.inputmethod.NuktaBangla"
        guard let current = Bundle.main.bundleIdentifier, current != old,
              defaults.persistentDomain(forName: current) == nil,
              let saved = defaults.persistentDomain(forName: old) else { return }
        defaults.setPersistentDomain(saved, forName: current)
    }

    private static func value<T: RawRepresentable>(_ key: String, default fallback: T) -> T where T.RawValue == String {
        defaults.string(forKey: key).flatMap(T.init(rawValue:)) ?? fallback
    }

    /// Switches are on unless turned off.
    private static func bool(_ key: String) -> Bool {
        defaults.object(forKey: key) as? Bool ?? true
    }

    private static func set(_ key: String, _ value: Any) {
        defaults.set(value, forKey: key)
        NotificationCenter.default.post(name: .nuktaSettingsChanged, object: nil)
    }
}

extension Notification.Name {
    static let nuktaSettingsChanged = Notification.Name("NuktaSettingsChanged")
}

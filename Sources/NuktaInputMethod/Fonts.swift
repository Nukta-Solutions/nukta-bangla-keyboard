import Cocoa
import CoreText

/// The brand's Bangla font, Noto Serif Bengali, shipped inside the app (Resources/Fonts) and
/// registered for this process only: it is never installed in macOS.
///
/// Text keeps the system font, so English stays in SF; Bangla letters fall through to Noto Serif
/// Bengali (a cascade list), and to the system's Bangla font if the file is missing.
enum Fonts {
    static let banglaFamily = "Noto Serif Bengali"

    /// Registers the bundled font. Called once at start-up; harmless if the file isn't there.
    static func register() {
        guard let url = Bundle.main.url(forResource: "NotoSerifBengali", withExtension: "ttf",
                                        subdirectory: "Fonts") else { return }
        CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
    }

    /// The system font at `size` and `weight`, with Bangla in Noto Serif Bengali.
    static func ui(_ size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        withBangla(.systemFont(ofSize: size, weight: weight), weight: weight)
    }

    /// `font` with Bangla in Noto Serif Bengali.
    static func withBangla(_ font: NSFont, weight: NSFont.Weight = .regular) -> NSFont {
        let bangla = NSFontDescriptor(fontAttributes: [
            .family: banglaFamily,
            .traits: [NSFontDescriptor.TraitKey.weight: weight],
        ])
        let descriptor = font.fontDescriptor.addingAttributes([.cascadeList: [bangla]])
        return NSFont(descriptor: descriptor, size: font.pointSize) ?? font
    }
}

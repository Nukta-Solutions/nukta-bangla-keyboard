import XCTest
@testable import NuktaPhonetic

/// Plays keys into a composer the way the input controller does, keeping the app's text:
/// committed text, plus the marked (underlined) text still being composed.
private final class Rig {
    let composer: PhoneticComposer
    let directory: URL
    var doc = ""
    var marked = ""

    init(_ configure: (inout PhoneticOptions) -> Void = { _ in }, directory: URL? = nil) {
        var options = PhoneticOptions()
        configure(&options)
        self.directory = directory ?? FileManager.default.temporaryDirectory
            .appendingPathComponent("nukta-tests-\(UUID().uuidString)")
        composer = PhoneticComposer(options: options, directory: self.directory)
    }

    func apply(_ out: PhoneticComposer.Output, appText: String = "") {
        doc += out.insert
        marked = out.marked
        if !out.handled { doc += appText }
    }

    /// Letters, digits and punctuation; `␣` space, `⏎` Enter, `⎋` Escape, `⌫` backspace,
    /// `↓`/`↑` move the selection.
    @discardableResult
    func type(_ keys: String) -> Rig {
        for ch in keys {
            switch ch {
            case "␣": apply(composer.handle(.space), appText: " ")
            case "⏎": apply(composer.handle(.enter), appText: "\n")
            case "⎋": apply(composer.handle(.escape))
            case "⌫":
                let out = composer.handle(.backspace(wholeWord: false))
                if out.handled { apply(out) } else if !doc.isEmpty { doc.removeLast() }
            case "↓": apply(composer.handle(.down))
            case "↑": apply(composer.handle(.up))
            default: apply(composer.handle(.character(ch, shift: ch.isUppercase)), appText: String(ch))
            }
        }
        return self
    }

    /// Everything on screen once the word in progress is committed.
    var finished: String {
        apply(composer.commit())
        return doc
    }
}

final class PhoneticComposerTests: XCTestCase {
    func testPhoneticFirstCommitsTheTransliteration() {
        XCTAssertEqual(Rig().type("ami␣banglay␣likhchi").finished, "আমি বাংলায় লিখছি")
        // Without dictionary help, `sonar` is সনার, exactly as typed.
        XCTAssertEqual(Rig().type("sonar␣").doc, "সনার ")
    }

    func testMarkedTextFollowsTheWord() {
        let rig = Rig().type("kh")
        XCTAssertEqual(rig.marked, "খ")
        XCTAssertEqual(rig.doc, "")
        rig.type("i")
        XCTAssertEqual(rig.marked, "খি")
        XCTAssertEqual(rig.composer.candidates.first, "খি")
        XCTAssertEqual(rig.composer.auxiliary, "khi")
    }

    func testListOffersDictionaryWords() {
        let rig = Rig().type("sonar")
        XCTAssertEqual(rig.composer.candidates.first, "সনার")
        XCTAssertTrue(rig.composer.candidates.contains("সোনার"), "\(rig.composer.candidates)")
        XCTAssertEqual(rig.composer.selectedIndex, 0)
    }

    func testNumberKeyPicksACandidate() {
        let rig = Rig().type("sonar")
        let index = rig.composer.candidates.firstIndex(of: "সোনার")!
        rig.type(String(index + 1))
        XCTAssertEqual(rig.doc, "সোনার")
        XCTAssertFalse(rig.composer.isComposing)
    }

    func testArrowsMoveTheSelectionAndEnterCommitsWithoutNewline() {
        let rig = Rig().type("sonar↓")
        XCTAssertEqual(rig.composer.selectedIndex, 1)
        let second = rig.composer.candidates[1]
        XCTAssertEqual(rig.marked, second)
        rig.type("↑↓⏎")
        XCTAssertEqual(rig.doc, second)
    }

    func testPhoneticFirstRemembersAPick() {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("nukta-tests-\(UUID().uuidString)")
        let rig = Rig(directory: directory).type("sonar")
        let index = rig.composer.candidates.firstIndex(of: "সোনার")!
        rig.type(String(index + 1))
        rig.type("␣sonar␣")
        XCTAssertEqual(rig.doc, "সোনার সোনার ")

        // Picking the plain transliteration again forgets it.
        rig.type("sonar1␣sonar␣")
        XCTAssertEqual(rig.doc, "সোনার সোনার সনার সনার ")
    }

    func testSmartModeCommitsTheDictionaryWord() {
        XCTAssertEqual(Rig { $0.mode = .smart }.type("sonar␣").doc, "সোনার ")
    }

    func testAutocorrectSwitch() {
        // riti's autocorrect list spells `account` as `oZakaunT` (অ্যাকাউন্ট).
        let on = Rig { $0.mode = .smart }.type("account")
        XCTAssertEqual(on.composer.candidates.first, "অ্যাকাউন্ট")
        XCTAssertEqual(on.type("␣").doc, "অ্যাকাউন্ট ")

        let off = Rig { $0.mode = .smart; $0.autocorrect = false }.type("account")
        XCTAssertFalse(off.composer.candidates.contains("অ্যাকাউন্ট"), "\(off.composer.candidates)")
        XCTAssertEqual(off.type("␣").doc, "আচ্চউন্ত ")
    }

    func testPhoneticOnlyHasNoList() {
        let rig = Rig { $0.mode = .phoneticOnly }.type("sonar")
        XCTAssertEqual(rig.marked, "সনার")
        XCTAssertTrue(rig.composer.candidates.isEmpty)
        rig.type("↓")
        XCTAssertEqual(rig.doc, "সনার")
    }

    func testEscapeDropsTheWord() {
        let rig = Rig().type("ami␣sonar⎋")
        XCTAssertEqual(rig.doc, "আমি ")
        XCTAssertEqual(rig.marked, "")
        XCTAssertFalse(rig.composer.isComposing)
    }

    func testBackspaceEditsTheWordThenTheApp() {
        XCTAssertEqual(Rig().type("khi⌫a").finished, "খা")
        XCTAssertEqual(Rig().type("ami␣⌫").finished, "আমি")
    }

    func testDigitsAreBanglaOutsideAWord() {
        XCTAssertEqual(Rig().type("2024␣").doc, "২০২৪ ")
    }

    func testPunctuationEndsAWord() {
        XCTAssertEqual(Rig().type("ki?").finished, "কি?")
        XCTAssertEqual(Rig().type("ami.").finished, "আমি।")
    }

    func testEmojiOnlyByExplicitPickAndCanBeHidden() {
        let with = Rig().type("hasi")
        XCTAssertFalse(PhoneticComposer.containsEmoji(with.composer.candidates[with.composer.selectedIndex]))
        let without = Rig { $0.emoji = false }.type("hasi")
        XCTAssertFalse(without.composer.candidates.contains(where: PhoneticComposer.containsEmoji))
    }

    /// The examples in README.md.
    func testReadmeExamples() {
        XCTAssertEqual(Rig().type("kotha␣dhorrmo␣prem").finished, "কথা ধর্ম প্রেম")
        XCTAssertTrue(Rig().type("hasi").composer.candidates.contains("😄"))
    }

    func testTextSymbolsShownAsEmojiCountAsEmoji() {
        XCTAssertTrue(PhoneticComposer.containsEmoji("©️"))
        XCTAssertTrue(PhoneticComposer.containsEmoji("😄"))
        XCTAssertFalse(PhoneticComposer.containsEmoji("©"))
        XCTAssertFalse(PhoneticComposer.containsEmoji("সোনার"))
        XCTAssertFalse(PhoneticComposer.containsEmoji("১২#*"))
    }

    func testSmartModeNeverCommitsAnEmojiByDefault() {
        // riti offers ©️ for `st`.
        let doc = Rig { $0.mode = .smart }.type("st␣").doc
        XCTAssertFalse(PhoneticComposer.containsEmoji(doc), doc)
    }

    func testColonTypesAColonByDefault() {
        XCTAssertEqual(Rig().type("somoy:␣10").finished, "সময়: ১০")
        XCTAssertEqual(Rig().type(":").finished, ":")
        // ঃ words still come from the list.
        XCTAssertTrue(Rig().type("dukho").composer.candidates.contains("দুঃখ"))
    }

    func testColonCanTypeBisarga() {
        XCTAssertEqual(Rig { $0.colonIsBisarga = true }.type("du:kho").finished, "দুঃখ")
    }

    func testDisplayOnlyOptionsKeepTheWord() {
        let rig = Rig().type("sonar")
        var options = rig.composer.options
        options.colonIsBisarga = true
        options.emoji = false
        rig.composer.update(options: options)
        XCTAssertTrue(rig.composer.isComposing)
    }

    func testOptionsChangeDropsTheWord() {
        let rig = Rig().type("sonar")
        var options = rig.composer.options
        options.mode = .smart
        rig.composer.update(options: options)
        XCTAssertFalse(rig.composer.isComposing)
        XCTAssertEqual(rig.type("sonar␣").doc, "সোনার ")
    }

    // MARK: The user's shortcuts (riti-bridge/src/shortcuts.rs)

    private let salam = "আসসালামু আলাইকুম"

    /// A data directory with `asw` → আসসালামু আলাইকুম.
    private func shortcutDirectory() -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("nukta-tests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        XCTAssertNoThrow(try Shortcuts(directory: directory).set(code: "asw", text: salam).get())
        return directory
    }

    func testAShortcutComesFirstAndThePlainWordSecond() {
        let plain = Rig().type("asw").composer.candidates[0]
        let directory = shortcutDirectory()
        let rig = Rig(directory: directory).type("asw")
        XCTAssertEqual(rig.composer.candidates[0], salam)
        XCTAssertEqual(rig.composer.candidates[1], plain)
        XCTAssertEqual(rig.marked, salam)
        XCTAssertEqual(rig.type("␣").doc, salam + " ")
        // 2 types the plain word.
        XCTAssertEqual(Rig(directory: directory).type("asw2").doc, plain)
    }

    func testOnlyTheWholeCodeOffersAShortcut() {
        let directory = shortcutDirectory()
        let rig = Rig(directory: directory).type("as")
        XCTAssertFalse(rig.composer.candidates.contains(salam))
        rig.type("wa")
        XCTAssertFalse(rig.composer.candidates.contains(salam))
        // Any case.
        XCTAssertEqual(Rig(directory: directory).type("Asw").composer.candidates[0], salam)
    }

    func testAShortcutWorksInSmartModeToo() {
        let rig = Rig({ $0.mode = .smart }, directory: shortcutDirectory())
        XCTAssertEqual(rig.type("asw␣").doc, salam + " ")
    }

    func testPunctuationStaysAroundAShortcut() {
        let directory = shortcutDirectory()
        XCTAssertEqual(Rig(directory: directory).type("asw,␣").doc, salam + ", ")
        // The plain word, picked before the comma, stays picked.
        let rig = Rig(directory: directory).type("asw↓")
        let plain = rig.composer.candidates[1]
        XCTAssertEqual(rig.type(",␣").doc, plain + ", ")
    }

    func testAShortcutAddedMeanwhileIsOfferedAndNotRememberedAsAPick() {
        let rig = Rig()
        let plain = rig.type("asw").composer.candidates[0]
        rig.type("⎋")
        // The settings add it while the app is open.
        let shortcuts = Shortcuts(directory: rig.directory)
        XCTAssertNoThrow(try shortcuts.set(code: "asw", text: salam).get())
        rig.type("asw␣")
        XCTAssertEqual(rig.doc, salam + " ")
        // Removed: the plain word again, not a remembered pick of the shortcut's text.
        shortcuts.remove(code: "asw")
        rig.type("asw␣")
        XCTAssertEqual(rig.doc, "\(salam) \(plain) ")
    }

    func testDictionaryWordsAreNamed() {
        XCTAssertEqual(Shortcuts.dictionaryWord(for: "ami"), "আমি")
        XCTAssertNil(Shortcuts.dictionaryWord(for: "tnx"))
    }
}

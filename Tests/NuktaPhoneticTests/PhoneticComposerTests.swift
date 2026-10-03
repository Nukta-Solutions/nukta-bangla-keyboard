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
}

import XCTest
@testable import BijoyEngine

/// `⌫` in a key string means backspace.
private func type(_ keys: String, _ order: KarOrder = .classic) -> String {
    let engine = Engine(karOrder: order)
    var doc = ""
    for ch in keys {
        if ch == "⌫" {
            if engine.backspace() == nil, !doc.unicodeScalars.isEmpty {
                doc.unicodeScalars.removeLast()
            }
            continue
        }
        if let out = engine.process(character: ch) {
            doc += out.committed
        } else {
            doc += engine.commit().committed + String(ch)
        }
    }
    return doc + engine.commit().committed
}

/// Same, but plays the edits through TextDiff the way the direct-insert writer does.
private func typeViaDiffs(_ keys: String, _ order: KarOrder = .classic) -> String {
    let engine = Engine(karOrder: order)
    var doc = ""
    var shown = ""
    func apply(_ out: Output) {
        let edit = TextDiff.edit(from: shown, to: out.committed + out.display)
        XCTAssertTrue(String(doc.unicodeScalars.suffix(edit.deleted.unicodeScalars.count)) == edit.deleted)
        doc.unicodeScalars.removeLast(edit.deleted.unicodeScalars.count)
        doc += edit.inserted
        shown = out.display
    }
    for ch in keys {
        if ch == "⌫" {
            if let out = engine.backspace() { apply(out) } else if !doc.isEmpty { doc.unicodeScalars.removeLast() }
            continue
        }
        if let out = engine.process(character: ch) {
            apply(out)
        } else {
            apply(engine.commit())
            shown = ""
            doc += String(ch)
        }
    }
    apply(engine.commit())
    return doc
}

private func scalars(_ s: String) -> String {
    s.unicodeScalars.map { String(format: "U+%04X", $0.value) }.joined(separator: " ")
}

final class EngineTests: XCTestCase {
    let cases: [(keys: String, expected: String)] = [
        // basic kar + reordering of pre-base kars
        ("j", "ক"),
        ("jf", "কা"),
        ("dj", "কি"),
        ("jD", "কী"),
        ("cj", "কে"),
        ("Cj", "কৈ"),
        ("cjf", "কো"),
        ("cjX", "কৌ"),
        ("js", "কু"),
        ("jS", "কূ"),
        ("ja", "কৃ"),
        // juktakkhor
        ("djgk", "ক্তি"),
        ("jgN", "ক্ষ"),
        ("ngkz", "স্ত্র"),
        ("cjgNkz", "ক্ষেত্র"),
        ("nQngjadk", "সংস্কৃতি"),
        ("gsugughV", "উজ্জ্বল"),
        ("ug", "জ্"),
        ("jgg", "ক্\u{200C}"),
        ("uIf", "জঞা"),
        ("ugIfb", "জ্ঞান"),
        ("dhugIfb", "বিজ্ঞান"),
        ("jaNgB", "কৃষ্ণ"),
        ("nmgmfb", "সম্মান"),
        ("LbZhfl", "ধন্যবাদ"),
        ("rvDjgNf", "পরীক্ষা"),
        ("cMzdB", "শ্রেণি"),
        ("cmgr", "ম্পে"),
        // phala
        ("ozfm", "গ্রাম"),
        ("dhlZfVW", "বিদ্যাল\u{09DF}"),
        ("hZfQj", "ব্যাংক"),
        ("gCjZ", "ঐক্য"),
        ("vZfh", "র\u{200D}\u{09CD}যাব"),
        // reph
        ("mA", "র্ম"),
        ("jmA", "কর্ম"),
        ("djA", "র্কি"),
        ("FKA", "অর্থ"),
        ("jkAarjgN", "কর্তৃপক্ষ"),
        ("gSLghA", "ঊর্ধ্ব"),
        // independent vowels
        ("gf", "আ"), ("gd", "ই"), ("gD", "ঈ"), ("gs", "উ"), ("gS", "ঊ"),
        ("ga", "ঋ"), ("gc", "এ"), ("gC", "ঐ"), ("gX", "ঔ"), ("F", "অ"), ("x", "ও"),
        ("jgd", "কই"),
        ("gXNL", "ঔষধ"),
        ("gaB", "ঋণ"),
        ("gDl", "ঈদ"),
        ("xcj", "ওকে"),
        // x after a consonant → ো
        ("jx", "কো"),
        ("jxb", "কোন"),
        ("Hfvx", "ভারো"),
        ("jx&", "কোঁ"),
        ("mAx", "র্মো"),
        ("ozx", "গ্রো"),
        ("jgx", "কও"),
        ("j x", "ক ও"),
        ("cjx", "কেও"),
        ("jx⌫", "ক"),
        // X after a consonant → ৌ
        ("jX", "কৌ"),
        ("cjX", "কৌ"),
        ("mXn", "মৌস"),
        ("ozX", "গ্রৌ"),
        // signs
        ("hfQVf", "বাংলা"),
        ("yf&l", "চাঁদ"),
        ("cjf&", "কোঁ"),
        ("crX&Yfcbf", "পৌঁছানো"),
        ("j|", "কঃ"),
        ("iTf\\", "হঠাৎ"),
        // words and sentences
        ("gfmfv ncfbfv hfQVf", "আমার সোনার বাংলা"),
        ("hfQVfclM", "বাংলাদেশ"),
        ("mfbsN", "মানুষ"),
        ("HfcVfhfnf", "ভালোবাসা"),
        ("gfmd hfQVf dVdJ|", "আমি বাংলা লিখিঃ"),
        ("gfmd hfQVf dVdJG", "আমি বাংলা লিখি।"),
        // digits and symbols
        ("123", "১২৩"),
        ("$100", "৳১০০"),
        ("j, K.", "ক, থ।"),
        ("gfmd .", "আমি ।"),
        // F f → আ
        ("Ff", "আ"),
        ("Ffm", "আম"),
        ("FfmfV", "আমাল"),
        ("Ffmfv ncfbfv", "আমার সোনার"),
        ("Fj", "অক"),
        ("FF", "অঅ"),
        ("F ", "অ "),
        // a kar with nothing to attach to is kept, not lost
        ("f", "\u{09BE}"),
        ("d", "\u{09BF}"),
        ("d ", "\u{09BF} "),
        ("A", "র্"),
        ("Aj", "র্ক"),
    ]

    func testWords() {
        for c in cases {
            let got = type(c.keys)
            XCTAssertEqual(scalars(got), scalars(c.expected), "keys: \(c.keys) → \(got), expected \(c.expected)")
        }
    }

    /// Kar-after-consonant mode: j c → কে. Same rules otherwise.
    let afterConsonantCases: [(keys: String, expected: String)] = [
        ("jc", "কে"),
        ("jd", "কি"),
        ("jC", "কৈ"),
        ("jD", "কী"),
        ("jcf", "কো"),
        ("jx", "কো"),
        ("jX", "কৌ"),
        ("jgkd", "ক্তি"),
        ("jgNckz", "ক্ষেত্র"),
        ("jVcu", "কলেজ"),
        ("hfQVflcM", "বাংলাদেশ"),
        ("hdlZfVW", "বিদ্যাল\u{09DF}"),
        ("jAd", "র্কি"),
        ("jdA", "র্কি"),
        ("Mzcd", "শ্রে\u{09BF}"),
        ("MzcBd", "শ্রেণি"),
        ("gfmd", "আমি"),
        ("jgd", "কই"),
        ("jc&", "কেঁ"),
        ("d", "\u{09BF}"),
        ("d j", "\u{09BF} ক"),
        ("jcg", "কে্"),
        ("jc⌫", "ক"),
    ]

    func testAfterConsonantMode() {
        for c in afterConsonantCases {
            XCTAssertEqual(scalars(type(c.keys, .afterConsonant)), scalars(c.expected), "keys: \(c.keys)")
            XCTAssertEqual(scalars(typeViaDiffs(c.keys, .afterConsonant)), scalars(c.expected), "keys: \(c.keys) (diff path)")
        }
    }

    func testDirectInsertDiffsProduceSameText() {
        for c in cases {
            let got = typeViaDiffs(c.keys)
            XCTAssertEqual(scalars(got), scalars(c.expected), "keys: \(c.keys) (diff path)")
        }
    }

    func testBackspaceUndoesKeystrokes() {
        XCTAssertEqual(type("djgk⌫⌫"), "কি")
        XCTAssertEqual(type("djA⌫"), "কি")
        XCTAssertEqual(type("cjf⌫"), "কে")
        XCTAssertEqual(type("jf⌫"), "ক")
        XCTAssertEqual(type("dj⌫j"), "কি")
        XCTAssertEqual(type("jf ⌫"), "কা")        // committed text: app-style delete
        XCTAssertEqual(type("gf⌫"), "")
        XCTAssertEqual(type("F⌫"), "")            // full vowel is committed; app deletes it
        XCTAssertEqual(typeViaDiffs("djgk⌫⌫"), "কি")
        XCTAssertEqual(typeViaDiffs("cjfX⌫"), "কো")
    }

    func testDisplayWhileComposing() {
        let e = Engine()
        XCTAssertEqual(e.process(character: "d")?.display, "\u{09BF}")
        XCTAssertEqual(e.process(character: "j")?.display, "কি")
        XCTAssertEqual(e.process(character: "g")?.display, "ক্\u{09BF}")
        XCTAssertEqual(e.process(character: "k")?.display, "ক্তি")
        let next = e.process(character: "h")!
        XCTAssertEqual(next.committed, "ক্তি")
        XCTAssertEqual(next.display, "ব")
    }

    func testNonBijoyKeysReturnNil() {
        let e = Engine()
        XCTAssertNil(e.process(character: " "))
        XCTAssertNil(e.process(character: "!"))
        XCTAssertNil(e.process(character: "\n"))
    }

    func testTextDiff() {
        XCTAssertEqual(TextDiff.edit(from: "\u{09BF}", to: "কি"), .init(deleted: "\u{09BF}", inserted: "কি"))
        XCTAssertEqual(TextDiff.edit(from: "ক", to: "কা"), .init(deleted: "", inserted: "\u{09BE}"))
        XCTAssertEqual(TextDiff.edit(from: "ম", to: "র্ম"), .init(deleted: "ম", inserted: "র্ম"))
    }
}

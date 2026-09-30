import XCTest
@testable import BijoyEngine

/// `⌫` in a key string means backspace.
private func type(_ keys: String, _ order: KarOrder = .mixed) -> String {
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

/// Same, but plays the edits through TextDiff the way the direct-insert writer does,
/// keeping the document exactly as an app would hold it. `rewrites` counts edits that had
/// to replace text already on screen (the ones some editors ignore).
private func typeWithEdits(_ keys: String, _ order: KarOrder = .mixed) -> (doc: String, rewrites: Int) {
    let engine = Engine(karOrder: order)
    var doc = ""
    var shown = ""
    var rewrites = 0
    func apply(_ out: Output) {
        let edit = TextDiff.edit(from: shown, to: out.committed + out.visible)
        if !edit.deleted.isEmpty {
            rewrites += 1
            XCTAssertTrue(String(doc.unicodeScalars.suffix(edit.deleted.unicodeScalars.count)) == edit.deleted)
        }
        doc = TextDiff.dropLast(edit.deleted.unicodeScalars.count, of: doc) + edit.inserted
        let tail = TextDiff.dropLast(edit.deleted.unicodeScalars.count, of: shown) + edit.inserted
        shown = TextDiff.remainder(of: tail, after: out.committed)
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
    return (doc, rewrites)
}

/// Compared with canonical equivalence: on screen ো may be ে + া.
private func typeViaDiffs(_ keys: String, _ order: KarOrder = .mixed) -> String {
    typeWithEdits(keys, order).doc
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
        ("ga", "ঋ"), ("gc", "এ"), ("gC", "ঐ"), ("gX", "ঔ"), ("F", "অ"), ("gx", "ও"), ("x", "\u{09CB}"),
        ("jgd", "কই"),
        ("gXNL", "ঔষধ"),
        ("gaB", "ঋণ"),
        ("gDl", "ঈদ"),
        ("gxcj", "ওকে"),
        // x after a consonant → ো
        ("jx", "কো"),
        ("jxb", "কোন"),
        ("Hfvx", "ভারো"),
        ("jx&", "কোঁ"),
        ("mAx", "র্মো"),
        ("ozx", "গ্রো"),
        ("jgx", "কও"),
        ("j gx", "ক ও"),
        ("j x", "ক \u{09CB}"),
        ("cjx", "কো"),
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
        for order in [KarOrder.mixed, .classic] {
            for c in cases {
                let got = type(c.keys, order)
                XCTAssertEqual(scalars(got), scalars(c.expected), "\(order) keys: \(c.keys) → \(got), expected \(c.expected)")
            }
        }
    }

    /// Mixed (default): both orders, as in Avro's Bijoy layout.
    let mixedCases: [(keys: String, expected: String)] = [
        // কো / কৌ / কি / কে / কৈ, every way round
        ("cjf", "কো"), ("jx", "কো"), ("jcf", "কো"), ("cjx", "কো"), ("jcx", "কো"),
        ("cjX", "কৌ"), ("jX", "কৌ"), ("jcX", "কৌ"),
        ("dj", "কি"), ("jd", "কি"),
        ("cj", "কে"), ("jc", "কে"),
        ("Cj", "কৈ"), ("jC", "কৈ"),
        // বিজয় typed both ways
        ("hduW", "বিজ\u{09DF}"),
        ("dhuW", "বিজ\u{09DF}"),
        // juktakkhor both ways
        ("djgk", "ক্তি"), ("jgkd", "ক্তি"),
        ("cjgNkz", "ক্ষেত্র"), ("jgNckz", "ক্ষেত্র"),
        ("Mzcd", "শ্রে\u{09BF}"), ("McBd", "শেণি"),
        ("Mzcbd", "শ্রেনি"), ("MzcBd", "শ্রেণি"), ("cMzdB", "শ্রেণি"),
        // kar after a bare consonant attaches to it
        ("jVcu", "কলেজ"), ("jcVu", "কেলজ"),
        ("mbc", "মনে"), ("mcb", "মেন"),
        ("hdlZfVW", "বিদ্যাল\u{09DF}"),
        ("hfQVflcM", "বাংলাদেশ"), ("hfQVfclM", "বাংলাদেশ"),
        ("LbZhfl", "ধন্যবাদ"),
        // reph both ways, before or after the kar
        ("jAd", "র্কি"), ("jdA", "র্কি"), ("djA", "র্কি"), ("vgjd", "র্কি"),
        ("mAx", "র্মো"), ("mxA", "র্মো"),
        // other
        ("gfmd", "আমি"),
        ("jgd", "কই"),
        ("jc&", "কেঁ"),
        ("d", "\u{09BF}"),
        ("d ", "\u{09BF} "),
        ("jcg", "কে্"),
        ("jc⌫", "ক"),
        ("hd⌫u", "বজ"),
    ]

    /// Strict classic: ি ে ৈ always wait for the next consonant.
    let classicCases: [(keys: String, expected: String)] = [
        ("jcVu", "কলেজ"),
        ("mcb", "মনে"),
        ("hduW", "বজি\u{09DF}"),
        ("jc ", "ক\u{09C7} "),
    ]

    func testMixedMode() {
        for c in mixedCases {
            XCTAssertEqual(scalars(type(c.keys, .mixed)), scalars(c.expected), "keys: \(c.keys) → \(type(c.keys, .mixed))")
            XCTAssertEqual(typeViaDiffs(c.keys, .mixed), c.expected, "keys: \(c.keys) (diff path)")
        }
    }

    func testClassicMode() {
        for c in classicCases {
            XCTAssertEqual(scalars(type(c.keys, .classic)), scalars(c.expected), "keys: \(c.keys) → \(type(c.keys, .classic))")
            XCTAssertEqual(typeViaDiffs(c.keys, .classic), c.expected, "keys: \(c.keys) (diff path)")
        }
    }

    func testDirectInsertDiffsProduceSameText() {
        for order in [KarOrder.mixed, .classic] {
            for c in cases {
                let got = typeViaDiffs(c.keys, order)
                XCTAssertEqual(got, c.expected, "\(order) keys: \(c.keys) (diff path)")
            }
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

    /// These must never rewrite text already on screen, so they work in editors that ignore
    /// rewrites (Facebook, Messenger…).
    func testCommonSpellingsNeedNoRewrite() {
        let words = [
            "Ofcnv",        // ঘাসের
            "cjf", "jcf", "jx", "cjX", "jcX", "jX",  // কো কৌ
            "dj", "jd", "cj", "jc", "Cj",
            "dhuW", "hduW",  // বিজয়
            "hfQVfclM", "hfQVflcM",  // বাংলাদেশ
            "Ff", "Ffmd", "gd", "gx", "jgd",
            "cnf&", "jVcu", "mbc",
            "crX&Yfcbf",  // পৌঁছানো
            "gfmfv ncfbfv hfQVf",
            "vgm", "jgkd", "ugug", "ugughV",  // র্ম ক্তি, juktakkhor in Unicode order
        ]
        for order in [KarOrder.mixed, .classic] {
            for w in words {
                XCTAssertEqual(typeWithEdits(w, order).rewrites, 0, "\(order) \(w) → \(typeWithEdits(w, order).doc)")
            }
        }
        for w in ["jgNckz", "jgkd", "LbZhfl"] {  // ক্ষেত্র, mixed only
            XCTAssertEqual(typeWithEdits(w, .mixed).rewrites, 0, "mixed \(w) → \(typeWithEdits(w, .mixed).doc)")
        }
    }

    func testRephAndLateJuktakkharStillRewrite() {
        XCTAssertEqual(typeWithEdits("mA").rewrites, 1)
        XCTAssertEqual(typeWithEdits("mA").doc, "র্ম")
        XCTAssertEqual(typeWithEdits("djgk").doc, "ক্তি")
    }

    func testTextDiff() {
        XCTAssertEqual(TextDiff.edit(from: "কে", to: "কো"), .init(deleted: "", inserted: "\u{09BE}"))
        XCTAssertEqual(TextDiff.edit(from: "কে", to: "কৌ"), .init(deleted: "", inserted: "\u{09D7}"))
        XCTAssertEqual(TextDiff.edit(from: "কে", to: "কো\u{09DF}").inserted.unicodeScalars.last, "\u{09DF}")
        XCTAssertEqual(TextDiff.edit(from: "\u{09BF}", to: "কি"), .init(deleted: "\u{09BF}", inserted: "কি"))
        XCTAssertEqual(TextDiff.edit(from: "ক", to: "কা"), .init(deleted: "", inserted: "\u{09BE}"))
        XCTAssertEqual(TextDiff.edit(from: "ম", to: "র্ম"), .init(deleted: "ম", inserted: "র্ম"))
    }
}

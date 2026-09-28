/// One in-progress syllable, stored in the order the keys were pressed (Bijoy/visual order).
/// `rendered` turns it into correct Unicode (logical) order.
struct Syllable: Equatable {
    enum Token: Equatable {
        case consonant(String)
        case hasanta
        case explicitHasanta
        case phala(String)
        case preKar(String)
        case postKar(String)
        case reph
        case sign(String)
        case vowel(String)
    }

    static let hasantaChar = "\u{09CD}"
    static let preBaseKars: Set<String> = ["\u{09BF}", "\u{09C7}", "\u{09C8}"] // ি ে ৈ

    /// `g` + kar → full vowel.
    static let independentVowels: [String: String] = [
        "\u{09BE}": "আ", "\u{09BF}": "ই", "\u{09C0}": "ঈ", "\u{09C1}": "উ", "\u{09C2}": "ঊ",
        "\u{09C3}": "ঋ", "\u{09C7}": "এ", "\u{09C8}": "ঐ", "\u{09CB}": "ও", "\u{09D7}": "ঔ",
    ]

    private(set) var tokens: [Token] = []

    var isEmpty: Bool { tokens.isEmpty }
    var hasCluster: Bool { tokens.contains { if case .consonant = $0 { return true }; return false } }
    var hasReph: Bool { tokens.contains(.reph) }
    var hasPreKar: Bool { tokens.contains { if case .preKar = $0 { return true }; return false } }
    var lastIsHasanta: Bool { tokens.last == .hasanta }
    /// A kar or sign after the consonants closes the cluster: no more consonants can join.
    /// (A pre-base kar typed first, classic style, doesn't.)
    var isClusterClosed: Bool {
        tokens.enumerated().contains { index, token in
            switch token {
            case .postKar, .sign, .explicitHasanta: return true
            case .preKar: return index > 0
            default: return false
            }
        }
    }

    mutating func append(_ token: Token) { tokens.append(token) }
    mutating func removeLast() { tokens.removeLast() }

    var rendered: String {
        var cluster = ""
        var pre = ""
        var post: [String] = []
        var signs = ""
        var reph = false

        for token in tokens {
            switch token {
            case .consonant(let c): cluster += c
            case .hasanta: cluster += Self.hasantaChar
            case .explicitHasanta: cluster += Self.hasantaChar + "\u{200C}"
            case .phala(let p):
                // র + ্য needs a ZWJ so it renders as র‍্য (র‍্যাব), not as reph + য.
                if p == Self.hasantaChar + "য", cluster.unicodeScalars.last == "র" {
                    cluster += "\u{200D}"
                }
                cluster += p
            case .preKar(let k): pre += k
            case .postKar(let k): post.append(k)
            case .reph: reph = true
            case .sign(let s): signs += s
            case .vowel(let v): cluster += v
            }
        }

        // Split vowels: ে … া → ো, ে … ৗ → ৌ, ে … ো → ো; ৗ on its own after a consonant → ৌ (j X → কৌ)
        if pre == "\u{09C7}", let first = post.first {
            if first == "\u{09BE}" || first == "\u{09CB}" { pre = "\u{09CB}"; post.removeFirst() }
            else if first == "\u{09D7}" { pre = "\u{09CC}"; post.removeFirst() }
        } else if pre.isEmpty, !cluster.isEmpty, post.first == "\u{09D7}" {
            post[0] = "\u{09CC}"
        }

        return (reph ? "র" + Self.hasantaChar : "") + cluster + pre + post.joined() + signs
    }
}

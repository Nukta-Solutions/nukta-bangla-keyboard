//! One in-progress syllable: a port of `Sources/NuktaEngine/Syllable.swift`.

/// A key of the syllable, stored in the order the keys were pressed (Bijoy/visual order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Token {
    Consonant(&'static str),
    Hasanta,
    ExplicitHasanta,
    Phala(&'static str),
    PreKar(&'static str),
    PostKar(&'static str),
    Reph,
    Sign(&'static str),
    Vowel(&'static str),
    /// ি ে ৈ pressed once after a consonant; not shown until the next key decides.
    WaitingKar(&'static str),
}

pub(crate) const HASANTA: &str = "\u{09CD}";

/// ি ে ৈ
pub(crate) fn is_pre_base_kar(kar: &str) -> bool {
    matches!(kar, "\u{09BF}" | "\u{09C7}" | "\u{09C8}")
}

/// `g` + kar → full vowel.
pub(crate) fn independent_vowel(kar: &'static str) -> &'static str {
    match kar {
        "\u{09BE}" => "আ", "\u{09BF}" => "ই", "\u{09C0}" => "ঈ", "\u{09C1}" => "উ", "\u{09C2}" => "ঊ",
        "\u{09C3}" => "ঋ", "\u{09C7}" => "এ", "\u{09C8}" => "ঐ", "\u{09CB}" => "ও", "\u{09D7}" => "ঔ",
        other => other,
    }
}

/// The keys of one syllable. `rendered` turns them into correct Unicode (logical) order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Syllable {
    pub(crate) tokens: Vec<Token>,
}

impl Syllable {
    pub(crate) fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub(crate) fn has_cluster(&self) -> bool {
        self.tokens.iter().any(|t| matches!(t, Token::Consonant(_)))
    }

    pub(crate) fn has_reph(&self) -> bool {
        self.tokens.contains(&Token::Reph)
    }

    pub(crate) fn has_pre_kar(&self) -> bool {
        self.tokens.iter().any(|t| matches!(t, Token::PreKar(_)))
    }

    pub(crate) fn last_is_hasanta(&self) -> bool {
        self.tokens.last() == Some(&Token::Hasanta)
    }

    /// A kar or sign after the consonants closes the cluster: no more consonants can join.
    /// (A pre-base kar typed first, classic style, doesn't.)
    pub(crate) fn is_cluster_closed(&self) -> bool {
        self.tokens.iter().enumerate().any(|(index, token)| match token {
            Token::PostKar(_) | Token::Sign(_) | Token::ExplicitHasanta => true,
            Token::PreKar(_) => index > 0,
            _ => false,
        })
    }

    /// A phala can still join the consonant: after a kar too (ট + ু + ্য = ট্যু, typed either way),
    /// but not after ং ঃ ঁ or a visible hasanta, which end the syllable.
    pub(crate) fn accepts_phala(&self) -> bool {
        self.has_cluster() && !self.tokens.iter().any(|t| matches!(t, Token::Sign(_) | Token::ExplicitHasanta))
    }

    pub(crate) fn push(&mut self, token: Token) {
        self.tokens.push(token);
    }

    pub(crate) fn pop(&mut self) {
        self.tokens.pop();
    }

    /// What is on screen while the syllable is being typed: everything, except that a trailing ্
    /// waits for the next key (it may turn into a full vowel: j g d → কই).
    pub(crate) fn visible_rendered(&self) -> String {
        if !self.has_cluster() {
            // অ (Shift+F) shows straight away; an f after it still turns it into আ.
            if matches!(self.tokens.first(), Some(Token::Vowel(_))) {
                return self.rendered();
            }
            // So does a ি ে ৈ typed before its consonant (c j → কে): ে now, কে with the j.
            // It sits on a no-break space, so it neither clings to the letter before it (হে + ে
            // showed as হেে) nor gets a dotted circle. The space is only shown, never typed.
            // Whatever follows it before the consonant shows too (c f → ো, c Q → েং).
            // Windows only; Swift and Python still hold it back.
            if matches!(self.tokens.first(), Some(Token::PreKar(_))) {
                return format!("\u{00A0}{}", self.rendered());
            }
            return String::new();
        }
        let waiting = match self.tokens.last() {
            Some(Token::WaitingKar(k)) => Some(*k),
            _ => None,
        };
        if !self.last_is_hasanta() && waiting.is_none() {
            return self.rendered();
        }
        let mut without_link = self.clone();
        without_link.pop();
        match waiting {
            // A ি ে ৈ pressed once after a consonant (i c in হরেক, হয়ে) most often waits for the
            // next one: shown apart, on a no-break space, like one typed first. Pressed again
            // it joins this consonant (j c c → কে).
            Some(kar) => format!("{}\u{00A0}{kar}", without_link.rendered()),
            None => without_link.rendered(),
        }
    }

    pub(crate) fn rendered(&self) -> String {
        let mut cluster = String::new();
        let mut pre = String::new();
        let mut post: Vec<&str> = Vec::new();
        let mut signs = String::new();
        let mut reph = false;

        for token in &self.tokens {
            match token {
                Token::Consonant(c) => cluster.push_str(c),
                Token::Hasanta => cluster.push_str(HASANTA),
                Token::ExplicitHasanta => {
                    cluster.push_str(HASANTA);
                    cluster.push('\u{200C}');
                }
                Token::Phala(p) => {
                    // র + ্য needs a ZWJ so it renders as র‍্য (র‍্যাব), not as reph + য.
                    if *p == "\u{09CD}য" && cluster.ends_with('র') {
                        cluster.push('\u{200D}');
                    }
                    cluster.push_str(p);
                }
                Token::PreKar(k) => pre.push_str(k),
                Token::PostKar(k) => post.push(k),
                Token::Reph => reph = true,
                Token::Sign(s) => signs.push_str(s),
                Token::Vowel(v) => cluster.push_str(v),
                Token::WaitingKar(_) => {}
            }
        }

        // Split vowels: ে … া → ো, ে … ৗ → ৌ, ে … ো → ো; ৗ on its own after a consonant → ৌ (j X → কৌ)
        if pre == "\u{09C7}" && !post.is_empty() {
            let first = post[0];
            if first == "\u{09BE}" || first == "\u{09CB}" {
                pre = "\u{09CB}".into();
                post.remove(0);
            } else if first == "\u{09D7}" {
                pre = "\u{09CC}".into();
                post.remove(0);
            }
        } else if pre.is_empty() && !cluster.is_empty() && post.first() == Some(&"\u{09D7}") {
            post[0] = "\u{09CC}";
        }

        let mut out = String::new();
        if reph {
            out.push('র');
            out.push_str(HASANTA);
        }
        out.push_str(&cluster);
        out.push_str(&pre);
        for k in post {
            out.push_str(k);
        }
        out.push_str(&signs);
        out
    }
}

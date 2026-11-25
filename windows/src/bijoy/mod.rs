//! The Bijoy state machine: a port of `Sources/NuktaEngine/Engine.swift` (and of
//! `linux/nukta_bangla/engine.py`). Pure logic, no Windows APIs, so it is tested on any machine.
//!
//! Keep the three copies in step: a rule changed here is changed in Swift and Python too.

pub mod keymap;
mod syllable;

pub use keymap::BanglaKey;
use syllable::{independent_vowel, is_pre_base_kar, Syllable, Token, HASANTA};

/// Result of one keystroke.
///
/// `committed` is text that is final; `display` is the syllable still being built (it can still be
/// reordered by later keys). `visible` is what goes on screen for it: all of it, a trailing ্
/// included, except a lone g (no consonant yet), which waits for the next key; a ি ে ৈ whose
/// consonant isn't settled shows apart, on a no-break space that is never typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub committed: String,
    pub display: String,
    pub visible: String,
}

/// ি ে ৈ work as in Avro 4.5.1's Bijoy layout: pressed once they wait for the next consonant
/// (classic, `c j` → কে); pressed twice right after a consonant they attach to it (`j c c` → কে).
#[derive(Clone, Default)]
pub struct Engine {
    syllable: Syllable,
    committed: String,
    /// The letter just before the cursor in the app, when the caller could read it.
    before: Option<char>,
}

/// ক … হ, ড় ঢ় য়: a letter a ্ can join.
fn is_consonant(c: char) -> bool {
    matches!(c, '\u{0995}'..='\u{09B9}' | '\u{09DC}' | '\u{09DD}' | '\u{09DF}')
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_composing(&self) -> bool {
        !self.syllable.is_empty()
    }

    pub fn display(&self) -> String {
        self.syllable.rendered()
    }

    /// Tells the engine the letter just before the cursor in the app (None when unknown), so a ্
    /// typed after a finished letter shows (ম, then g → ম্) instead of waiting for the next key.
    /// Only matters while nothing is being composed: the engine knows what it has typed itself.
    pub fn set_text_before(&mut self, before: Option<char>) {
        self.before = before;
    }

    /// None when the character is not a Bijoy key (the caller should commit and pass it through).
    pub fn process_character(&mut self, character: char) -> Option<Output> {
        keymap::key(character).map(|key| self.process(key))
    }

    pub fn process(&mut self, key: BanglaKey) -> Output {
        self.committed.clear();

        // Kar pressed once after a consonant: pressed again, it belongs to that consonant
        // (j c c → কে); anything else, and it waits for the next consonant as usual (j c V → কলে).
        if let Some(Token::WaitingKar(k)) = self.syllable.tokens.last().cloned() {
            self.syllable.pop();
            if key == BanglaKey::Kar(k) {
                self.syllable.push(Token::PreKar(k)); // rendered after the cluster
                return self.output();
            }
            self.commit_syllable();
            self.syllable.push(Token::PreKar(k));
        }

        // A visible hasanta (g g) ends the syllable, but it stays in progress until the next key, so
        // a backspace right after it gives the letter back (ম্‌ → ম, and g then makes ম্ again)
        // instead of leaving it to the app, which may delete only the invisible ZWNJ.
        if self.syllable.tokens.last() == Some(&Token::ExplicitHasanta) {
            self.commit_syllable();
        }

        // অ waits one key: া makes it আ (F f), anything else leaves it as অ.
        if matches!(self.syllable.tokens.first(), Some(Token::Vowel(_))) {
            if key == BanglaKey::Kar("\u{09BE}") {
                self.syllable = Syllable::default();
                return Output { committed: "আ".into(), ..Output::default() };
            }
            self.commit_syllable();
        }

        match key {
            BanglaKey::Literal(text) => {
                self.commit_syllable();
                self.committed.push_str(text);
            }

            BanglaKey::Vowel(v) => {
                self.commit_syllable();
                self.syllable.push(Token::Vowel(v));
            }

            BanglaKey::Consonant(c) => {
                let s = &self.syllable;
                if s.has_cluster() && s.last_is_hasanta() {
                    self.syllable.push(Token::Consonant(c)); // juktakkhor: ক + ্ + ত
                } else if s.has_cluster() || s.is_cluster_closed() || s.last_is_hasanta() {
                    self.commit_syllable();
                    self.syllable.push(Token::Consonant(c));
                } else {
                    self.syllable.push(Token::Consonant(c)); // empty, or only a pending pre-base kar
                }
            }

            BanglaKey::Hasanta => {
                if self.syllable.last_is_hasanta() {
                    // g g → visible hasanta (্ + ZWNJ)
                    self.syllable.pop();
                    if self.syllable.has_cluster() {
                        self.syllable.push(Token::ExplicitHasanta); // finished by the next key
                    } else {
                        self.commit_syllable();
                        self.committed.push_str(HASANTA);
                        self.committed.push('\u{200C}');
                    }
                } else if self.syllable.has_cluster() && !self.syllable.is_cluster_closed() {
                    self.syllable.push(Token::Hasanta);
                } else {
                    self.commit_syllable();
                    self.syllable.push(Token::Hasanta); // may become a full vowel with the next kar
                }
            }

            BanglaKey::Kar(k) => {
                if self.syllable.last_is_hasanta() {
                    // g + kar → full vowel (কই = j g d)
                    self.syllable.pop();
                    self.commit_syllable();
                    self.committed.push_str(independent_vowel(k));
                } else if is_pre_base_kar(k) {
                    let s = &self.syllable;
                    if s.has_cluster() && !s.is_cluster_closed() && !s.has_pre_kar() {
                        // a second press attaches it here; see the top of process()
                        self.syllable.push(Token::WaitingKar(k));
                    } else {
                        self.commit_syllable();
                        self.syllable.push(Token::PreKar(k));
                    }
                } else if !self.syllable.is_empty() {
                    self.syllable.push(Token::PostKar(k));
                } else {
                    self.committed.push_str(k);
                }
            }

            BanglaKey::Reph => {
                if self.syllable.has_cluster() && !self.syllable.has_reph() {
                    self.syllable.push(Token::Reph);
                } else {
                    self.commit_syllable();
                    self.committed.push('র');
                    self.committed.push_str(HASANTA);
                }
            }

            BanglaKey::Phala(p) => {
                if self.syllable.accepts_phala() {
                    if self.syllable.last_is_hasanta() {
                        self.syllable.pop();
                    }
                    self.syllable.push(Token::Phala(p));
                } else {
                    self.commit_syllable();
                    self.committed.push_str(p);
                }
            }

            BanglaKey::Sign(s) => {
                let syl = &self.syllable;
                if syl.is_empty() || (!syl.has_cluster() && syl.last_is_hasanta()) {
                    self.commit_syllable();
                    self.committed.push_str(s);
                } else {
                    self.syllable.push(Token::Sign(s));
                }
            }
        }

        self.output()
    }

    /// Undoes the last keystroke of the current syllable. None when nothing is being composed
    /// (the caller should let the app handle backspace).
    pub fn backspace(&mut self) -> Option<Output> {
        if self.syllable.is_empty() {
            return None;
        }
        self.syllable.pop();
        self.committed.clear();
        Some(self.output())
    }

    /// Finalises the current syllable.
    pub fn commit(&mut self) -> Output {
        self.committed.clear();
        self.commit_syllable();
        Output { committed: std::mem::take(&mut self.committed), ..Output::default() }
    }

    pub fn reset(&mut self) {
        self.syllable = Syllable::default();
        self.committed.clear();
    }

    fn output(&mut self) -> Output {
        // A ্ with no consonant of its own waits for the next key (g f → আ, g d → ই), unless
        // it sits on a finished letter: what this key just finished, else what was in the app.
        let lone_link = self.syllable.tokens == [Token::Hasanta];
        let before = self.committed.chars().last().or(self.before);
        let visible = if lone_link && before.is_some_and(is_consonant) {
            HASANTA.to_owned()
        } else {
            self.syllable.visible_rendered()
        };
        Output { committed: std::mem::take(&mut self.committed), display: self.syllable.rendered(), visible }
    }

    fn commit_syllable(&mut self) {
        if let Some(Token::WaitingKar(k)) = self.syllable.tokens.last().cloned() {
            // Finished with a single press pending: same text classic would give (ক + ে = কে).
            self.syllable.pop();
            self.committed.push_str(&self.syllable.rendered());
            self.committed.push_str(k);
        } else {
            self.committed.push_str(&self.syllable.rendered());
        }
        self.syllable = Syllable::default();
    }
}

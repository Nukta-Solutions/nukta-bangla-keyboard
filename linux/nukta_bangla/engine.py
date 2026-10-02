"""Bijoy state machine. Pure logic, no IBus — a port of ``Sources/NuktaEngine/Engine.swift``.

ি ে ৈ work as in Avro 4.5.1's Bijoy layout: pressed once they wait for the next consonant
(classic, ``c j`` → কে); pressed twice right after a consonant they attach to it (``j c c`` → কে).
"""

from . import keymap
from .syllable import HASANTA, INDEPENDENT_VOWELS, PRE_BASE_KARS, ZWNJ, Syllable


class Output:
    """Result of one keystroke.

    ``committed`` is text that is final; ``display`` is the syllable still being built (it can
    still be reordered by later keys, so it belongs in the preedit). ``visible`` is the part of
    ``display`` that would be safe to put straight into a document; the IBus front end shows the
    whole preedit and does not need it.
    """

    __slots__ = ("committed", "display", "visible")

    def __init__(self, committed, display, visible=None):
        self.committed = committed
        self.display = display
        self.visible = display if visible is None else visible

    def __eq__(self, other):
        return (isinstance(other, Output)
                and (self.committed, self.display, self.visible)
                == (other.committed, other.display, other.visible))

    def __repr__(self):
        return f"Output(committed={self.committed!r}, display={self.display!r}, visible={self.visible!r})"


class Engine:
    def __init__(self):
        self._syllable = Syllable()
        self._committed = ""

    @property
    def is_composing(self):
        return not self._syllable.is_empty

    @property
    def display(self):
        return self._syllable.rendered

    def process_character(self, character):
        """Returns None when the character is not a Bijoy key (the caller should commit and pass
        the key through to the application)."""
        key = keymap.key(character)
        if key is None:
            return None
        return self.process(key)

    def process(self, key):
        self._committed = ""
        syllable = self._syllable

        # Kar pressed once after a consonant: pressed again, it belongs to that consonant
        # (j c c → কে); anything else, and it waits for the next consonant as usual (j c V → কলে).
        waiting = syllable.waiting_kar
        if waiting is not None:
            syllable.remove_last()
            if key == ("kar", waiting):
                syllable.append(("preKar", waiting))  # rendered after the cluster
                return Output(self._committed, syllable.rendered, syllable.visible_rendered)
            self._commit_syllable()
            syllable = self._syllable
            syllable.append(("preKar", waiting))

        # অ waits one key: া makes it আ (F f), anything else leaves it as অ.
        if syllable.tokens and syllable.tokens[0][0] == "vowel":
            if key == ("kar", "া"):
                self._syllable = Syllable()
                self._committed = "আ"
                return Output(self._committed, "")
            self._commit_syllable()
            syllable = self._syllable

        kind = key[0]

        if kind == "literal":
            self._commit_syllable()
            self._committed += key[1]

        elif kind == "vowel":
            self._commit_syllable()
            self._syllable.append(("vowel", key[1]))

        elif kind == "consonant":
            if syllable.has_cluster and syllable.last_is_hasanta:
                syllable.append(("consonant", key[1]))  # juktakkhor: ক + ্ + ত
            elif syllable.has_cluster or syllable.is_cluster_closed or syllable.last_is_hasanta:
                self._commit_syllable()
                self._syllable.append(("consonant", key[1]))
            else:
                syllable.append(("consonant", key[1]))  # empty, or only a pending pre-base kar

        elif kind == "hasanta":
            if syllable.last_is_hasanta:
                # g g → visible hasanta (্ + ZWNJ)
                syllable.remove_last()
                if syllable.has_cluster:
                    syllable.append(("explicitHasanta",))
                    self._commit_syllable()
                else:
                    self._commit_syllable()
                    self._committed += HASANTA + ZWNJ
            elif syllable.has_cluster and not syllable.is_cluster_closed:
                syllable.append(("hasanta",))
            else:
                self._commit_syllable()
                self._syllable.append(("hasanta",))  # may become a full vowel with the next kar

        elif kind == "kar":
            kar = key[1]
            if syllable.last_is_hasanta:
                # g + kar → full vowel (কই = j g d)
                syllable.remove_last()
                self._commit_syllable()
                self._committed += INDEPENDENT_VOWELS.get(kar, kar)
            elif kar in PRE_BASE_KARS:
                if syllable.has_cluster and not syllable.is_cluster_closed and not syllable.has_pre_kar:
                    # a second press attaches it here; see the top of process()
                    syllable.append(("waitingKar", kar))
                else:
                    self._commit_syllable()
                    self._syllable.append(("preKar", kar))
            elif not syllable.is_empty:
                syllable.append(("postKar", kar))
            else:
                self._committed += kar

        elif kind == "reph":
            if syllable.has_cluster and not syllable.has_reph:
                syllable.append(("reph",))
            else:
                self._commit_syllable()
                self._committed += "র" + HASANTA

        elif kind == "phala":
            if syllable.has_cluster and not syllable.is_cluster_closed:
                if syllable.last_is_hasanta:
                    syllable.remove_last()
                syllable.append(("phala", key[1]))
            else:
                self._commit_syllable()
                self._committed += key[1]

        elif kind == "sign":
            if syllable.is_empty or (not syllable.has_cluster and syllable.last_is_hasanta):
                self._commit_syllable()
                self._committed += key[1]
            else:
                syllable.append(("sign", key[1]))

        else:  # pragma: no cover - the key map produces no other kinds
            raise ValueError(f"unknown key {key!r}")

        return Output(self._committed, self._syllable.rendered, self._syllable.visible_rendered)

    def backspace(self):
        """Undoes the last keystroke of the current syllable. Returns None when nothing is being
        composed (the caller should let the application handle backspace)."""
        if self._syllable.is_empty:
            return None
        self._syllable.remove_last()
        return Output("", self._syllable.rendered, self._syllable.visible_rendered)

    def commit(self):
        """Finalises the current syllable."""
        self._committed = ""
        self._commit_syllable()
        return Output(self._committed, "")

    def reset(self):
        self._syllable = Syllable()
        self._committed = ""

    def _commit_syllable(self):
        waiting = self._syllable.waiting_kar
        if waiting is not None:
            # Finished with a single press pending: same text classic would give (ক + ে = কে).
            self._syllable.remove_last()
            self._committed += self._syllable.rendered + waiting
        else:
            self._committed += self._syllable.rendered
        self._syllable = Syllable()

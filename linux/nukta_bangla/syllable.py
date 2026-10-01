"""One in-progress syllable, stored in the order the keys were pressed (Bijoy/visual order).

``rendered`` turns it into correct Unicode (logical) order. A port of
``Sources/NuktaEngine/Syllable.swift``; tokens are ``(kind, value)`` tuples:
``consonant``, ``hasanta``, ``explicitHasanta``, ``phala``, ``preKar``, ``postKar``, ``reph``,
``sign``, ``vowel`` and ``waitingKar`` (ি ে ৈ pressed once after a consonant; not shown until the
next key decides).
"""

HASANTA = "্"
ZWNJ = "‌"
ZWJ = "‍"

PRE_BASE_KARS = frozenset(("ি", "ে", "ৈ"))  # ি ে ৈ

# `g` + kar → full vowel.
INDEPENDENT_VOWELS = {
    "া": "আ", "ি": "ই", "ী": "ঈ", "ু": "উ", "ূ": "ঊ",
    "ৃ": "ঋ", "ে": "এ", "ৈ": "ঐ", "ো": "ও", "ৗ": "ঔ",
}


class Syllable:
    __slots__ = ("tokens",)

    def __init__(self, tokens=None):
        self.tokens = list(tokens) if tokens else []

    # MARK: state

    @property
    def is_empty(self):
        return not self.tokens

    @property
    def has_cluster(self):
        return any(t[0] == "consonant" for t in self.tokens)

    @property
    def has_reph(self):
        return any(t[0] == "reph" for t in self.tokens)

    @property
    def has_pre_kar(self):
        return any(t[0] == "preKar" for t in self.tokens)

    @property
    def last_is_hasanta(self):
        return bool(self.tokens) and self.tokens[-1][0] == "hasanta"

    @property
    def waiting_kar(self):
        """The kar of a trailing single-press ি ে ৈ, or None."""
        if self.tokens and self.tokens[-1][0] == "waitingKar":
            return self.tokens[-1][1]
        return None

    @property
    def is_cluster_closed(self):
        """A kar or sign after the consonants closes the cluster: no more consonants can join.
        (A pre-base kar typed first, classic style, doesn't.)"""
        for index, token in enumerate(self.tokens):
            if token[0] in ("postKar", "sign", "explicitHasanta"):
                return True
            if token[0] == "preKar" and index > 0:
                return True
        return False

    def append(self, token):
        self.tokens.append(token)

    def remove_last(self):
        self.tokens.pop()

    # MARK: rendering

    @property
    def visible_rendered(self):
        """Nothing goes on screen until there is a consonant to hang it on, and a trailing ্ waits
        for the next key (it may turn into a full vowel: ``j g d`` → কই)."""
        if not self.has_cluster:
            return ""
        if not (self.last_is_hasanta or self.waiting_kar is not None):
            return self.rendered
        return Syllable(self.tokens[:-1]).rendered

    @property
    def rendered(self):
        cluster = ""
        pre = ""
        post = []
        signs = ""
        reph = False

        for kind, *rest in self.tokens:
            if kind == "consonant":
                cluster += rest[0]
            elif kind == "hasanta":
                cluster += HASANTA
            elif kind == "explicitHasanta":
                cluster += HASANTA + ZWNJ
            elif kind == "phala":
                # র + ্য needs a ZWJ so it renders as র‍্য (র‍্যাব), not as reph + য.
                if rest[0] == HASANTA + "য" and cluster.endswith("র"):
                    cluster += ZWJ
                cluster += rest[0]
            elif kind == "preKar":
                pre += rest[0]
            elif kind == "postKar":
                post.append(rest[0])
            elif kind == "reph":
                reph = True
            elif kind == "sign":
                signs += rest[0]
            elif kind == "vowel":
                cluster += rest[0]
            elif kind == "waitingKar":
                pass

        # Split vowels: ে … া → ো, ে … ৗ → ৌ, ে … ো → ো; ৗ on its own after a consonant → ৌ (j X → কৌ)
        if pre == "ে" and post:
            if post[0] in ("া", "ো"):
                pre = "ো"
                post.pop(0)
            elif post[0] == "ৗ":
                pre = "ৌ"
                post.pop(0)
        elif not pre and cluster and post and post[0] == "ৗ":
            post[0] = "ৌ"

        return (("র" + HASANTA) if reph else "") + cluster + pre + "".join(post) + signs

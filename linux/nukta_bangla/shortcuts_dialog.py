"""নিজের শর্টকাট: the dialog where the user's shortcuts are added, changed and removed, as a small
GTK 3 window (the design is the dialog in ``windows/design/settings-preview.html``). The layout
menu's "নিজের শর্টকাট…" runs it as a process of its own (``shortcuts.open_dialog``), so IBus never waits on a window::

    python3 -m nukta_bangla.shortcuts_dialog

The rules and the file live in riti-bridge (``shortcuts.py`` wraps them); this is only the window.
The code box asks for English letters (it is marked as a password box that shows its text), so the
নুকতা বাংলা engines leave its keys alone; the text box takes Bangla typed with নুকতা বাংলা itself.
"""

import sys

from . import shortcuts as store
from .settings import data_directory

try:
    import gi

    gi.require_version("Gtk", "3.0")
    from gi.repository import Gdk, GLib, Gtk, Pango
except (ImportError, ValueError):
    Gtk = None

WIDTH = 460

CSS = b"""
.nukta { background-color: #0D0807; color: rgba(255, 255, 255, 0.92); }
.nukta label { color: rgba(255, 255, 255, 0.92); }
.nukta .title { font-size: 19px; font-weight: 700; }
.nukta .dim { color: rgba(255, 255, 255, 0.56); font-size: 12.5px; }
.nukta .faint { color: rgba(255, 255, 255, 0.32); font-size: 12px; }
.nukta .warn { color: #F0A24E; font-size: 12.5px; }
.nukta .code { color: #E87A1E; font-family: monospace; font-size: 13px; }
.nukta entry { background-color: #241C1A; color: rgba(255, 255, 255, 0.92); border: 1px solid #2D2827;
               border-radius: 6px; padding: 6px 10px; box-shadow: none; }
.nukta entry:focus { border-color: #E87A1E; }
.nukta entry.code-box { font-family: monospace; }
.nukta .list { background-color: #161110; border: 1px solid #2D2827; border-radius: 8px; }
.nukta list, .nukta list row { background-color: transparent; }
.nukta list row { border-top: 1px solid #2D2827; padding: 2px 6px 2px 12px; }
.nukta list row:first-child { border-top: none; }
.nukta list row:hover { background-color: #241C1A; }
.nukta list row.editing { background-color: #2A1A13; }
.nukta button { background-image: none; box-shadow: none; text-shadow: none; border-radius: 6px;
                padding: 6px 14px; min-height: 20px; }
.nukta button.brand { background-image: linear-gradient(63deg, #D13A0E, #E87A1E); border: none;
                      padding: 6px 16px; }
.nukta button.brand label { color: #FFFFFF; font-weight: 600; }
.nukta button.remove { background-color: transparent; border: none; padding: 2px 8px; }
.nukta button.remove label { color: rgba(255, 255, 255, 0.56); }
.nukta button.remove:hover { background-color: #2B2220; }
"""

LEAD = ("কোডটা পুরো লিখলে লেখাটা তালিকার প্রথমে আসে, সাধারণ শব্দটা থাকে দ্বিতীয়তে। "
        "কোডের শুধু শুরুটা মিললে কিছু আসে না।")
HINT = ("কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া (লেখার সময় সংখ্যা চাপলে তালিকা থেকে শব্দ "
        "বাছাই হয়)। কোডের ঘরে নুকতা নিজে থেকেই ইংরেজি লেখে। কোনো সারিতে ক্লিক করলে সেটা বদলানো যায়।")


def _label(text, *classes, wrap=False):
    label = Gtk.Label(label=text, xalign=0)
    if wrap:
        label.set_line_wrap(True)
        label.set_max_width_chars(52)
    for name in classes:
        label.get_style_context().add_class(name)
    return label


class ShortcutsDialog(Gtk.Window if Gtk else object):
    def __init__(self):
        super().__init__(title="নুকতা বাংলা — নিজের শর্টকাট")
        self.shortcuts = store.Shortcuts(data_directory())
        #: The code whose text is being changed (picked in the list, or typed).
        self.editing = None
        #: A code the user was warned about: pressing the button again saves it.
        self.pending = None

        self.set_default_size(WIDTH, -1)
        self.set_resizable(False)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.get_style_context().add_class("nukta")
        self.connect("destroy", Gtk.main_quit)
        self.connect("key-press-event", self._key)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_border_width(24)
        self.add(box)
        heading = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        heading.pack_start(_label("নিজের শর্টকাট", "title"), False, False, 0)
        heading.pack_start(_label(LEAD, "dim", wrap=True), False, False, 0)
        box.pack_start(heading, False, False, 0)

        # The code, the text, and the button.
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.code = Gtk.Entry(max_length=store.MAX_CODE, width_chars=8, placeholder_text="কোড: asw")
        self.code.get_style_context().add_class("code-box")
        # English letters whatever the layout: the engines leave a password box's keys alone.
        self.code.set_input_purpose(Gtk.InputPurpose.PASSWORD)
        self.code.set_visibility(True)
        self.code.connect("changed", self._code_changed)
        self.code.connect("activate", self._save)
        self.text = Gtk.Entry(max_length=store.MAX_TEXT, placeholder_text="পুরো লেখা", hexpand=True)
        self.text.connect("changed", self._text_changed)
        self.text.connect("activate", self._save)
        self.button = Gtk.Button(label="যোগ করুন")
        self.button.get_style_context().add_class("brand")
        self.button.connect("clicked", self._save)
        row.pack_start(self.code, False, False, 0)
        row.pack_start(self.text, True, True, 0)
        row.pack_start(self.button, False, False, 0)
        box.pack_start(row, False, False, 0)

        self.warning = _label("", "warn", wrap=True)
        self.warning.set_no_show_all(True)
        box.pack_start(self.warning, False, False, 0)

        self.list = Gtk.ListBox(selection_mode=Gtk.SelectionMode.NONE)
        self.list.connect("row-activated", self._row_activated)
        scroller = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER)
        scroller.set_max_content_height(262)
        scroller.set_propagate_natural_height(True)
        scroller.get_style_context().add_class("list")
        scroller.add(self.list)
        box.pack_start(scroller, False, False, 0)

        box.pack_start(_label(HINT, "dim", wrap=True), False, False, 0)
        foot = _label("পরিবর্তন সাথে সাথে সব অ্যাপে চালু হয়", "faint")
        foot.set_xalign(0.5)
        box.pack_start(foot, False, False, 0)
        self._fill()
        self.code.grab_focus()

    # MARK: The list

    def _fill(self):
        for child in self.list.get_children():
            self.list.remove(child)
        entries = self.shortcuts.all()
        if not entries:
            empty = _label("এখনো কোনো শর্টকাট নেই।\nযেমন: asw → আসসালামু আলাইকুম", "dim")
            empty.set_xalign(0.5)
            empty.set_justify(Gtk.Justification.CENTER)
            empty.set_margin_top(16)
            empty.set_margin_bottom(16)
            row = Gtk.ListBoxRow(activatable=False, selectable=False)
            row.add(empty)
            self.list.add(row)
        for code, text in entries.items():
            line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            code_label = _label(code, "code")
            code_label.set_size_request(88, -1)
            text_label = _label(text)
            text_label.set_ellipsize(Pango.EllipsizeMode.END)
            remove = Gtk.Button(label="✕")
            remove.get_style_context().add_class("remove")
            remove.set_tooltip_text("মুছুন")
            remove.connect("clicked", self._remove, code)
            line.pack_start(code_label, False, False, 0)
            line.pack_start(text_label, True, True, 0)
            line.pack_end(remove, False, False, 0)
            row = Gtk.ListBoxRow()
            row.code = code
            row.add(line)
            if code == self.editing:
                row.get_style_context().add_class("editing")
            self.list.add(row)
        self.list.show_all()

    def _row_activated(self, _list, row):
        code = getattr(row, "code", None)
        if code is None:
            return
        self.code.set_text(code)        # sets `editing` through _code_changed
        self.text.set_text(self.shortcuts.all().get(code, ""))
        self.text.grab_focus()
        self.text.set_position(-1)

    def _remove(self, _button, code):
        self.shortcuts.remove(code)
        if code == self.editing:
            self._clear()
        self._fill()

    # MARK: Typing

    def _code_changed(self, entry):
        typed = entry.get_text()
        kept = "".join(c.lower() for c in typed if c.isascii() and c.isalpha())
        if kept != typed:
            # Comes back here through "changed".
            GLib.idle_add(self._set_code, kept)
            return
        self.editing = kept if kept in self.shortcuts.all() else None
        self._reset()
        self._fill()

    def _set_code(self, text):
        self.code.set_text(text)
        self.code.set_position(-1)
        return False

    def _text_changed(self, _entry):
        if self.pending is not None or self.warning.get_visible():
            self._reset()

    def _reset(self):
        self.pending = None
        self.warning.hide()
        self.button.set_label("বদলান" if self.editing else "যোগ করুন")

    def _warn(self, text, pending=None):
        self.pending = pending
        self.warning.set_text(text)
        self.warning.show()
        if pending:
            self.button.set_label("তবুও যোগ করুন")

    def _save(self, *_):
        code, text = self.code.get_text().strip().lower(), self.text.get_text().strip()
        if not 2 <= len(code) <= store.MAX_CODE or not (code.isascii() and code.isalpha()):
            return self._warn("কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া।")
        if not text:
            return self._warn("পুরো লেখাটা দিন।")
        if code not in self.shortcuts.all() and self.pending != code:
            word = store.dictionary_word(code)
            if word:
                return self._warn(f"{code} দিয়ে অভিধানের শব্দ «{word}»-ও লেখা হয়। শর্টকাট করলে {word} "
                                  "লিখতে গেলে আগে শর্টকাট আসবে। তবুও রাখতে আবার চাপুন।", pending=code)
        result = self.shortcuts.set(code, text)
        if result == store.TEXT_TOO_LONG:
            return self._warn("লেখাটা ৩০০ অক্ষরের বেশি হতে পারবে না।")
        if result != store.SAVED:
            return self._warn("সেভ করা গেল না। আবার চেষ্টা করুন।")
        self._clear()
        self._fill()

    def _clear(self):
        self.editing = None
        self.text.set_text("")
        self.code.set_text("")
        self._reset()
        self.code.grab_focus()

    def _key(self, _window, event):
        if event.keyval == Gdk.KEY_Escape:
            self.destroy()
            return True
        return False


def main(argv):
    if Gtk is None or Gdk.Screen.get_default() is None:
        print("নুকতা বাংলা: the shortcuts dialog needs GTK 3 (python3-gi).", file=sys.stderr)
        return 1
    provider = Gtk.CssProvider()
    provider.load_from_data(CSS)
    Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), provider,
                                             Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    ShortcutsDialog().show_all()
    Gtk.main()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

"""The update dialog (windows/design/update-preview.html), as a small GTK 3 window. ``updates.py``
runs it as a process of its own, so IBus never waits on a window::

    python3 -m nukta_bangla.update_dialog --update              what update.json last found
    python3 -m nukta_bangla.update_dialog --update-background   the same, without taking the focus
    python3 -m nukta_bangla.update_dialog --latest              "already up to date"
    python3 -m nukta_bangla.update_dialog --failed              "could not check"

The buttons wait a second before they answer and are clicked, never pressed from the keyboard, so a
keystroke meant for the application underneath can't press one; Esc is "পরে".

Without GTK 3's Python bindings it falls back to a desktop notification (``notify-send``).
"""

import os
import subprocess
import sys
import time

from . import updates

try:
    import gi

    gi.require_version("Gtk", "3.0")
    from gi.repository import Gdk, GdkPixbuf, Gio, GLib, Gtk
except (ImportError, ValueError):
    Gtk = None

GUARD_MS = 1000
WIDTH = 420

CSS = b"""
.nukta { background-color: #0D0807; color: rgba(255, 255, 255, 0.92); }
.nukta label { color: rgba(255, 255, 255, 0.92); }
.nukta .title { font-size: 19px; font-weight: 700; }
.nukta .dim { color: rgba(255, 255, 255, 0.56); }
.nukta .new { color: #E87A1E; font-weight: 600; }
.nukta .card { background-color: #161110; border: 1px solid #2D2827; border-radius: 10px; }
.nukta .heading { color: #E87A1E; font-size: 12px; font-weight: 600; }
.nukta button { background-image: none; box-shadow: none; text-shadow: none; border-radius: 6px;
                padding: 6px 14px; min-height: 20px; }
.nukta button.plain { background-color: #241C1A; border: 1px solid #2D2827; }
.nukta button.plain:hover { background-color: #2B2220; }
.nukta button.brand { background-image: linear-gradient(63deg, #D13A0E, #E87A1E); border: none;
                      padding: 6px 18px; }
.nukta button.brand label { color: #FFFFFF; font-weight: 600; }
.nukta button.link { background-color: transparent; border: none; padding: 6px 4px; }
.nukta button.link label { color: rgba(255, 255, 255, 0.56); }
.nukta button.link:hover label { color: rgba(255, 255, 255, 0.92); }
.nukta button:disabled { opacity: 0.45; }
"""


def _icon_path():
    """The নু logo: icons/ beside the package, installed or in a clone."""
    return os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                        "icons", "nukta-bangla.svg")


def _label(text, *classes, wrap=False):
    label = Gtk.Label(label=text, xalign=0)
    if wrap:
        label.set_line_wrap(True)
        label.set_max_width_chars(44)
    for name in classes:
        label.get_style_context().add_class(name)
    return label


def _button(text, kind, handler):
    button = Gtk.Button(label=text)
    button.get_style_context().add_class(kind)
    # Clicked only: Space or Enter meant for another application must not press it.
    button.set_can_focus(False)
    button.connect("clicked", handler)
    return button


def _open(uri):
    try:
        Gio.AppInfo.launch_default_for_uri(uri, None)
    except GLib.Error as error:
        print(f"নুকতা বাংলা: cannot open {uri} — {error.message}", file=sys.stderr)


def _notify(mode):
    """No GTK: a desktop notification says the same, more briefly."""
    if mode in ("--update", "--update-background"):
        release = updates.State().latest
        title = "নুকতা বাংলা: নতুন সংস্করণ এসেছে"
        body = f"সংস্করণ {updates.bangla_digits(release.version)} — {release.download or release.url}"
        state = updates.State()
        state.mark_shown(time.time())
        state.save()
    elif mode == "--latest":
        title, body = "নুকতা বাংলা", "আপনি সর্বশেষ সংস্করণ ব্যবহার করছেন।"
    else:
        title, body = "নুকতা বাংলা", "আপডেট খোঁজা গেল না। ইন্টারনেট সংযোগ দেখে আবার চেষ্টা করুন।"
    try:
        subprocess.run(["notify-send", "--app-name=নুকতা বাংলা", f"--icon={_icon_path()}", title, body],
                       check=False, timeout=10)
    except (OSError, subprocess.SubprocessError) as error:
        print(f"নুকতা বাংলা: {title}: {body} ({error})", file=sys.stderr)


class UpdateDialog(Gtk.Window if Gtk else object):
    def __init__(self, mode):
        super().__init__(title="নুকতা বাংলা আপডেট")
        self.mode = mode
        self.state = updates.State()
        self.release = self.state.latest
        self.guarded = []

        self.set_default_size(WIDTH, -1)
        self.set_resizable(False)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_keep_above(True)
        self.get_style_context().add_class("nukta")
        if mode == "--update-background":
            # In front, but the application being typed in keeps the keyboard.
            self.set_focus_on_map(False)
        try:
            self.set_icon_from_file(_icon_path())
        except GLib.Error:
            pass
        self.connect("destroy", Gtk.main_quit)
        self.connect("key-press-event", self._key)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        box.set_border_width(26)
        self.add(box)
        if mode in ("--update", "--update-background"):
            self._build_update(box)
        else:
            self._build_message(box)

        for button in self.guarded:
            button.set_sensitive(False)
        GLib.timeout_add(GUARD_MS, self._release_guard)

    # MARK: Building

    def _head(self, title, subtitle):
        head = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        try:
            logo = GdkPixbuf.Pixbuf.new_from_file_at_scale(_icon_path(), 62, 47, True)
            head.pack_start(Gtk.Image.new_from_pixbuf(logo), False, False, 0)
        except GLib.Error:
            pass
        titles = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2, valign=Gtk.Align.CENTER)
        titles.pack_start(_label(title, "title"), False, False, 0)
        titles.pack_start(subtitle, False, False, 0)
        head.pack_start(titles, True, True, 0)
        return head

    def _build_update(self, box):
        release = self.release
        versions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        versions.pack_start(_label(f"আপনার: {updates.bangla_digits(updates.current())}", "dim"),
                            False, False, 0)
        versions.pack_start(_label("→", "dim"), False, False, 0)
        versions.pack_start(_label(f"নতুন: {updates.bangla_digits(release.version)}", "new"),
                            False, False, 0)
        box.pack_start(self._head("নতুন সংস্করণ এসেছে", versions), False, False, 0)

        if release.notes:
            card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
            card.get_style_context().add_class("card")
            card.set_border_width(0)
            inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
            inner.set_margin_top(12)
            inner.set_margin_bottom(12)
            inner.set_margin_start(14)
            inner.set_margin_end(14)
            inner.pack_start(_label("নতুন যা আছে", "heading"), False, False, 2)
            for note in release.notes:
                inner.pack_start(_label(f"•  {note}", wrap=True), False, False, 0)
            more = _button("সব পরিবর্তন দেখুন", "link", lambda *_: _open(release.url))
            more.set_halign(Gtk.Align.START)
            inner.pack_start(more, False, False, 4)
            card.pack_start(inner, True, True, 0)
            box.pack_start(card, False, False, 0)

        # এই সংস্করণ বাদ দিন on the left; পরে and ডাউনলোড on the right. None is the default.
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        skip = _button("এই সংস্করণ বাদ দিন", "link", self._skip)
        later = _button("পরে", "plain", self._later)
        download = _button("ডাউনলোড", "brand", self._download)
        row.pack_start(skip, False, False, 0)
        row.pack_end(download, False, False, 0)
        row.pack_end(later, False, False, 0)
        box.pack_start(row, False, False, 0)
        self.guarded = [skip, later, download]

        # Not again for a day, whichever button is clicked.
        self.state.mark_shown(time.time())
        self.state.save()

    def _build_message(self, box):
        if self.mode == "--latest":
            title = "আপনি সর্বশেষ সংস্করণ ব্যবহার করছেন"
            detail = f"সংস্করণ {updates.bangla_digits(updates.current())}"
        else:
            title = "আপডেট খোঁজা গেল না"
            detail = "ইন্টারনেট সংযোগ দেখে আবার চেষ্টা করুন।"
        box.pack_start(self._head(title, _label(detail, "dim", wrap=True)), False, False, 0)
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        row.pack_end(_button("ঠিক আছে", "plain", lambda *_: self.destroy()), False, False, 0)
        box.pack_start(row, False, False, 0)

    # MARK: The buttons

    def _release_guard(self):
        for button in self.guarded:
            button.set_sensitive(True)
        self.guarded = []
        return False

    def _key(self, _window, event):
        if event.keyval == Gdk.KEY_Escape and not self.guarded:
            self._later()
            return True
        return False

    def _download(self, *_):
        # The release's installer straight to Downloads, through the browser; a release without one
        # opens its GitHub page instead.
        _open(self.release.download or self.release.url)
        # Asked again in a day, in case it wasn't installed.
        self._later()

    def _skip(self, *_):
        self.state = updates.State()
        self.state.skip(self.release.version)
        self._later()

    def _later(self, *_):
        if self.mode in ("--update", "--update-background"):
            self.state.mark_shown(time.time())
            self.state.save()
        self.destroy()


def main(argv):
    mode = argv[1] if len(argv) > 1 else "--update"
    if mode in ("--update", "--update-background"):
        latest = updates.State().latest
        if latest is None or not updates.is_newer(latest.version, updates.current()):
            return 0
    if Gtk is None or Gdk.Screen.get_default() is None:
        _notify(mode)
        return 0
    provider = Gtk.CssProvider()
    provider.load_from_data(CSS)
    Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), provider,
                                             Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    UpdateDialog(mode).show_all()
    Gtk.main()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

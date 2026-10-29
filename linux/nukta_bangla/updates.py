"""Update checks — the counterpart of ``windows/src/update.rs`` and
``Sources/NuktaInputMethod/Updates.swift``, with the same rules.

- The newest release *for Linux* comes from GitHub: of the recent published releases, the highest
  version that has ``nukta-bangla-linux.sh`` attached. A release without it (one for Windows or
  macOS only) is never offered here. Its tag is the version, and the description's bullet lines
  (``- …`` or ``* …``, at most five) are what the dialog shows under "নতুন যা আছে".
- Switching to a নুকতা বাংলা layout asks GitHub at most once a day, while ``autoUpdateCheck`` is on.
  The dialog then waits a day before asking again ("পরে"), and a skipped version is never offered
  again ("বাদ দিন").
- "এখনই আপডেট খুঁজুন" in the layout menu asks straight away and always answers.

What was found and what the user chose live beside the settings, in their own file::

    ~/.config/nukta-bangla/update.json

The dialog itself is ``update_dialog.py``, run as a process of its own: the engine lives inside
IBus and must never block on a window.
"""

import json
import os
import re
import subprocess
import sys
import threading
import time
import urllib.request
from collections import namedtuple

from .settings import Settings, config_directory

#: The recent releases, newest first.
API_URL = "https://api.github.com/repos/Nukta-Solutions/nukta-bangla-keyboard/releases?per_page=20"
#: The keyboard's page, which has every platform's installer.
WEBSITE = "https://www.nuktasolutions.com/nukta-bangla-keyboard"
#: The Linux installer's name in every release (CLAUDE.md: never renamed).
INSTALLER_ASSET = "nukta-bangla-linux.sh"
#: How long between automatic checks, and how long "পরে" waits.
DAY = 24 * 60 * 60
#: Bullet lines shown in the dialog; the rest are on the release page.
MAX_NOTES = 5
STATE_FILE = "update.json"

#: A published release: the version without the tag's "v" (1.0.3), its notes, its GitHub page, and
#: its Linux installer if it has one (what ডাউনলোড fetches).
Release = namedtuple("Release", "version notes url download", defaults=(None,))


def current():
    """This build's version."""
    from . import __version__
    return __version__


# MARK: Reading GitHub's answer

def version_parts(version):
    """1.0.3 → [1, 0, 3]. Anything after a "-" or "+" (1.1.0-beta) is ignored; so is a leading "v"."""
    core = re.split(r"[-+]", version.strip().lstrip("vV"), maxsplit=1)[0]
    parts = []
    for part in core.split("."):
        if not part.isdigit():
            return []
        parts.append(int(part))
    return parts


def is_newer(candidate, current_version):
    """True if `candidate` is a later version than `current_version` (1.0.10 is later than 1.0.9)."""
    a, b = version_parts(candidate), version_parts(current_version)
    if not a or not b:
        return False
    length = max(len(a), len(b))
    return a + [0] * (length - len(a)) > b + [0] * (length - len(b))


def plain(line):
    """A line of Markdown as plain text: **bold**, __bold__ and `code` lose their marks, and
    [a link](url) keeps only its text."""
    line = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", line)
    return line.replace("**", "").replace("__", "").replace("`", "").strip()


def notes_from(body):
    """The bullet lines of a release description, without their bullets or Markdown."""
    notes = []
    for line in body.splitlines():
        line = line.strip()
        if not (line.startswith("- ") or line.startswith("* ")):
            continue
        text = plain(line[2:])
        if text:
            notes.append(text)
        if len(notes) == MAX_NOTES:
            break
    return notes


def parse_release(data):
    """GitHub's answer for the latest release, or None if it isn't one."""
    try:
        value = json.loads(data)
    except (ValueError, TypeError):
        return None
    tag = value.get("tag_name") if isinstance(value, dict) else None
    if not isinstance(tag, str):
        return None
    version = tag.strip().lstrip("vV")
    if not version_parts(version):
        return None
    body = value.get("body") if isinstance(value.get("body"), str) else ""
    url = value.get("html_url") if isinstance(value.get("html_url"), str) else WEBSITE
    download = None
    for asset in value.get("assets") if isinstance(value.get("assets"), list) else []:
        if isinstance(asset, dict) and asset.get("name") == INSTALLER_ASSET:
            link = asset.get("browser_download_url")
            download = link if isinstance(link, str) else None
            break
    return Release(version, notes_from(body), url, download)


def newest_for_linux(data):
    """GitHub's list of releases: the newest published one with the Linux installer, or None.
    Drafts and pre-releases are passed over."""
    try:
        listed = json.loads(data)
    except (ValueError, TypeError):
        return None
    if not isinstance(listed, list):
        return None
    best = None
    for item in listed:
        if not isinstance(item, dict) or item.get("draft") or item.get("prerelease"):
            continue
        release = parse_release(json.dumps(item))
        if release is None or not release.download:
            continue
        if best is None or is_newer(release.version, best.version):
            best = release
    return best


def bangla_digits(text):
    """A version in Bangla digits (১.০.৩)."""
    return "".join(chr(0x09E6 + int(c)) if c in "0123456789" else c for c in text)


# MARK: What was found and what the user chose

class State:
    """update.json. A missing or broken file means nothing found yet."""

    def __init__(self, path=None):
        self.path = path or os.path.join(config_directory(), STATE_FILE)
        self.last_check = 0
        self.latest = None
        self.skipped = None
        self.remind_after = 0
        try:
            with open(self.path, "rb") as handle:
                value = json.load(handle)
        except (OSError, ValueError):
            value = {}
        if not isinstance(value, dict):
            value = {}
        self.last_check = _number(value.get("lastCheck"))
        self.remind_after = _number(value.get("remindAfter"))
        skipped = value.get("skippedVersion")
        self.skipped = skipped if isinstance(skipped, str) else None
        version = value.get("latestVersion")
        if isinstance(version, str):
            notes = value.get("latestNotes")
            notes = [n for n in notes if isinstance(n, str)] if isinstance(notes, list) else []
            url = value.get("latestUrl")
            download = value.get("latestDownload")
            self.latest = Release(version, notes, url if isinstance(url, str) else WEBSITE,
                                  download if isinstance(download, str) else None)

    def save(self):
        value = {"lastCheck": self.last_check, "remindAfter": self.remind_after}
        if self.latest is not None:
            value.update(latestVersion=self.latest.version, latestNotes=list(self.latest.notes),
                         latestUrl=self.latest.url)
            if self.latest.download:
                value["latestDownload"] = self.latest.download
        if self.skipped is not None:
            value["skippedVersion"] = self.skipped
        try:
            os.makedirs(os.path.dirname(self.path), exist_ok=True)
            temporary = f"{self.path}.{os.getpid()}.tmp"
            with open(temporary, "w", encoding="utf-8") as handle:
                json.dump(value, handle, ensure_ascii=False, indent=2)
            os.replace(temporary, self.path)
        except OSError:
            pass

    def check_due(self, now):
        """A day has passed since GitHub last answered."""
        return now - self.last_check >= DAY or self.last_check > now

    def record(self, release, now):
        """GitHub answered."""
        self.last_check = now
        self.latest = release

    def pending(self, current_version, now):
        """The release worth telling the user about, if any: newer than `current_version`, not
        skipped, and not put off."""
        release = self.latest
        if (release is not None and is_newer(release.version, current_version)
                and self.skipped != release.version and now >= self.remind_after):
            return release
        return None

    def mark_shown(self, now):
        """The dialog is on screen: not again for a day, whatever is clicked."""
        self.remind_after = now + DAY

    def skip(self, version):
        self.skipped = version


def _number(value):
    return int(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else 0


# MARK: Asking GitHub

def fetch_latest(timeout=15):
    """Asks GitHub for the latest release. Blocks: call it off the IBus thread."""
    request = urllib.request.Request(API_URL, headers={
        "Accept": "application/vnd.github+json",
        "User-Agent": f"NuktaBangla/{current()} (Linux)",
    })
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            if response.status != 200:
                return None
            return newest_for_linux(response.read(4 << 20))
    except (OSError, ValueError):
        return None


_checking = threading.Lock()


def check_if_due():
    """A layout got the focus: asks GitHub in the background if a day has passed since it last
    answered, or else shows what an earlier check found. Nothing at all while update checks are
    off, and never more than one request at a time."""
    if not Settings().auto_update_check:
        return
    if not State().check_due(time.time()):
        show_if_pending()
        return
    if not _checking.acquire(blocking=False):
        return

    def work():
        try:
            release = fetch_latest()
            if release is not None:
                state = State()
                state.record(release, time.time())
                state.save()
            show_if_pending()
        finally:
            _checking.release()

    threading.Thread(target=work, daemon=True).start()


def check_now():
    """"এখনই আপডেট খুঁজুন": asks GitHub now and always answers — with the update dialog, even for a
    version skipped before (the user asked), or a short message."""
    def work():
        release = fetch_latest()
        if release is None:
            open_dialog("--failed")
            return
        state = State()
        state.record(release, time.time())
        state.save()
        open_dialog("--update" if is_newer(release.version, current()) else "--latest")

    threading.Thread(target=work, daemon=True).start()


def show_if_pending():
    """Opens the update dialog if there is a new version the user hasn't skipped or put off. It
    opens without taking the keyboard, so typing in the application underneath goes on."""
    if not Settings().auto_update_check:
        return
    state = State()
    now = time.time()
    if state.pending(current(), now) is None:
        return
    # Not again for a day, whichever button is clicked.
    state.mark_shown(now)
    state.save()
    open_dialog("--update-background")


def open_dialog(mode):
    """Runs update_dialog.py on its own, so IBus never waits for a window."""
    package_parent = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    environment = dict(os.environ)
    environment["PYTHONPATH"] = os.pathsep.join(
        p for p in (package_parent, environment.get("PYTHONPATH")) if p)
    try:
        subprocess.Popen([sys.executable, "-m", "nukta_bangla.update_dialog", mode],
                         env=environment, cwd=package_parent, start_new_session=True,
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError as error:
        print(f"নুকতা বাংলা: cannot open the update dialog — {error}", file=sys.stderr)

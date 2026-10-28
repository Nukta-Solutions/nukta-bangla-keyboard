"""Update checks: the same rules as Windows (update.rs) and macOS (Updates.swift)."""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from nukta_bangla import updates  # noqa: E402
from nukta_bangla.settings import Settings  # noqa: E402
from nukta_bangla.updates import DAY, MAX_NOTES, State  # noqa: E402


def release_json(tag, body="", url=None):
    value = {"tag_name": tag, "body": body}
    if url:
        value["html_url"] = url
    return json.dumps(value).encode()


class VersionTests(unittest.TestCase):
    def test_compares_versions_by_number(self):
        self.assertTrue(updates.is_newer("1.0.3", "1.0.2"))
        self.assertTrue(updates.is_newer("v1.0.10", "1.0.9"))
        self.assertTrue(updates.is_newer("1.1", "1.0.9"))
        self.assertTrue(updates.is_newer("2.0.0", "1.9.9"))
        self.assertFalse(updates.is_newer("1.0.2", "1.0.2"))
        self.assertFalse(updates.is_newer("1.0", "1.0.0"))
        self.assertFalse(updates.is_newer("1.0.1", "1.0.2"))
        self.assertFalse(updates.is_newer("nonsense", "1.0.0"))
        self.assertEqual(updates.version_parts("v1.2.0-beta"), [1, 2, 0])
        self.assertEqual(updates.version_parts("1.x"), [])

    def test_writes_versions_in_bangla_digits(self):
        self.assertEqual(updates.bangla_digits("1.0.3"), "১.০.৩")


class ReleaseTests(unittest.TestCase):
    def test_reads_a_github_release(self):
        body = ("Intro paragraph.\r\n\r\n- ফোনেটিক সাজেশন আরও দ্রুত\r\n* বিজয়ে যুক্তাক্ষর ঠিক\r\n"
                "-not a bullet\r\n## Install\r\n- \r\n")
        url = "https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/tag/v1.0.3"
        release = updates.parse_release(release_json("v1.0.3", body, url))
        self.assertEqual(release.version, "1.0.3")
        self.assertEqual(release.notes, ["ফোনেটিক সাজেশন আরও দ্রুত", "বিজয়ে যুক্তাক্ষর ঠিক"])
        self.assertTrue(release.url.endswith("/v1.0.3"))
        self.assertIsNone(updates.parse_release(b'{"message": "Not Found"}'))
        self.assertIsNone(updates.parse_release(b'{"tag_name": "latest"}'))
        self.assertIsNone(updates.parse_release(b"not json"))

    def test_shows_at_most_five_notes(self):
        body = "".join(f"- note {n}\n" for n in range(1, 9))
        self.assertEqual(len(updates.notes_from(body)), MAX_NOTES)
        self.assertEqual(updates.notes_from("No bullets here."), [])

    def test_notes_lose_their_markdown(self):
        self.assertEqual(updates.plain("**Phonetic typing** (macOS): `ami` → আমি"),
                         "Phonetic typing (macOS): ami → আমি")
        self.assertEqual(updates.plain("See [the guide](https://example.com/guide) and [this]"),
                         "See the guide and [this]")
        self.assertEqual(updates.plain("__bold__ [a](b)[c](d)"), "bold ac")
        self.assertEqual(updates.plain("[x] y [a](b)"), "[x] y a")


class StateTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.directory.name, "update.json")

    def tearDown(self):
        self.directory.cleanup()

    def test_checks_once_a_day(self):
        state = State(self.path)
        self.assertTrue(state.check_due(1_000_000))
        state.record(updates.parse_release(release_json("v1.0.3")), 1_000_000)
        self.assertFalse(state.check_due(1_000_000 + DAY - 1))
        self.assertTrue(state.check_due(1_000_000 + DAY))
        # A clock set back doesn't stop checks for good.
        self.assertTrue(state.check_due(10))

    def test_offers_a_new_version_until_skipped_and_waits_a_day_after_showing(self):
        state = State(self.path)
        now = 5_000_000
        self.assertIsNone(state.pending("1.0.2", now))

        state.record(updates.parse_release(release_json("v1.0.3")), now)
        self.assertEqual(state.pending("1.0.2", now).version, "1.0.3")
        self.assertIsNone(state.pending("1.0.3", now), "already up to date")

        state.mark_shown(now)
        self.assertIsNone(state.pending("1.0.2", now + DAY - 1))
        self.assertIsNotNone(state.pending("1.0.2", now + DAY))

        state.skip("1.0.3")
        self.assertIsNone(state.pending("1.0.2", now + 2 * DAY))
        # The next release is offered again.
        state.record(updates.parse_release(release_json("v1.0.4")), now + 2 * DAY)
        self.assertIsNotNone(state.pending("1.0.2", now + 2 * DAY))

    def test_keeps_its_state_in_update_json(self):
        state = State(self.path)
        state.record(updates.parse_release(release_json("v1.0.3", "- one\n- দুই", "https://example.com/r")), 42)
        state.skip("1.0.1")
        state.mark_shown(100)
        state.save()

        again = State(self.path)
        self.assertEqual(again.last_check, 42)
        self.assertEqual(again.remind_after, 100 + DAY)
        self.assertEqual(again.skipped, "1.0.1")
        self.assertEqual(again.latest.version, "1.0.3")
        self.assertEqual(again.latest.notes, ["one", "দুই"])
        self.assertEqual(again.latest.url, "https://example.com/r")

        # A broken file is no state at all, not a crash.
        with open(self.path, "w") as handle:
            handle.write("{not json")
        self.assertIsNone(State(self.path).latest)


class SettingTests(unittest.TestCase):
    def test_the_switch_is_on_until_turned_off_and_keeps_the_other_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "settings.json")
            settings = Settings(path)
            self.assertTrue(settings.auto_update_check)
            with open(path, "w") as handle:
                json.dump({"typingMode": "smart", "someFutureKey": 7}, handle)
            settings.set_auto_update_check(False)
            self.assertFalse(settings.auto_update_check)
            self.assertFalse(Settings(path).auto_update_check)
            with open(path) as handle:
                saved = json.load(handle)
            self.assertEqual(saved["typingMode"], "smart")
            self.assertEqual(saved["someFutureKey"], 7)


if __name__ == "__main__":
    unittest.main()

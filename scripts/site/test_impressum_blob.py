#!/usr/bin/env python3
"""Tests for scripts/site/impressum_blob.py and the static Impressum files.

Only made-up data ("Erika Mustermann", "Jürgen Müller") is used here; the
owner's real data is in no file of this repository and these tests never read
it. Run from the repository root:

    python3 -m unittest discover -s scripts/site -p 'test_*.py'
    python3 scripts/site/test_impressum_blob.py

Standard library only.
"""

from __future__ import annotations

import base64
import contextlib
import fnmatch
import io
import json
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import impressum_blob as ib  # noqa: E402

REPO_ROOT = ib.REPO_ROOT
SITE = REPO_ROOT / "web" / "site"

FAKE = {
    "name": "Erika Mustermann",
    "street": "Musterweg 12",
    "postcode": "12345",
    "city": "Musterstadt",
    "country_de": "Deutschland",
    "country_en": "Germany",
    "email": "erika@example.org",
    "phone": "+49 30 1234567",
}
FAKE_UMLAUT = {
    "name": "Jürgen Müller",
    "street": "Hauptstraße 5",
    "postcode": "80331",
    "city": "Köln-Süd",
    "country_de": "Österreich",
    "country_en": "Austria",
    "email": "j.mueller@example.net",
    "phone": "+43 (0) 660 987 6543",
}
# A fixed nonce makes the "is it a substring of the blob" checks deterministic:
# a three-letter word has a small but real chance to appear in random base64.
NONCE = bytes(range(1, 17))

EMAIL_RE = re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")
DIGITS5_RE = re.compile(r"\d{5,}")
DIGITS6_RE = re.compile(r"\d{6,}")
ALPHABET_RE = re.compile(r"^[A-Za-z0-9_-]+$")


def _unb64(blob: bytes) -> bytes:
    text = blob.strip()
    return base64.urlsafe_b64decode(text + b"=" * (-len(text) % 4))


def _needles(fields: dict) -> set[str]:
    """Every string that must not be readable in a blob made from `fields`."""
    out: set[str] = set()
    for value in fields.values():
        out.add(value)
        out.update(w for w in value.split() if len(w) >= 3)
    phone = fields["phone"]
    out.add(re.sub(r"[^\d+]", "", phone))
    out.add(re.sub(r"\D", "", phone))
    local, _, domain = fields["email"].partition("@")
    out.update((local, domain))
    return {n for n in out if n}


def leaks(blob: bytes, fields: dict) -> list[str]:
    """The needles that can be read in `blob`: as text, as the raw bytes
    behind the base64, or as their hex. Empty when the blob is clean."""
    text = blob.decode("ascii")
    raw = _unb64(blob)
    hexed = raw.hex()
    found = []
    for n in sorted(_needles(fields)):
        if n in text or n.encode("utf-8") in raw or n.encode("utf-8").hex() in hexed:
            found.append(n)
    return found


def find_private_files(root: Path) -> list[Path]:
    """Files under `root` that must never be committed: a `.bin`, an
    `impressum-data*.json`, or anything in a directory named `private`."""
    found = []
    for p in sorted(root.rglob("*")):
        if not p.is_file():
            continue
        rel_dirs = p.relative_to(root).parts[:-1]
        if (
            p.name.endswith(".bin")
            or fnmatch.fnmatch(p.name, "impressum-data*.json")
            or "private" in rel_dirs
        ):
            found.append(p)
    return found


def _run_main(argv: list[str]) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = ib.main(argv)
    return code, out.getvalue(), err.getvalue()


class BlobTests(unittest.TestCase):
    def test_roundtrip(self):
        for fake in (FAKE, FAKE_UMLAUT):
            with self.subTest(name=fake["name"]):
                padded = {k: f"  {v}  " for k, v in fake.items()}
                padded["updated"] = "2026-10-13"
                padded["internal_note"] = "not for the page"
                got = ib.decode(ib.encode(padded))
                self.assertEqual(got, fake)
                self.assertEqual(set(got), set(ib.FIELDS))
                self.assertNotIn("updated", got)
                self.assertNotIn("internal_note", got)

    def test_nonce(self):
        self.assertEqual(ib.encode(FAKE, nonce=NONCE), ib.encode(FAKE, nonce=NONCE))
        self.assertNotEqual(ib.encode(FAKE), ib.encode(FAKE))
        self.assertNotEqual(ib.encode(FAKE, nonce=NONCE), ib.encode(FAKE, nonce=bytes(16)))
        with self.assertRaises(ValueError):
            ib.encode(FAKE, nonce=b"short")

    def test_alphabet(self):
        for fake in (FAKE, FAKE_UMLAUT):
            for nonce in (NONCE, None, None):
                blob = ib.encode(fake, nonce=nonce).decode("ascii")
                self.assertRegex(blob, ALPHABET_RE)
                for ch in "@.+/=":
                    self.assertNotIn(ch, blob)

    def test_no_plaintext(self):
        for fake in (FAKE, FAKE_UMLAUT):
            with self.subTest(name=fake["name"]):
                blob = ib.encode(fake, nonce=NONCE)
                self.assertEqual(leaks(blob, fake), [])
                text = blob.decode("ascii")
                self.assertIsNone(EMAIL_RE.search(text))
                self.assertIsNone(DIGITS5_RE.search(text))

    def test_plaintext_check_can_fail(self):
        # The same check, fed an unencrypted document as the "blob", must find
        # the data: otherwise test_no_plaintext proves nothing.
        for fake in (FAKE, FAKE_UMLAUT):
            with self.subTest(name=fake["name"]):
                doc = json.dumps({"v": 1, "fields": fake}, ensure_ascii=False).encode("utf-8")
                as_raw = base64.urlsafe_b64encode(doc).rstrip(b"=")
                found = leaks(as_raw, fake)
                for value in fake.values():
                    self.assertIn(value, found)
                self.assertIn(fake["email"].partition("@")[0], found)
        # a blob whose payload is plaintext after the header is caught as well
        header = ib.MAGIC + bytes([ib.VERSION]) + NONCE
        sloppy = base64.urlsafe_b64encode(header + FAKE["name"].encode()).rstrip(b"=")
        self.assertIn(FAKE["name"], leaks(sloppy, FAKE))

    def test_missing_or_empty_field(self):
        for key in ib.FIELDS:
            with self.subTest(missing=key):
                broken = {k: v for k, v in FAKE.items() if k != key}
                with self.assertRaises(ValueError):
                    ib.encode(broken)
            with self.subTest(empty=key):
                with self.assertRaises(ValueError):
                    ib.encode({**FAKE, key: "   "})
        with self.assertRaises(ValueError):
            ib.encode({**FAKE, "phone": 4930})

    def test_decode_rejects_other_blobs(self):
        with self.assertRaises(ValueError):
            ib.decode(base64.urlsafe_b64encode(b"NOPE" + bytes(40)).rstrip(b"="))


class MainTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)
        self.assertFalse(ib._inside_repo(self.tmp))

    def _write_json(self, data) -> Path:
        p = self.tmp / "in.json"
        p.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
        return p

    def test_main_writes_blob(self):
        src = self._write_json({**FAKE, "updated": "2026-10-13"})
        out = self.tmp / "nested" / "impressum.bin"
        code, stdout, stderr = _run_main([str(src), str(out)])
        self.assertEqual(code, 0, stderr)
        self.assertEqual(ib.decode(out.read_bytes()), FAKE)
        self.assertEqual(stat.S_IMODE(out.stat().st_mode), 0o644)
        self.assertTrue(out.read_bytes().endswith(b"\n"))
        self.assertFalse((out.parent / "impressum.bin.tmp").exists())
        for text in (stdout, stderr):
            for value in _needles(FAKE):
                self.assertNotIn(value, text)

    def test_main_umlauts(self):
        src = self._write_json(FAKE_UMLAUT)
        out = self.tmp / "impressum.bin"
        code, stdout, stderr = _run_main([str(src), str(out)])
        self.assertEqual(code, 0, stderr)
        self.assertEqual(ib.decode(out.read_bytes()), FAKE_UMLAUT)
        self.assertEqual(leaks(out.read_bytes().strip(), FAKE_UMLAUT), [])
        for value in _needles(FAKE_UMLAUT):
            self.assertNotIn(value, stdout + stderr)

    def test_main_incomplete_json(self):
        for key in ("email", "name"):
            with self.subTest(missing=key):
                src = self._write_json({k: v for k, v in FAKE.items() if k != key})
                out = self.tmp / "o.bin"
                code, stdout, stderr = _run_main([str(src), str(out)])
                self.assertEqual(code, 1)
                self.assertFalse(out.exists())
                for value in _needles(FAKE):
                    self.assertNotIn(value, stdout + stderr)
        with self.subTest("empty value"):
            src = self._write_json({**FAKE, "city": ""})
            self.assertEqual(_run_main([str(src), str(self.tmp / "o.bin")])[0], 1)
        with self.subTest("not json"):
            bad = self.tmp / "bad.json"
            bad.write_text("{not json", encoding="utf-8")
            self.assertEqual(_run_main([str(bad), str(self.tmp / "o.bin")])[0], 1)
        with self.subTest("not an object"):
            self.assertEqual(_run_main([str(self._write_json([1, 2])), str(self.tmp / "o.bin")])[0], 1)
        with self.subTest("no such file"):
            self.assertEqual(_run_main([str(self.tmp / "none.json"), str(self.tmp / "o.bin")])[0], 1)

    def test_main_refuses_output_inside_repo(self):
        src = self._write_json(FAKE)
        for rel in (("web", "site", "x.bin"), ("web", "private", "impressum.bin"), ("x.bin",)):
            with self.subTest(path="/".join(rel)):
                target = REPO_ROOT.joinpath(*rel)
                existed = target.exists()
                code, stdout, stderr = _run_main([str(src), str(target)])
                self.assertEqual(code, 2)
                self.assertEqual(target.exists(), existed)
                self.assertFalse(existed, "the test would have hidden a real file")
                for value in _needles(FAKE):
                    self.assertNotIn(value, stdout + stderr)

    def test_default_out_is_outside_repo(self):
        self.assertFalse(ib._inside_repo(ib.DEFAULT_OUT))
        with self.assertRaises(ValueError):
            ib.DEFAULT_OUT.resolve().relative_to(REPO_ROOT)


class GuardTests(unittest.TestCase):
    def test_guard_finds_planted_files(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "site" / "private").mkdir(parents=True)
            (root / "site" / "private" / "impressum.bin").write_bytes(b"x")
            (root / "site" / "private" / "notes.txt").write_text("x")
            (root / "impressum-data.json").write_text("{}")
            (root / "impressum-data-2.json").write_text("{}")
            (root / "site" / "ok.html").write_text("<p>")
            (root / "data.json").write_text("{}")
            found = {p.relative_to(root).as_posix() for p in find_private_files(root)}
            self.assertEqual(
                found,
                {
                    "site/private/impressum.bin",
                    "site/private/notes.txt",
                    "impressum-data.json",
                    "impressum-data-2.json",
                },
            )
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "a").mkdir()
            (root / "a" / "blob.bin").write_bytes(b"x")
            self.assertEqual(len(find_private_files(root)), 1)

    def test_guard_finds_nothing_under_web(self):
        self.assertEqual(find_private_files(REPO_ROOT / "web"), [])

    @unittest.skipUnless(shutil.which("git"), "git is not available")
    def test_gitignore_covers_private_paths(self):
        for path in ("web/site/private/impressum.bin", "web/impressum-data.json", "impressum-data.json"):
            with self.subTest(path=path):
                r = subprocess.run(
                    ["git", "check-ignore", "-q", path],
                    cwd=REPO_ROOT,
                    capture_output=True,
                )
                self.assertEqual(r.returncode, 0, f"{path} is not ignored")
        r = subprocess.run(
            ["git", "check-ignore", "-q", "web/site/impressum.html"],
            cwd=REPO_ROOT,
            capture_output=True,
        )
        self.assertEqual(r.returncode, 1, "the page itself must not be ignored")


class StaticFileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.js = (SITE / "impressum.js").read_text(encoding="utf-8")
        cls.html = (SITE / "impressum.html").read_text(encoding="utf-8")
        cls.index = (SITE / "index.html").read_text(encoding="utf-8")
        cls.robots = (SITE / "robots.txt").read_text(encoding="utf-8")

    def test_script_key_and_constants_agree_with_the_python(self):
        m = re.search(r'var SCRIPT_KEY = "([0-9a-fA-F]{32})";', self.js)
        self.assertIsNotNone(m, "no SCRIPT_KEY in impressum.js")
        self.assertEqual(m.group(1).lower(), ib.SCRIPT_KEY.hex())
        m = re.search(r'var MAGIC = "([^"]+)";', self.js)
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1).encode("ascii"), ib.MAGIC)
        m = re.search(r"var NONCE_LEN = (\d+);", self.js)
        self.assertIsNotNone(m)
        self.assertEqual(int(m.group(1)), ib.NONCE_LEN)
        m = re.search(r"var VERSION = (\d+);", self.js)
        self.assertIsNotNone(m)
        self.assertEqual(int(m.group(1)), ib.VERSION)

    def test_static_files_hold_no_data_shapes(self):
        for name, text in (("impressum.html", self.html), ("impressum.js", self.js)):
            with self.subTest(file=name):
                self.assertIsNone(EMAIL_RE.search(text), "an e-mail shaped string")
                self.assertIsNone(DIGITS6_RE.search(text), "a run of six or more digits")

    def test_data_shape_regexes_can_fail(self):
        self.assertIsNotNone(EMAIL_RE.search("write to erika@example.org today"))
        self.assertIsNotNone(DIGITS6_RE.search("call 0301234567"))
        self.assertIsNone(DIGITS6_RE.search("12345 Musterstadt"))

    def test_impressum_page(self):
        for needle in (
            "§ 5 DDG",
            "Bitte JavaScript aktivieren",
            "Kontaktformular",
            "noindex",
            "/site/impressum.js",
        ):
            with self.subTest(needle=needle):
                self.assertIn(needle, self.html)

    def test_index_links_to_the_impressum_twice(self):
        self.assertGreaterEqual(self.index.count('"/site/impressum.html"'), 2)

    def test_robots_keeps_crawlers_away(self):
        lines = {ln.strip() for ln in self.robots.splitlines()}
        self.assertIn("Disallow: /site/impressum.html", lines)
        self.assertIn("Disallow: /private/", lines)


if __name__ == "__main__":
    unittest.main()

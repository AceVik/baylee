#!/usr/bin/env python3
"""Tests for scripts/site/card_status.py: which cards may show a picture.

The rule (docs/legal.md section 3): only a card in the pool or on the
never-built list has a picture, and never a Universes Beyond printing, by its
security stamp or by its set. These tests cover `pictured`, `printing_facts`,
`fact` and `bulk_printings`, and check the committed set files obey the rule.
Only made-up ids and cards are used. Run from the repository root:

    python3 -m unittest discover -s scripts/site -p 'test_*.py'
    python3 scripts/site/test_card_status.py

Standard library only.
"""

from __future__ import annotations

import gzip
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import card_status as cs  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
SETS_DIR = REPO_ROOT / "web" / "site" / "data" / "sets"

# Made-up ids; none of them is a real printing or oracle id.
IMG = "11111111-1111-4111-8111-111111111111"
IMG2 = "22222222-2222-4222-8222-222222222222"
IMG3 = "33333333-3333-4333-8333-333333333333"
IMG4 = "44444444-4444-4444-8444-444444444444"
IMG5 = "55555555-5555-4555-8555-555555555555"
IMG6 = "66666666-6666-4666-8666-666666666666"
IMG7 = "77777777-7777-4777-8777-777777777777"
OID1 = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
OID2 = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
OID3 = "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
OID_OTHER = "dddddddd-dddd-4ddd-8ddd-dddddddddddd"

NOT_UB_SET = "zzq"  # made up, in no list
UB_SET = "ltr"


class PicturedTests(unittest.TestCase):
    def test_every_status_in_the_pictured_list_keeps_its_picture(self):
        """A plain printing in an ordinary set is shown for all four statuses."""
        self.assertEqual(cs.PICTURED, ("implemented", "partial", "unimplemented", "never"))
        facts = {IMG: (None, None, None, NOT_UB_SET)}
        for status in cs.PICTURED:
            with self.subTest(status=status):
                self.assertEqual(cs.pictured(status, NOT_UB_SET, IMG, facts), IMG)

    def test_an_absent_card_never_has_a_picture(self):
        """Status absent gives None whatever the printing and its facts say."""
        self.assertIsNone(cs.pictured("absent", NOT_UB_SET, IMG, {}))
        self.assertIsNone(cs.pictured("absent", NOT_UB_SET, IMG, {IMG: (None, None, None, NOT_UB_SET)}))

    def test_an_unknown_status_has_no_picture(self):
        """A status outside the pictured list gives None."""
        self.assertIsNone(cs.pictured("banned", NOT_UB_SET, IMG, {}))
        self.assertIsNone(cs.pictured("", NOT_UB_SET, IMG, {}))

    def test_no_printing_id_gives_no_picture(self):
        """An empty img (None or empty string) gives None for every status."""
        for status in cs.PICTURED:
            for img in (None, ""):
                with self.subTest(status=status, img=img):
                    self.assertIsNone(cs.pictured(status, NOT_UB_SET, img, {}))

    def test_a_triangle_stamp_drops_the_picture(self):
        """The security stamp triangle marks Universes Beyond even in an ordinary set."""
        facts = {IMG: ("triangle", None, None, NOT_UB_SET)}
        self.assertIsNone(cs.pictured("implemented", NOT_UB_SET, IMG, facts))

    def test_another_stamp_keeps_the_picture(self):
        """Stamps other than triangle (oval, acorn, none) do not drop it."""
        for stamp in ("oval", "acorn", "arena", "", None):
            with self.subTest(stamp=stamp):
                facts = {IMG: (stamp, None, None, NOT_UB_SET)}
                self.assertEqual(cs.pictured("implemented", NOT_UB_SET, IMG, facts), IMG)

    def test_a_printing_in_a_universes_beyond_set_drops_the_picture(self):
        """The printing's own set in UB_SETS gives None, though the listed set is ordinary."""
        facts = {IMG: (None, None, None, UB_SET)}
        self.assertIsNone(cs.pictured("implemented", NOT_UB_SET, IMG, facts))

    def test_every_universes_beyond_set_drops_a_listed_card(self):
        """With no facts, a card listed under any UB_SETS code has no picture."""
        self.assertTrue(cs.UB_SETS)
        for code in sorted(cs.UB_SETS):
            with self.subTest(code=code):
                self.assertIsNone(cs.pictured("implemented", code, IMG, {}))

    def test_a_card_listed_in_a_universes_beyond_set_drops_a_printing_from_an_ordinary_one(self):
        """The listed set in UB_SETS gives None even when the printing's own set is not UB."""
        facts = {IMG: (None, None, None, NOT_UB_SET)}
        self.assertIsNone(cs.pictured("implemented", UB_SET, IMG, facts))
        self.assertIsNone(cs.pictured("never", UB_SET, IMG, facts))

    def test_without_facts_the_listed_set_decides(self):
        """No facts entry: an ordinary listed set keeps the picture."""
        self.assertEqual(cs.pictured("partial", NOT_UB_SET, IMG, {}), IMG)
        self.assertEqual(cs.pictured("partial", NOT_UB_SET, IMG, {IMG2: ("triangle", None, None, UB_SET)}), IMG)

    def test_a_facts_entry_without_a_set_falls_back_to_the_listed_set(self):
        """A fact tuple whose set is None is judged by the listed set, as no entry is."""
        facts = {IMG: (None, None, None, None)}
        self.assertEqual(cs.pictured("implemented", NOT_UB_SET, IMG, facts), IMG)
        self.assertIsNone(cs.pictured("implemented", UB_SET, IMG, facts))

    def test_the_universes_beyond_list_is_lowercase_codes(self):
        """UB_SETS holds lowercase set codes only, as the ledger writes them."""
        for code in cs.UB_SETS:
            self.assertRegex(code, r"^[a-z0-9]+$")


class FactTests(unittest.TestCase):
    def test_the_query_is_stripped_from_the_scryfall_uri(self):
        """A scryfall.com card address loses its ?utm query."""
        c = {"scryfall_uri": "https://scryfall.com/card/zzq/12/made-up?utm_source=api", "set": "zzq"}
        self.assertEqual(cs.fact(c)[1], "https://scryfall.com/card/zzq/12/made-up")

    def test_an_address_elsewhere_is_dropped(self):
        """A uri not starting with https://scryfall.com/card/ becomes None."""
        for uri in ("https://example.com/card/zzq/1", "http://scryfall.com/card/zzq/1", "https://scryfall.com/sets/zzq", "", None):
            with self.subTest(uri=uri):
                self.assertIsNone(cs.fact({"scryfall_uri": uri})[1])

    def test_the_artist_comes_from_the_card(self):
        """A card-level artist wins over its faces'."""
        c = {"artist": "Made Up Artist", "card_faces": [{"artist": "Face Artist"}]}
        self.assertEqual(cs.fact(c)[2], "Made Up Artist")

    def test_the_artist_comes_from_the_first_face(self):
        """With no card-level artist the first face's artist is used."""
        c = {"card_faces": [{"artist": "First Face"}, {"artist": "Second Face"}]}
        self.assertEqual(cs.fact(c)[2], "First Face")

    def test_a_card_with_no_artist_gives_none(self):
        """No artist anywhere (also empty faces) gives None."""
        self.assertIsNone(cs.fact({})[2])
        self.assertIsNone(cs.fact({"card_faces": []})[2])
        self.assertIsNone(cs.fact({"card_faces": [{}]})[2])

    def test_the_tuple_is_stamp_uri_artist_set(self):
        """fact() returns (security_stamp, uri, artist, set) in that order."""
        c = {
            "security_stamp": "oval",
            "scryfall_uri": "https://scryfall.com/card/zzq/1/x?a=b",
            "artist": "A. Nonymous",
            "set": "zzq",
        }
        self.assertEqual(cs.fact(c), ("oval", "https://scryfall.com/card/zzq/1/x", "A. Nonymous", "zzq"))


class PrintingFactsTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def write(self, name, data):
        text = data if isinstance(data, str) else json.dumps(data)
        (self.dir / name).write_text(text, encoding="utf-8")

    def test_a_payload_becomes_a_fact_under_its_id(self):
        """One payload file gives {id: fact tuple}."""
        self.write("a.json", {"id": IMG, "set": "zzq", "security_stamp": "triangle", "artist": "X",
                              "scryfall_uri": "https://scryfall.com/card/zzq/1/n?utm=1"})
        self.assertEqual(
            cs.printing_facts(str(self.dir)),
            {IMG: ("triangle", "https://scryfall.com/card/zzq/1/n", "X", "zzq")},
        )

    def test_several_payloads_are_all_read(self):
        """Every .json file in the directory gets an entry."""
        self.write("a.json", {"id": IMG, "set": "zzq"})
        self.write("b.json", {"id": IMG2, "set": "zzr"})
        self.assertEqual(set(cs.printing_facts(str(self.dir))), {IMG, IMG2})

    def test_a_list_payload_is_skipped(self):
        """A file whose JSON is a list (a search result) is skipped."""
        self.write("list.json", [{"id": IMG, "set": "zzq"}])
        self.assertEqual(cs.printing_facts(str(self.dir)), {})

    def test_invalid_json_is_skipped(self):
        """A file that is not JSON is skipped, and the others still read."""
        self.write("bad.json", "{not json")
        self.write("good.json", {"id": IMG, "set": "zzq"})
        self.assertEqual(set(cs.printing_facts(str(self.dir))), {IMG})

    def test_an_id_that_is_no_uuid_is_skipped(self):
        """A missing, non-string-uuid or malformed id is skipped."""
        self.write("none.json", {"set": "zzq"})
        self.write("short.json", {"id": "abc", "set": "zzq"})
        self.write("upper.json", {"id": IMG.upper().replace("1", "A"), "set": "zzq"})
        self.write("number.json", {"id": 7, "set": "zzq"})
        self.write("null.json", {"id": None, "set": "zzq"})
        self.assertEqual(cs.printing_facts(str(self.dir)), {})

    def test_other_files_are_ignored(self):
        """Files without the .json ending are not read."""
        self.write("a.txt", {"id": IMG, "set": "zzq"})
        self.write("a.json.bak", {"id": IMG, "set": "zzq"})
        self.assertEqual(cs.printing_facts(str(self.dir)), {})

    def test_a_missing_directory_or_no_path_gives_nothing(self):
        """A path that is not a directory, None or empty gives {}."""
        self.assertEqual(cs.printing_facts(str(self.dir / "nowhere")), {})
        self.assertEqual(cs.printing_facts(None), {})
        self.assertEqual(cs.printing_facts(""), {})
        self.write("a.json", {"id": IMG})
        self.assertEqual(cs.printing_facts(str(self.dir / "a.json")), {})

    def test_an_empty_directory_gives_nothing(self):
        """No files, no facts."""
        self.assertEqual(cs.printing_facts(str(self.dir)), {})

    def test_the_facts_feed_the_rule(self):
        """A triangle payload read from disk makes pictured() drop the printing."""
        self.write("a.json", {"id": IMG, "set": "zzq", "security_stamp": "triangle"})
        facts = cs.printing_facts(str(self.dir))
        self.assertIsNone(cs.pictured("implemented", "zzq", IMG, facts))


def card(pid, oid, set_code, number, **extra):
    """A made-up English bulk-file card with a picture."""
    c = {
        "id": pid,
        "oracle_id": oid,
        "set": set_code,
        "collector_number": number,
        "lang": "en",
        "image_uris": {"normal": "https://example.invalid/x.jpg"},
        "artist": f"Artist {number}",
        "scryfall_uri": f"https://scryfall.com/card/{set_code}/{number}/made-up?utm_source=api",
    }
    c.update(extra)
    return c


class BulkPrintingsTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def jsonl(self, cards, name="bulk.jsonl", gz=False, wrap=False):
        if wrap:
            # Scryfall's plain JSON bulk: a list, one card per line, trailing commas.
            body = "[\n" + ",\n".join(json.dumps(c) for c in cards) + "\n]\n"
        else:
            body = "".join(json.dumps(c) + "\n" for c in cards)
        p = self.dir / (name + (".gz" if gz else ""))
        if gz:
            with gzip.open(p, "wt", encoding="utf-8") as f:
                f.write(body)
        else:
            p.write_text(body, encoding="utf-8")
        return str(p)

    def test_a_plain_printing_is_chosen_for_its_set_and_for_any(self):
        """A single printing is the pick under its set code and under '*'."""
        path = self.jsonl([card(IMG, OID1, "zzq", "7")])
        self.assertEqual(cs.bulk_printings(path, {OID1}), {OID1: {"zzq": IMG, "*": IMG}})

    def test_a_non_triangle_printing_beats_a_triangle_one_in_the_same_set(self):
        """Within one set the non-triangle printing wins, though the triangle one has the lower number."""
        path = self.jsonl([
            card(IMG, OID1, "zzq", "1", security_stamp="triangle"),
            card(IMG2, OID1, "zzq", "99"),
        ])
        got = cs.bulk_printings(path, {OID1})
        self.assertEqual(got[OID1]["zzq"], IMG2)
        self.assertEqual(got[OID1]["*"], IMG2)

    def test_the_order_in_the_file_does_not_matter(self):
        """The triangle printing coming second still loses."""
        path = self.jsonl([
            card(IMG2, OID1, "zzq", "99"),
            card(IMG, OID1, "zzq", "1", security_stamp="triangle"),
        ])
        self.assertEqual(cs.bulk_printings(path, {OID1})[OID1]["zzq"], IMG2)

    def test_a_card_that_is_ub_in_one_printing_and_not_in_another(self):
        """The ordinary printing is the pick for '*' and for its set; the UB set keeps its own printing."""
        path = self.jsonl([
            card(IMG, OID1, "ltr", "1", security_stamp="triangle"),
            card(IMG2, OID1, "zzq", "50"),
        ])
        got = cs.bulk_printings(path, {OID1})[OID1]
        self.assertEqual(got["*"], IMG2)
        self.assertEqual(got["zzq"], IMG2)
        self.assertEqual(got["ltr"], IMG)

    def test_a_card_with_only_a_triangle_printing_still_has_one(self):
        """The triangle printing is kept as the only one; pictured() drops it later."""
        path = self.jsonl([card(IMG, OID2, "zzr", "3", security_stamp="triangle")])
        facts = {}
        got = cs.bulk_printings(path, {OID2}, facts)
        self.assertEqual(got, {OID2: {"zzr": IMG, "*": IMG}})
        self.assertEqual(facts[IMG][0], "triangle")
        self.assertIsNone(cs.pictured("implemented", "zzr", got[OID2]["zzr"], facts))

    def test_a_non_english_printing_is_ignored(self):
        """A printing with lang other than en is never chosen, even if it would sort first."""
        path = self.jsonl([
            card(IMG, OID1, "zzq", "1", lang="de"),
            card(IMG2, OID1, "zzq", "40"),
        ])
        got = cs.bulk_printings(path, {OID1})
        self.assertEqual(got[OID1], {"zzq": IMG2, "*": IMG2})

    def test_a_card_with_only_a_non_english_printing_is_absent(self):
        """No English printing, no entry for the oracle id."""
        path = self.jsonl([card(IMG, OID1, "zzq", "1", lang="ja")])
        self.assertEqual(cs.bulk_printings(path, {OID1}), {})

    def test_a_card_without_lang_counts_as_english(self):
        """A missing lang key is read as en."""
        c = card(IMG, OID1, "zzq", "1")
        del c["lang"]
        self.assertEqual(cs.bulk_printings(self.jsonl([c]), {OID1})[OID1]["*"], IMG)

    def test_a_missing_or_placeholder_image_is_ignored(self):
        """image_status missing or placeholder never wins."""
        path = self.jsonl([
            card(IMG, OID1, "zzq", "1", image_status="missing"),
            card(IMG2, OID1, "zzq", "2", image_status="placeholder"),
            card(IMG3, OID1, "zzq", "30", image_status="highres_scan"),
        ])
        self.assertEqual(cs.bulk_printings(path, {OID1})[OID1], {"zzq": IMG3, "*": IMG3})

    def test_a_printing_with_no_picture_at_all_is_ignored(self):
        """No image_uris on the card nor on any face: skipped."""
        bare = card(IMG, OID1, "zzq", "1")
        del bare["image_uris"]
        path = self.jsonl([bare, card(IMG2, OID1, "zzq", "20")])
        self.assertEqual(cs.bulk_printings(path, {OID1})[OID1]["zzq"], IMG2)

    def test_a_double_faced_card_is_read_by_its_face(self):
        """oracle_id and image_uris on the faces, not the card, are enough."""
        dfc = card(IMG, None, "zzq", "4")
        del dfc["image_uris"]
        del dfc["oracle_id"]
        dfc["card_faces"] = [
            {"oracle_id": OID3, "image_uris": {"normal": "https://example.invalid/f.jpg"}, "artist": "Face Artist"},
            {"oracle_id": OID3, "image_uris": {"normal": "https://example.invalid/b.jpg"}},
        ]
        del dfc["artist"]
        facts = {}
        got = cs.bulk_printings(self.jsonl([dfc]), {OID3}, facts)
        self.assertEqual(got, {OID3: {"zzq": IMG, "*": IMG}})
        self.assertEqual(facts[IMG][2], "Face Artist")

    def test_an_unwanted_oracle_id_is_skipped_and_leaves_no_facts(self):
        """Only wanted oracle ids are chosen; others add nothing to the facts."""
        path = self.jsonl([card(IMG, OID_OTHER, "zzq", "1"), card(IMG2, OID1, "zzq", "2")])
        facts = {}
        got = cs.bulk_printings(path, {OID1}, facts)
        self.assertEqual(set(got), {OID1})
        self.assertEqual(set(facts), {IMG2})

    def test_facts_cover_exactly_the_chosen_printings(self):
        """A printing that lost every slot has no entry in facts."""
        path = self.jsonl([
            card(IMG, OID1, "zzq", "1", security_stamp="triangle"),
            card(IMG2, OID1, "zzq", "9"),
            card(IMG3, OID1, "zzq", "12"),
        ])
        facts = {}
        cs.bulk_printings(path, {OID1}, facts)
        self.assertEqual(set(facts), {IMG2})
        self.assertEqual(facts[IMG2], (None, "https://scryfall.com/card/zzq/9/made-up", "Artist 9", "zzq"))

    def test_facts_carry_the_stamp_and_set_for_the_rule(self):
        """The facts of a UB set's printing make pictured() drop it."""
        path = self.jsonl([card(IMG, OID1, "ltr", "1", security_stamp="triangle")])
        facts = {}
        got = cs.bulk_printings(path, {OID1}, facts)
        self.assertEqual(facts[IMG][3], "ltr")
        self.assertIsNone(cs.pictured("implemented", "ltr", got[OID1]["ltr"], facts))

    def test_facts_are_optional(self):
        """Called without facts it still answers (and does not need them)."""
        path = self.jsonl([card(IMG, OID1, "zzq", "1")])
        self.assertEqual(cs.bulk_printings(path, {OID1}, None)[OID1]["*"], IMG)

    def test_existing_facts_are_kept(self):
        """Entries the caller already holds (the per-card cache) stay."""
        facts = {IMG7: ("oval", None, "Kept", "zzq")}
        cs.bulk_printings(self.jsonl([card(IMG, OID1, "zzq", "1")]), {OID1}, facts)
        self.assertEqual(facts[IMG7], ("oval", None, "Kept", "zzq"))
        self.assertIn(IMG, facts)

    def test_the_lowest_collector_number_wins_by_number_not_text(self):
        """Collector number 9 sorts before 10 (numeric, not alphabetical)."""
        path = self.jsonl([card(IMG, OID1, "zzq", "10"), card(IMG2, OID1, "zzq", "9")])
        self.assertEqual(cs.bulk_printings(path, {OID1})[OID1]["zzq"], IMG2)

    def test_a_promo_loses_to_a_regular_printing(self):
        """A promo printing is chosen only when no regular one exists in the slot."""
        path = self.jsonl([card(IMG, OID1, "zzq", "1", promo=True), card(IMG2, OID1, "zzq", "80")])
        self.assertEqual(cs.bulk_printings(path, {OID1})[OID1]["zzq"], IMG2)

    def test_a_triangle_still_loses_to_a_better_printing_in_another_set(self):
        """For '*' the ordinary printing in a different set beats a triangle one."""
        path = self.jsonl([
            card(IMG, OID1, "zzr", "1", security_stamp="triangle"),
            card(IMG2, OID1, "zzq", "200", promo=True),
        ])
        got = cs.bulk_printings(path, {OID1})[OID1]
        self.assertEqual(got["*"], IMG2)
        self.assertEqual(got["zzr"], IMG)

    def test_each_oracle_id_is_chosen_on_its_own(self):
        """Two wanted cards do not disturb each other's pick."""
        path = self.jsonl([
            card(IMG, OID1, "zzq", "1"),
            card(IMG2, OID2, "zzq", "1", security_stamp="triangle"),
            card(IMG3, OID2, "zzq", "2"),
        ])
        got = cs.bulk_printings(path, {OID1, OID2})
        self.assertEqual(got[OID1]["zzq"], IMG)
        self.assertEqual(got[OID2]["zzq"], IMG3)

    def test_a_gzip_file_reads_like_a_plain_one(self):
        """A .jsonl.gz bulk file gives the same answer as its plain twin."""
        cards = [
            card(IMG, OID1, "zzq", "1", security_stamp="triangle"),
            card(IMG2, OID1, "zzq", "5"),
            card(IMG3, OID1, "zzq", "6", lang="fr"),
        ]
        plain = cs.bulk_printings(self.jsonl(cards), {OID1})
        zipped = cs.bulk_printings(self.jsonl(cards, name="bulk.jsonl", gz=True), {OID1})
        self.assertEqual(plain, zipped)
        self.assertEqual(zipped[OID1]["zzq"], IMG2)

    def test_a_json_list_with_trailing_commas_is_read_line_by_line(self):
        """The bracket lines are skipped and the commas after each card stripped."""
        path = self.jsonl([card(IMG, OID1, "zzq", "1"), card(IMG2, OID2, "zzr", "2")], name="bulk.json", wrap=True)
        got = cs.bulk_printings(path, {OID1, OID2})
        self.assertEqual(got[OID1]["*"], IMG)
        self.assertEqual(got[OID2]["*"], IMG2)

    def test_an_empty_wanted_set_chooses_nothing(self):
        """Nothing wanted, nothing read into facts."""
        facts = {}
        self.assertEqual(cs.bulk_printings(self.jsonl([card(IMG, OID1, "zzq", "1")]), set(), facts), {})
        self.assertEqual(facts, {})


class CommittedDataTests(unittest.TestCase):
    """The set files the landing page ships obey the picture rule."""

    @classmethod
    def setUpClass(cls):
        cls.files = sorted(SETS_DIR.glob("set-*.json"))
        cls.sets = {}
        for p in cls.files:
            cls.sets[p.name[len("set-"):-len(".json")]] = json.loads(p.read_text(encoding="utf-8"))

    def test_the_set_files_are_there(self):
        """Some set files are committed (a check below would otherwise pass on nothing)."""
        self.assertGreater(len(self.files), 100)

    def test_each_file_names_its_own_code(self):
        """set-<code>.json carries "code": "<code>"."""
        for code, doc in self.sets.items():
            with self.subTest(code=code):
                self.assertEqual(doc["code"], code)

    def test_no_absent_card_has_a_picture(self):
        """A card with status absent carries no img."""
        seen = 0
        for code, doc in self.sets.items():
            for c in doc["cards"]:
                if c["status"] == "absent":
                    seen += 1
                    self.assertNotIn("img", c, f"{code}: {c['name']}")
        self.assertGreater(seen, 0)

    def test_no_card_in_a_universes_beyond_set_has_a_picture(self):
        """No card of a set whose code is in UB_SETS carries img."""
        present = [code for code in cs.UB_SETS if code in self.sets]
        self.assertTrue(present)
        for code in present:
            for c in self.sets[code]["cards"]:
                self.assertNotIn("img", c, f"{code}: {c['name']}")

    def test_every_picture_is_a_uuid(self):
        """Each img is a lowercase Scryfall id."""
        seen = 0
        for code, doc in self.sets.items():
            for c in doc["cards"]:
                if "img" in c:
                    seen += 1
                    self.assertRegex(c["img"], cs.UUID, f"{code}: {c['name']}")
        self.assertGreater(seen, 0)

    def test_only_a_pictured_status_has_a_picture(self):
        """Every card with an img has a status in PICTURED."""
        for code, doc in self.sets.items():
            for c in doc["cards"]:
                if "img" in c:
                    self.assertIn(c["status"], cs.PICTURED, f"{code}: {c['name']}")

    def test_a_uri_and_an_artist_come_only_with_a_picture(self):
        """uri and artist appear only on a card that has img, and uri is a scryfall.com card page."""
        for code, doc in self.sets.items():
            for c in doc["cards"]:
                for key in ("uri", "artist"):
                    if key in c:
                        self.assertIn("img", c, f"{code}: {c['name']} has {key} without img")
                if "uri" in c:
                    self.assertTrue(c["uri"].startswith("https://scryfall.com/card/"), f"{code}: {c['name']}")
                    self.assertNotIn("?", c["uri"])


if __name__ == "__main__":
    unittest.main()

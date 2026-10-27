"""The live importer must fail closed and never manufacture policy supervision."""
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from import_live_records4 import import_records, validate


def record():
    lines = [
        {"kind": "header", "record": 1, "build": "test", "hash": "0"*16,
         "preset": {"seats": [{"controller": "Open"}, {"controller": {"Human": {"account": 0}}}]}},
        {"kind": "declared_mind", "n": 0, "at": 0, "seat": 1, "mind": {"kind": "human"}},
        {"kind": "input", "n": 0, "at": 0, "seat": 0, "by": "seat",
         "action": "PassPriority", "hash": "1"*16},
        {"kind": "input", "n": 1, "at": 0, "seat": 1, "by": "clock",
         "action": "PassPriority", "hash": "2"*16},
        {"kind": "input", "n": 2, "at": 0, "seat": 0, "by": "house",
         "action": {"SetPriorityHold": "None"}, "hash": "3"*16},
        {"kind": "input", "n": 3, "at": 0, "seat": 1, "by": "stand_in",
         "action": "Concede", "hash": "4"*16},
        {"kind": "end", "n": 4, "at": 0, "winners": [0], "reason": "Conceded", "turn": 2},
    ]
    for n, line in enumerate(lines[1:]):
        line["n"] = n
    return lines


def encode(lines):
    return ("\n".join(json.dumps(row) for row in lines) + "\n").encode()


class LiveImport(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.export = self.root / "export"
        self.export.mkdir()
        self.out = self.root / "out"

    def write(self, lines=None, number=1, mtime=10):
        raw = encode(record() if lines is None else lines)
        (self.export / f"{number:06d}.jsonl.gz").write_bytes(gzip.compress(raw, mtime=mtime))
        return hashlib.sha256(raw).hexdigest()

    def manifest(self):
        return json.loads((self.out / "manifest.json").read_text())

    def test_dedup_is_uncompressed_and_incremental_manifest_is_idempotent(self):
        digest = self.write()
        self.write(number=2, mtime=30)
        self.assertEqual(import_records(self.export, self.out)["records"], 1)
        before = (self.out / "manifest.json").read_bytes()
        import_records(self.export, self.out)
        self.assertEqual(before, (self.out / "manifest.json").read_bytes())
        manifest = self.manifest()
        item = manifest["records"][digest]
        self.assertEqual(item["origins_by_seat"], [{"seat": 1, "house": 1}, {"clock": 1, "stand_in": 1}])
        self.assertEqual(item["actions_by_seat"][0], {"decision": 1, "automation": 1})
        self.assertEqual(manifest["policy_targets"], "none_until_review")
        self.assertFalse(manifest["ppo_eligible"])
        self.assertEqual(item["split"], "train")
        self.assertIn("not_for_imitation", manifest["neural_policy"])
        self.assertIn("needs_review", manifest["human_policy"])
        self.assertFalse(manifest["training_done"])
        stored = self.out / item["path"]
        self.assertEqual(stored.stat().st_mode & 0o222, 0)
        self.assertEqual(gzip.decompress(stored.read_bytes()), encode(record()))
        self.assertNotIn("000001", json.dumps(manifest))

    def test_bad_new_file_preserves_existing_manifest_and_records(self):
        self.write()
        import_records(self.export, self.out)
        before = (self.out / "manifest.json").read_bytes()
        bad = record()[:-1]
        self.write(bad, number=2)
        with self.assertRaises(ValueError):
            import_records(self.export, self.out)
        self.assertEqual(before, (self.out / "manifest.json").read_bytes())
        self.assertEqual(len(list((self.out / "records").glob("*.gz"))), 1)

    def test_detectable_nonanonymous_and_nonduel_records_are_refused(self):
        variants = []
        for key, value in (("at", 1791000000), ("game_id", "private")):
            bad = record()
            bad[1][key] = value
            variants.append(bad)
        bad = record()
        bad[0]["preset"]["seats"][1]["controller"]["Human"]["account"] = 41
        variants.append(bad)
        bad = record()
        bad[0]["preset"]["seats"].append({"controller": "Open"})
        variants.append(bad)
        bad = record()
        bad[0]["build"] = "01a1728e-df52-7476-a918-efb7fc77fbe2"
        variants.append(bad)
        for lines in variants:
            with self.subTest(lines=variants.index(lines)), self.assertRaises(ValueError):
                validate(encode(lines), 0)

    def test_smoke_exclusion_is_sticky_and_anonymous(self):
        digest = self.write()
        exclusion = self.root / "exclude.txt"
        exclusion.write_text(f"{digest} deterministic_smoke\n")
        self.assertEqual(import_records(self.export, self.out, exclude_sha256=exclusion)["records"], 0)
        import_records(self.export, self.out)
        self.assertEqual(self.manifest()["excluded"], {digest: "deterministic_smoke"})
        self.assertFalse(list((self.out / "records").glob("*.gz")))

    def test_immutable_content_corruption_and_role_change_fail_closed(self):
        digest = self.write()
        import_records(self.export, self.out)
        before = (self.out / "manifest.json").read_bytes()
        with self.assertRaises(ValueError):
            import_records(self.export, self.out, neural_seat=1)
        stored = self.out / "records" / f"{digest}.jsonl.gz"
        stored.chmod(0o644)
        stored.write_bytes(gzip.compress(b"corrupt"))
        with self.assertRaises(ValueError):
            import_records(self.export, self.out)
        self.assertEqual(before, (self.out / "manifest.json").read_bytes())

    def test_house_excluded_unknown_and_mixed_quarantined(self):
        house = record()
        house[1]["mind"] = {"kind": "house", "level": "steady"}
        self.write(house, number=1)
        unknown = record()
        unknown[1]["mind"] = {"kind": "undeclared"}
        self.write(unknown, number=2)
        mixed = record()
        mixed.insert(-1, {"kind": "declared_mind", "at": 0, "seat": 1,
                          "mind": {"kind": "house", "level": "steady"}})
        for n, line in enumerate(mixed[1:]):
            line["n"] = n
        self.write(mixed, number=3)
        missing = record()
        del missing[1]
        for n, line in enumerate(missing[1:]):
            line["n"] = n
        self.write(missing, number=4)
        result = import_records(self.export, self.out)
        self.assertEqual(result["records"], 0)
        self.assertEqual(result["excluded_house"], 1)
        self.assertEqual(result["quarantined"], 3)
        self.assertFalse(list((self.out / "records").glob("*.gz")))


if __name__ == "__main__":
    unittest.main()

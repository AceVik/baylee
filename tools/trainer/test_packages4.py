"""A consumer must never see partial, corrupt or validation game packages."""
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from prepare_packages4 import publish
from test_recurrent4 import write_dataset
from trajectory_store4 import TrajectoryStore4


class PackagePublicationTests(unittest.TestCase):
    def fixture(self, directory):
        source = Path(directory) / "source"
        source.mkdir()
        write_dataset(source, [7])
        (source / "card_abilities.bin").write_bytes(b"")
        header = json.loads((source / "meta.json").read_text())
        entry = {"game_id": 0, "split": "train", "shard": "steps.jsonl.gz",
                 "steps": 7, "structure_sha256": header["structure_sha256"],
                 "replay_hashes_checked": True, "unencoded_decisions": 0}
        return source, Path(directory) / "package", header, entry

    def test_ready_means_complete_immutable_game_and_reusable_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output, header, entry = self.fixture(directory)
            publish(source, output, [entry], header)
            marker = json.loads((output / "ready.json").read_text())
            self.assertEqual(marker["frames"], 7)
            self.assertEqual(marker["metadata_sha256"],
                             hashlib.sha256((output / "meta.json").read_bytes()).hexdigest())
            self.assertEqual((source / "train" / entry["shard"]).stat().st_ino,
                             (output / "train" / entry["shard"]).stat().st_ino)
            with patch("trajectory_store4.build_part", side_effect=AssertionError("cache rebuilt")):
                store = TrajectoryStore4(output, "train", [entry["shard"]])
                self.assertEqual(store.read(6)["step_cursor"], 6)
                self.assertTrue(store.read(6)["is_terminal"])
                store.close()
            before = (output / "ready.json").read_bytes()
            with self.assertRaises(FileExistsError):
                publish(source, output, [entry], header)
            self.assertEqual((output / "ready.json").read_bytes(), before)

    def test_reject_validation_unverified_catalog_or_unencoded_games(self):
        for change in ({"split": "val"}, {"replay_hashes_checked": False},
                       {"structure_sha256": "different"}, {"unencoded_decisions": 1},
                       {"shard": "../steps.jsonl.gz"}, {"game_id": 123}, {"steps": 8}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                source, output, header, entry = self.fixture(directory)
                with self.assertRaises(ValueError):
                    publish(source, output, [{**entry, **change}], header)
                self.assertFalse((output / "ready.json").exists())

    def test_corrupt_gzip_and_incomplete_trajectory_never_publish(self):
        for failure in ("gzip", "cursor", "terminal"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                source, output, header, entry = self.fixture(directory)
                shard = source / "train" / entry["shard"]
                if failure == "gzip":
                    shard.write_bytes(shard.read_bytes()[:-8])
                else:
                    with gzip.open(shard, "rt") as stream:
                        rows = [json.loads(line) for line in stream]
                    if failure == "cursor":
                        rows[3]["step_cursor"] = 2
                    else:
                        rows[-1]["is_terminal"] = False
                    with gzip.open(shard, "wt") as stream:
                        stream.writelines(json.dumps(row) + "\n" for row in rows)
                with self.assertRaises((ValueError, EOFError)):
                    publish(source, output, [entry], header)
                self.assertFalse((output / "ready.json").exists())


if __name__ == "__main__":
    unittest.main()

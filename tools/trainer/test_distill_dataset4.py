"""Round 0's distillation data end to end: `convert4 --bundle-distill` makes a
dataset from an `eval4 --distill-labels` run, and the trainer's
`TrajectoryDataset4` under `--distill-bundle` reads every frame as convert4
wrote it: the block's subtype and fired groups, N != R, and R at weight 1
exactly where the JSON steps have a label.

Needs a recorded run and the binaries of one build:
    BAYLEE_DISTILL_RECORDS  an eval4 --record-out of a --distill-labels run
    BAYLEE_CONVERT4         that build's convert4
and that build's trajectory_cache4 on PATH. Skipped without them.
"""
from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from collections import Counter
from pathlib import Path

import dataset4
from trajectory_store4 import bst4_distill_block
from training4.distill import check_datasets

RECORDS = os.environ.get("BAYLEE_DISTILL_RECORDS")
CONVERT4 = os.environ.get("BAYLEE_CONVERT4")


def json_steps(root: Path) -> dict:
    """The labelled frames of a dataset's JSON steps, by (game, seat, cursor)."""
    labels = {}
    for path in sorted((root / "bundle-distill").glob("game-*.jsonl")):
        lines = [json.loads(line) for line in path.read_text().splitlines()]
        assert lines[0]["kind"] == "header" and lines[-1]["kind"] == "completed", path
        assert lines[-1]["labels"] == len(lines) - 2, path
        for line in lines[1:-1]:
            labels[(line["game_id"], line["seat"], line["step_cursor"])] = line
    return labels


@unittest.skipUnless(RECORDS and CONVERT4, "set BAYLEE_DISTILL_RECORDS and BAYLEE_CONVERT4")
class DistillDataset4(unittest.TestCase):
    def test_the_trainer_reads_what_convert4_wrote(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "dataset"
            subprocess.run([CONVERT4, "--encoder-version", "10", "--bundle-distill",
                            "--include-passive-obs", "--runs", RECORDS, "--out", str(out),
                            "--threads", "2", "--val-every", "2"],
                           check=True, capture_output=True)
            meta = json.loads((out / "meta.json").read_text())
            vocab = meta["bundle_distillation"]
            self.assertEqual(meta["bundle_distillation_status"], "complete")
            self.assertEqual(vocab["subtypes"], ["cast", "pass", "instant_their_turn", "spend",
                                                 "pay", "mana", "activate", "land", "other"])
            labels = json_steps(out)
            seen, kinds, subtypes = Counter(), Counter(), Counter()
            for split in ("train", "val"):
                data = dataset4.TrajectoryDataset4(out, split=split, encoder_version=10)
                check_datasets((data,), True)
                self.assertEqual(data.distill_groups, vocab["groups"])
                index = data.store.index
                for row in range(len(index)):
                    key = (int(index["game"][row]), int(index["seat"][row]), int(index["cursor"][row]))
                    raw = data.store.read_raw(row)
                    block = bst4_distill_block(raw)
                    frame = data.encode_single_step(row)
                    label = labels.get(key)
                    if label is None:
                        # A fallback of a recorded seat (a block that says
                        # nothing), another seat's frame or a passive one.
                        if block is not None:
                            self.assertEqual(block, (255, 0), key)
                            self.assertEqual(float(frame["loss_weight"]), 0.0, key)
                            seen["fallback"] += 1
                        self.assertEqual(int(frame["distill_subtype"]), -1, key)
                        self.assertEqual(int(frame["fired_mask"]), 0, key)
                        self.assertFalse(bool(frame["n_ne_r"]), key)
                        continue
                    subtype = label.get("distill_subtype")
                    code = vocab["subtypes"].index(subtype) if subtype else -1
                    mask = sum(1 << vocab["groups"].index(group) for group in label["fired"])
                    self.assertEqual(block, (255 if code < 0 else code, mask), key)
                    self.assertEqual(int(frame["distill_subtype"]), code, key)
                    self.assertEqual(int(frame["fired_mask"]), mask, key)
                    self.assertEqual(bool(frame["n_ne_r"]), label["n_ne_r"], key)
                    self.assertTrue(bool(frame["is_decision"]), key)
                    self.assertEqual(float(frame["loss_weight"]), 1.0, key)
                    seen["labelled"] += 1
                    seen["n_ne_r"] += label["n_ne_r"]
                    kinds[label["label_kind"]] += 1
                    subtypes[subtype] += subtype is not None
            counts = vocab["counts"]
            self.assertEqual(seen["labelled"], len(labels))
            self.assertEqual(seen["labelled"], counts["labelled"])
            self.assertEqual(seen["n_ne_r"], counts["n_ne_r"])
            self.assertEqual(dict(kinds), counts["by_kind"])
            self.assertEqual({k: v for k, v in subtypes.items() if v}, counts["by_subtype"])
            self.assertGreater(kinds["priority"], 0)
            self.assertGreater(subtypes["pass"], 0)


if __name__ == "__main__":
    unittest.main()

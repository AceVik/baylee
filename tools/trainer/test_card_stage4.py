"""The card stage moves only what its mode allows and keeps probes out of net.pt."""
import argparse
import copy
import json
from pathlib import Path
import tempfile
import unittest

import torch

from card_stage4 import anchored_roots, run
from export_onnx4 import load_export_checkpoint
from model4.structure import structure_digest
from refresh_structure4 import refresh_checkpoint
import test_refresh_structure4 as refresh_tests


def forest_with_roots():
    from test_structure4 import small_forest
    forest = small_forest()
    forest["roots"] = {"c/1/0": 3, "c/2/0": 4, "c/10/0": 5, "c/11/0": 6, "a/1/0/0": 2}
    return forest


def face(root, card, mana_value, colors, power):
    return {"root": root, "card": card, "face": 0, "fully_read": True, "castable_from_hand": True,
            "mana_value": mana_value, "cost": {k: 0 for k in (
                "white", "blue", "black", "red", "green", "colorless", "generic", "variable",
                "hybrid", "phyrexian", "snow")} | {"generic": mana_value},
            "colors": colors, "types": 2, "supertypes": 0, "subtypes": [0] if power else [],
            "power": power, "toughness": power, "loyalty": None, "defense": None,
            "keywords": [0] if power else [], "ability_kinds": [0], "families": [card % 2],
            "effects": [card % 3]}


def labels_for(forest, extra=()):
    return {"version": 1, "catalog_sha256": structure_digest(forest), "families": ["damage", "draw"],
            "ability_kinds": ["Spell"], "effects": ["A", "B", "C"], "subtypes": [7],
            "faces": [face("c/1/0", 1, 1, 1, 2), face("c/2/0", 2, 2, 2, None),
                      face("c/10/0", 10, 3, 4, 3), face("c/11/0", 11, 4, 8, None), *extra]}


class CardStage(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(5)

    def prepare(self, root):
        """A reference model, and its refresh onto a catalog with one new operator."""
        _, source = refresh_tests.RefreshCheckpoint().source(root / "reference", 2)
        # Roots name nodes; the encoder's buffers and weights do not change.
        forest = forest_with_roots()
        source["structure"] = forest
        torch.save(source, root / "reference/net.pt")
        refreshed = copy.deepcopy(forest)
        refreshed["tags"].append("Creature.NewField")
        refreshed["nodes"].append({"tag": len(refreshed["tags"]) - 1, "bytes": [], "children": [1]})
        refreshed["roots"]["c/21/0"] = len(refreshed["nodes"]) - 1
        catalog = root / "catalog.json"
        catalog.write_text(json.dumps(refreshed))
        refresh_checkpoint(root / "reference", catalog, root / "refreshed", allow_new_operators=True)
        labels = root / "labels.json"
        labels.write_text(json.dumps(labels_for(refreshed, [face("c/21/0", 21, 5, 16, 4)])))
        return labels

    def args(self, root, labels, train, out, **extra):
        values = dict(model=root / "refreshed", reference=root / "reference", labels=labels, out=root / out,
                      train=train, steps=20, probe_steps=20, lr=1e-2, anchor_weight=100.0, holdout_every=10,
                      measure_every=10, seed=3, device="cpu")
        values.update(extra)
        return argparse.Namespace(**values)

    def test_new_tags_move_only_cards_that_use_them(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            labels = self.prepare(root)
            lineage = run(self.args(root, labels, "new-tags", "stage"))
            self.assertEqual(lineage["new_tags"], ["Creature.NewField"])
            self.assertEqual(lineage["roots_moved"], 1)
            before = load_export_checkpoint(root / "refreshed/net.pt")[1]
            after = load_export_checkpoint(root / "stage/net.pt")[1]
            with torch.no_grad():
                old, new = before.rules.encode_catalog(), after.rules.encode_catalog()
            roots = after.rules.data["roots"]
            for key, node in roots.items():
                same = torch.equal(old[node], new[node])
                self.assertEqual(same, key != "c/21/0", key)
            for name, tensor in torch.load(root / "refreshed/net.pt", weights_only=False)["state_dict"].items():
                saved = torch.load(root / "stage/net.pt", weights_only=False)["state_dict"][name]
                if name != "rules.symbol.weight":
                    torch.testing.assert_close(saved, tensor, rtol=0, atol=0)
            self.assertNotIn("probes", torch.load(root / "stage/net.pt", weights_only=False))
            self.assertTrue((root / "stage/probes.pt").exists())

    def test_the_anchor_holds_unchanged_roots_and_labels_must_match_the_catalog(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            labels = self.prepare(root)
            held = run(self.args(root, labels, "encoder", "held"))
            free = run(self.args(root, labels, "encoder", "free", anchor_weight=0.0))
            # Every shared root came through the refresh unchanged.
            self.assertEqual(held["anchored_roots"], held["shared_roots"])
            self.assertLess(held["metrics_after"]["anchor_rms"], free["metrics_after"]["anchor_rms"])
            self.assertIn("mana_value_accuracy", held["metrics_before"]["validation"])
            wrong = json.loads(labels.read_text())
            wrong["catalog_sha256"] = "0" * 64
            labels.write_text(json.dumps(wrong))
            with self.assertRaisesRegex(ValueError, "another catalog"):
                run(self.args(root, labels, "encoder", "refused"))
            self.assertFalse((root / "refused").exists())

    def test_anchored_roots_stay_on_the_tables_device(self):
        # The first GPU run failed here: root ids built on the CPU indexed a
        # CUDA table. Runs on whatever device the machine has.
        device = "cuda" if torch.cuda.is_available() else "cpu"
        new = torch.arange(12.0, device=device).reshape(4, 3)
        old = new.clone()
        old[2] += 1
        ids, values, shared = anchored_roots(new, {"a": 1, "b": 2, "c": 3}, old, {"a": 1, "b": 2, "d": 3})
        self.assertEqual(shared, 2)
        self.assertEqual(ids.tolist(), [1])
        self.assertEqual((ids.device.type, values.device.type), (new.device.type, new.device.type))


if __name__ == "__main__":
    unittest.main()

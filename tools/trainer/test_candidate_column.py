"""`--train-candidate-column`: one column learns, nothing else moves, and a
candidate with the slot at zero scores as the initial network does."""
import io
import unittest

import torch
import model5 as V5
from export_onnx4 import make_dummy_inputs
from training4.candidate_column import carries_column, freeze_all_but_column, weight_of
from training4.optimizer_groups import build_optimizer_groups

SLOT = 12


class CandidateColumnTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(1912)
        self.net = V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64,
                                            structure_width=16, relational_width=16,
                                            relational_revision=2)).eval()
        self.inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=6, P=2))
        self.index = next(i for i, t in enumerate(self.inputs)
                          if t.dim() == 3 and t.shape[-1] == V5.CAND_FEAT_WIDTH)

    def train(self, steps=3):
        freeze_all_but_column(self.net, SLOT, start_from_zero=True)
        groups, _ = build_optimizer_groups(self.net, 1e-2, architecture="v5", decay=False)
        optimizer = torch.optim.AdamW(groups)
        for _ in range(steps):
            optimizer.zero_grad(set_to_none=True)
            logits = self.net(*self.inputs)[1]
            logits[..., 0].sum().backward()
            optimizer.step()

    def with_slot(self, value):
        self.inputs[self.index][..., SLOT] = value

    def test_only_the_column_moves(self):
        self.with_slot(1.0)
        before = {name: value.clone() for name, value in self.net.state_dict().items()}
        before[f"cand_proj.0.weight"][:, SLOT] = 0
        self.train()
        after = self.net.state_dict()
        for name, value in before.items():
            if name == "cand_proj.0.weight":
                moved = value[:, SLOT] != after[name][:, SLOT]
                self.assertTrue(moved.any(), "the column learnt")
                other = list(range(value.shape[1]))
                other.remove(SLOT)
                self.assertTrue(torch.equal(value[:, other], after[name][:, other]))
            else:
                self.assertTrue(torch.equal(value, after[name]), name)

    def test_a_zero_slot_gets_no_gradient(self):
        self.with_slot(0.0)
        self.train(1)
        self.assertTrue(torch.equal(weight_of(self.net)[:, SLOT],
                                    torch.zeros_like(weight_of(self.net)[:, SLOT])))

    def test_a_zero_slot_scores_as_the_initial_network(self):
        self.with_slot(0.0)
        with torch.no_grad():
            reference = self.net(*self.inputs)[1].clone()
        self.with_slot(1.0)
        self.train()
        self.with_slot(0.0)
        with torch.no_grad():
            self.assertTrue(torch.equal(reference, self.net(*self.inputs)[1]))

    def test_a_trained_column_round_trips_through_a_checkpoint(self):
        self.with_slot(1.0)
        self.train()
        buffer = io.BytesIO()
        torch.save(self.net.state_dict(), buffer)
        buffer.seek(0)
        restored = torch.load(buffer, weights_only=True)
        for name, value in self.net.state_dict().items():
            self.assertTrue(torch.equal(value, restored[name]), name)

    def test_a_batch_without_the_slot_has_nothing_to_give(self):
        feat = torch.zeros(2, 3, 4, V5.CAND_FEAT_WIDTH)
        self.assertFalse(carries_column({"cand_feat": feat}, SLOT))
        feat[1, 2, 3, SLOT] = -1.0
        self.assertTrue(carries_column({"cand_feat": feat}, SLOT))


    def test_train_resume_and_export_read_one_column_end_to_end(self):
        import contextlib
        import gzip
        import io
        import json
        import tempfile
        from pathlib import Path
        from unittest.mock import patch

        from export_onnx4 import load_export_checkpoint
        from test_recurrent4 import write_dataset
        from training4 import driver

        original = driver.D4.TrajectoryDataset4

        def dataset(*args, **kwargs):
            result = original(*args, **kwargs)
            self.addCleanup(result.store.close)
            return result

        patcher = patch.object(driver.D4, "TrajectoryDataset4", side_effect=dataset)
        patcher.start()
        self.addCleanup(patcher.stop)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            data = root / "data"
            data.mkdir()
            write_dataset(data, [2, 2], candidates=2)
            path = data / "train/steps.jsonl.gz"
            with gzip.open(path, "rt") as stream:
                rows = [json.loads(line) for line in stream]
            for row in rows:
                for index, feat in enumerate(row["cand_feat"]):
                    row["cand_feat"][index] = list(feat)
                    row["cand_feat"][index][SLOT] = 1.0 if index else -1.0
            with gzip.open(path, "wt") as stream:
                for row in rows:
                    stream.write(json.dumps(row) + "\n")
            (data / "val").mkdir()
            (data / "val/steps.jsonl.gz").write_bytes(path.read_bytes())
            meta = json.loads((data / "meta.json").read_text())
            meta.update(encoder_version=10, val_shards=["steps.jsonl.gz"])
            (data / "meta.json").write_text(json.dumps(meta))
            forest = json.loads((data / "card_structure.json").read_text())
            old = V5.PlayerNet5(V5.Config5(d_model=16, n_heads=4, n_layers=1, d_ff=32,
                                           relational_revision=2))
            old.attach_structure(forest)
            source = root / "source.pt"
            torch.save(dict(model_version=5, encoder_version=7,
                            architecture=V5.architecture_for_config(old.cfg), config=old.cfg.to_dict(),
                            structure=forest, state_dict=old.state_dict(), trajectory_version=2,
                            sampling_version=2), source)
            output = root / "out"
            args = ["train5", "--data", str(data), "--out", str(output), "--architecture", "v5",
                    "--encoder-version", "10", "--d-model", "16", "--layers", "1", "--heads", "4",
                    "--d-ff", "32", "--batch-size", "1", "--unroll", "2", "--num-workers", "0",
                    "--epochs", "1", "--train-candidate-column", str(SLOT), "--lr", "1e-2"]
            with patch("sys.argv", args + ["--max-steps", "1", "--init", str(source),
                                           "--encoder-warmstart-to10"]), \
                    contextlib.redirect_stdout(io.StringIO()):
                driver.main()
            first = torch.load(output / "last.pt", weights_only=False)
            self.assertEqual(first["encoder_version"], 10)
            self.assertEqual(first["candidate_column"], SLOT)
            with patch("sys.argv", args + ["--max-steps", "2", "--resume", str(output / "last.pt")]), \
                    contextlib.redirect_stdout(io.StringIO()):
                driver.main()
            second = torch.load(output / "last.pt", weights_only=False)
            start = old.state_dict()
            for name, value in second["state_dict"].items():
                if name != "cand_proj.0.weight":
                    self.assertTrue(torch.equal(value, start[name]), name)
            column = second["state_dict"]["cand_proj.0.weight"]
            other = [i for i in range(column.shape[1]) if i != SLOT]
            self.assertTrue(torch.equal(column[:, other], start["cand_proj.0.weight"][:, other]))
            self.assertGreater(int(torch.count_nonzero(column[:, SLOT])), 0)
            self.assertFalse(torch.equal(first["state_dict"]["cand_proj.0.weight"][:, SLOT], column[:, SLOT]),
                             "a resumed run keeps training the column")
            checkpoint, net, _ = load_export_checkpoint(output / "net.pt")
            self.assertEqual(checkpoint["encoder_version"], 10)
            self.assertTrue(torch.equal(net.state_dict()["cand_proj.0.weight"][:, other],
                                        start["cand_proj.0.weight"][:, other]))


if __name__ == "__main__":
    unittest.main()

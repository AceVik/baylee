"""Regression tests for the actual sequence and probability failures in v4."""
import gzip
import json
from pathlib import Path
import random
import tempfile
import unittest

import numpy as np
import torch
from torch.utils.data import DataLoader

import dataset4 as D
import model4 as M
from export_onnx4 import make_dummy_inputs
from train_rl4 import compute_reverse_gae
from checkpoint4 import atomic_save, rng_state, restore_rng


def write_dataset(root, lengths, candidates=1):
    (root / "train").mkdir()
    meta = {"encoder_version": M.ENCODER_VERSION, "trajectory_version": M.TRAJECTORY_VERSION,
            "sampling_version": M.SAMPLING_VERSION, "cards": [M.UNKNOWN_CARD],
            "train_shards": ["steps.jsonl.gz"], "val_shards": []}
    from test_structure4 import small_forest
    from model4.structure import structure_digest
    forest = small_forest()
    meta["structure_sha256"] = structure_digest(forest)
    (root / "card_structure.json").write_text(json.dumps(forest))
    (root / "meta.json").write_text(json.dumps(meta))
    for table in ("card_dsl.bin", "card_dsl_back.bin", "token_dsl.bin"):
        np.zeros((1, M.CARD_DSL_WIDTH), dtype=np.float32).tofile(root / table)
    with gzip.open(root / "train" / "steps.jsonl.gz", "wt") as f:
        for game, length in enumerate(lengths):
            for cursor in range(length):
                decision = cursor == length - 1
                step = {"game_id": game, "seat": 0, "step_cursor": cursor,
                        "is_decision": decision, "is_terminal": decision,
                        "head": 0, "outcome": 1.0,
                        "state": {"entity_feat": [], "entity_card_ids": [], "entity_rule_refs": [], "edges": [],
                                  "seat_feat": [[0.] * M.SEAT_FEAT_WIDTH] * 8,
                                  "global_feat": [0.] * M.GLOBAL_FEAT_WIDTH},
                        "cand_feat": [[0.] * M.CAND_FEAT_WIDTH] * (candidates if decision else 0),
                        "cand_entity_idx": [-1] * (candidates if decision else 0),
                        "cand_rel_seat_idx": [-1] * (candidates if decision else 0),
                        "cand_rule_refs": [[] for _ in range(candidates if decision else 0)],
                        "pairs": [], "target": {"single_idx": candidates - 1 if decision else 0,
                                                  "scalar_norm": 0.0}}
                f.write(json.dumps(step) + "\n")


class RecurrentContracts(unittest.TestCase):
    def test_unseen_named_card_structure_changes_action_score(self):
        torch.manual_seed(7)
        net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
        inputs = list(make_dummy_inputs(32, E=1, M_edges=1, C=2, P=1, K=1))
        inputs[M.INPUT_NAMES.index("cand_feat")].zero_()
        inputs[M.INPUT_NAMES.index("cand_entity_idx")].fill_(-1)
        inputs[M.INPUT_NAMES.index("cand_dsl")][:, 1, 20] = 1.0
        logits = net(*inputs)[1]
        self.assertGreater(float((logits[0, 0] - logits[0, 1]).abs().detach()), 1e-6)

    def test_unequal_lanes_keep_every_tail_and_never_migrate_state(self):
        for batch_size in (1, 2, 4, 8):
            rng = random.Random(19)
            lengths = [rng.randrange(1, 19) for _ in range(13)]
            trajectories, cursor = [], 0
            for length in lengths:
                trajectories.append(list(range(cursor, cursor + length)))
                cursor += length
            ds = type("Dataset", (), {"trajectories": trajectories})()
            sampler = D.StatefulBatchSampler4(ds, batch_size, shuffle=False)
            batches = list(sampler)
            self.assertEqual(sorted(i for b in batches for i in b if i >= 0), list(range(cursor)))
            for trajectory in trajectories:
                found = [(t, lane) for t, b in enumerate(batches) for lane, i in enumerate(b) if i in trajectory]
                self.assertEqual(len({lane for _, lane in found}), 1)
                self.assertEqual([t for t, _ in found], list(range(found[0][0], found[0][0] + len(trajectory))))

    def test_cache_preserves_passive_only_segments_and_large_choices(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_dataset(root, [1, 40], candidates=350)
            ds = D.TrajectoryDataset4(root, unroll_len=8)
            self.assertEqual(len(ds), 6)
            self.assertEqual(len(ds.trajectories[1]), 5)
            sample = ds[ds.trajectories[1][-1]]
            self.assertEqual(int(sample["single_idx"][-1]), 349)
            self.assertEqual(sample["cand_mask"].shape[1], 350)
            self.assertFalse(ds[1]["is_decision"].any())
            sampler = D.StatefulBatchSampler4(ds, 2, shuffle=False)
            seen = []
            for b in DataLoader(ds, batch_sampler=sampler, collate_fn=D.collate_trajectories, num_workers=2):
                seen.extend(i for i in b["segment_index"].tolist() if i >= 0)
                self.assertTrue(torch.isfinite(b["entity_feat"]).all())
            self.assertEqual(sorted(seen), list(range(6)))
            mtime = ds.store.index_path.stat().st_mtime_ns
            ds2 = D.TrajectoryDataset4(root, unroll_len=8)
            self.assertEqual(ds2.store.index_path.stat().st_mtime_ns, mtime)
            ds.store.close()
            ds2.store.close()

    def test_decisions_and_reconsideration_do_not_rewrite_memory(self):
        torch.manual_seed(3)
        cfg = M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)
        net = M.PlayerNet4(cfg).eval()
        inputs = list(make_dummy_inputs(32, E=5, M_edges=1, C=3, P=1, K=0))
        with torch.no_grad():
            observed = net(*inputs)[0]
            self.assertFalse(torch.equal(observed, inputs[-2]))
            inputs[-2] = observed
            for k in (1, 2, 4, 8):
                inputs[-1] = torch.tensor([k])
                self.assertTrue(torch.equal(net(*inputs)[0], observed))
            inputs[-1] = torch.tensor([0])
            inputs[4] = inputs[4] + 0.5
            next_h = net(*inputs)[0]
            inputs[-2] = torch.zeros_like(observed)
            reset_h = net(*inputs)[0]
            self.assertFalse(torch.allclose(next_h, reset_h))

    def test_gae_crosses_chunk_boundary_with_real_bootstrap(self):
        v = torch.tensor([[.2, .7, .1, .8]])
        outcome = torch.ones_like(v)
        valid = torch.ones_like(v, dtype=torch.bool)
        terminal = torch.tensor([[False, False, False, True]])
        full, _ = compute_reverse_gae(v, outcome, valid, terminal, .99, .95)
        tail, _ = compute_reverse_gae(v[:, 2:], outcome[:, 2:], valid[:, 2:], terminal[:, 2:], .99, .95)
        head, _ = compute_reverse_gae(v[:, :2], outcome[:, :2], valid[:, :2], terminal[:, :2], .99, .95,
                                      bootstrap_value=v[:, 2], bootstrap_advantage=tail[:, 0])
        torch.testing.assert_close(torch.cat([head, tail], 1), full)
        with self.assertRaisesRegex(ValueError, "actual next-state"):
            compute_reverse_gae(v[:, :2], outcome[:, :2], valid[:, :2], terminal[:, :2], .99, .95)

    def test_checkpoint_restores_optimizer_and_rng_for_next_update(self):
        torch.manual_seed(13)
        net = torch.nn.Linear(3, 2)
        optimizer = torch.optim.AdamW(net.parameters())
        def update():
            optimizer.zero_grad()
            loss = net(torch.randn(4, 3)).square().mean()
            loss.backward()
            optimizer.step()
            return loss.detach()
        update()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "last.pt"
            atomic_save({"state_dict": net.state_dict(), "optimizer": optimizer.state_dict(), "rng": rng_state()}, path)
            expected = update()
            weights = {k: v.clone() for k, v in net.state_dict().items()}
            saved = torch.load(path, weights_only=False)
            net.load_state_dict(saved["state_dict"])
            optimizer.load_state_dict(saved["optimizer"])
            restore_rng(saved["rng"])
            self.assertTrue(torch.equal(update(), expected))
            for key, value in net.state_dict().items():
                self.assertTrue(torch.equal(value, weights[key]))


if __name__ == "__main__":
    torch.set_num_threads(1)
    unittest.main()

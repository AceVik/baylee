"""Opt-in greedy eligibility and turn-start samples of the isolated value calibration."""
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import numpy as np
import torch

import model4 as M
from export_onnx4 import make_dummy_inputs
from value_calibration4.cache import FeatureCache
from value_calibration4.extract import extract
from value_calibration4.features import neural_decisions
from value_calibration4.metrics import stride_indices, turn_start_indices, turn_start_report


class TurnDataset:
    """One six-frame learner trajectory, turns 1,1,1,2,2,3 (/20), greedy net decision at cursor 4."""

    def __init__(self, width, house=False, greedy=True):
        self.unroll_len = 3
        self.trajectories = [[0, 1]]
        self.segments = [np.array([0, 1, 2]), np.array([3, 4, 5])]
        index = np.zeros(6, dtype=[("game", "<u4"), ("seat", "<u4"), ("cursor", "<u8")])
        index["game"] = 10
        index["cursor"] = np.arange(6)
        self.store = SimpleNamespace(index=index)
        args = make_dummy_inputs(width, B=1, E=3, M_edges=1, C=3, P=1, K=1)
        self.frame = {name: value[0].numpy() for name, value in zip(M.INPUT_NAMES[:-2], args[:-2])}
        self.turns = np.array([1, 1, 1, 2, 2, 3], dtype=np.float32) / 20
        self.house, self.greedy = house, greedy

    def __len__(self):
        return len(self.segments)

    def __getitem__(self, idx):
        rows = self.segments[idx]
        out = {key: np.stack([value] * 3) for key, value in self.frame.items()}
        for prefix in ("cand", "pair"):
            for kind in ("target", "free"):
                out[f"{prefix}_{kind}_mask"] = np.zeros_like(out[f"{prefix}_mask"], dtype=np.float32)
        out["global_feat"] = out["global_feat"].copy()
        out["global_feat"][:, 0] = self.turns[rows]
        out.update(step_valid=np.ones(3, dtype=bool), is_new_traj=np.array(idx == 0), segment_index=np.array(idx),
                   is_decision=rows == 4, source_kind=np.where(rows == 4, 1 if self.house else 0, 1),
                   has_logprob=(rows == 4) & (not self.greedy), temperature=np.where(rows == 4, 0. if self.greedy else .7, 0.).astype(np.float32),
                   thinking_steps=np.where(rows == 4, 2, 0), is_terminal=rows == 5,
                   outcome=np.full(3, 1., dtype=np.float32))
        return out


class GreedyAndTurnStarts(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(5)
        self.net = M.PlayerNet4(M.Config4(d_model=16, n_heads=4, n_layers=1, d_ff=32)).eval()

    def extracted(self, path, dataset, **options):
        meta = extract(self.net, dataset, path, stride=64, batch_size=1, board_chunk=0, **options)
        (path / "complete.json").write_text(json.dumps(meta))
        return meta, FeatureCache(path, 16)

    def test_greedy_net_games_are_eligible_only_when_asked_and_house_never(self):
        batch = {k: torch.from_numpy(np.asarray(v)) for k, v in TurnDataset(16)[1].items()}
        self.assertFalse(bool(neural_decisions(batch).any()))
        self.assertEqual(neural_decisions(batch, greedy=True).tolist(), [False, True, False])
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "stochastic neural trajectory"):
                extract(self.net, TurnDataset(16), Path(temporary), stride=64, batch_size=1, board_chunk=0)
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "stochastic neural trajectory"):
                extract(self.net, TurnDataset(16, house=True), Path(temporary), stride=64, batch_size=1,
                        board_chunk=0, greedy=True)
        with tempfile.TemporaryDirectory() as temporary:
            meta, cache = self.extracted(Path(temporary), TurnDataset(16), greedy=True)
            self.assertEqual(meta["eligible_trajectories"], [0])
            self.assertEqual(cache.records["cursor"][cache.indices].tolist(), [0, 5])

    def test_turn_starts_are_extra_samples_told_apart_from_the_stride(self):
        with tempfile.TemporaryDirectory() as temporary:
            _, cache = self.extracted(Path(temporary), TurnDataset(16), greedy=True, turn_starts=True)
            self.assertEqual(cache.records["cursor"][cache.indices].tolist(), [0, 3, 5])
            self.assertEqual(cache.records["cursor"][stride_indices(cache, 64)].tolist(), [0, 5])
            self.assertEqual(cache.records["cursor"][turn_start_indices(cache)].tolist(), [0, 3, 5])
            restricted = cache.restrict(stride_indices(cache, 64))
            self.assertEqual(restricted.indices.tolist(), stride_indices(cache, 64).tolist())
            self.assertEqual(cache.indices.tolist(), [0, 1, 2])
            with self.assertRaisesRegex(ValueError, "only drop admitted records"):
                restricted.restrict(np.array([1]))
            report = turn_start_report(self.net.seat_value_head, cache, "cpu")
            self.assertEqual((report["source"]["samples"], report["calibrated"]["samples"]), (3, 3))
            # The untouched head reproduces the source value exactly.
            self.assertAlmostEqual(report["source"]["frame_weighted"]["mse"],
                                   report["calibrated"]["frame_weighted"]["mse"], places=6)


if __name__ == "__main__":
    unittest.main()

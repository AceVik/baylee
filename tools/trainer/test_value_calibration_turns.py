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


class LearnerScan(unittest.TestCase):
    def test_header_scan_keeps_exactly_the_eligible_trajectories(self):
        from trajectory_store4 import BST4_HDR
        from value_calibration4.extract import learner_trajectories

        def header(source, flags, temperature):
            return BST4_HDR.pack(b"BST4", 1, 0, 0, 0, 0, flags, 0, 0, source, 0, 2, 0,
                                 *([0.0] * 8), temperature, *([0] * 12))
        frames = {1: header(1, 1, 0.), 4: header(2, 1, 0.), 5: header(0, 1, 0.), 7: header(0, 1 | 4, .7)}
        index = np.zeros(9, dtype=[("flags", "<u4")])
        index["flags"][[1, 4, 5, 7]] = 1
        dataset = SimpleNamespace(trajectories=[[0], [1], [2]],
                                  segments=[np.arange(0, 3), np.arange(3, 6), np.arange(6, 9)],
                                  store=SimpleNamespace(index=index, read_raw=lambda row: frames[row]))
        # House-only, then forced-then-greedy net, then a sampled net decision.
        self.assertEqual(learner_trajectories(dataset, greedy=True), [[1], [2]])
        self.assertEqual(learner_trajectories(dataset, greedy=False), [[2]])
        # The same rule as the tensor path that admission applies.
        batch = {k: torch.tensor(v) for k, v in dict(step_valid=[True] * 3, is_decision=[True] * 3,
                                                     source_kind=[1, 0, 0], has_logprob=[False, False, True],
                                                     temperature=[0., 0., .7]).items()}
        self.assertEqual(neural_decisions(batch, greedy=True).tolist(), [False, True, True])
        self.assertEqual(neural_decisions(batch).tolist(), [False, False, True])


class SeedHalves(unittest.TestCase):
    def test_halves_follow_the_tactician_rule_and_refuse_foreign_seeds(self):
        from value_calibration4.metrics import seed_halves
        records = np.zeros(6, dtype=[("game", "<u8")])
        records["game"] = [1, 2, 3, 4, 4, 5]
        cache = FeatureCache.__new__(FeatureCache)
        cache.records, cache.indices = records, np.arange(6)
        seeds = {1: 1150301000000000000, 2: 1150301000000000010, 3: 1150301000000000020,
                 4: 1150301000000000030, 5: 1150301000000000040}
        select, report = seed_halves(cache, seeds, 10)
        self.assertEqual(records["game"][select].tolist(), [1, 3, 5])
        self.assertEqual(records["game"][report].tolist(), [2, 4, 4])
        seeds[5] += 1
        with self.assertRaisesRegex(ValueError, "divisible"):
            seed_halves(cache, seeds, 10)

    def test_game_bootstrap_runs_natively_on_cached_samples(self):
        from value_calibration4.metrics import bootstrap
        torch.manual_seed(5)
        net = M.PlayerNet4(M.Config4(d_model=16, n_heads=4, n_layers=1, d_ff=32)).eval()
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            meta = extract(net, TurnDataset(16), path, stride=64, batch_size=1, board_chunk=0,
                           greedy=True, turn_starts=True)
            (path / "complete.json").write_text(json.dumps(meta))
            cache = FeatureCache(path, 16)
            report = bootstrap(net.seat_value_head, cache, cache.indices, "cpu", 50, 3)
        self.assertEqual((report["games"], report["samples"], report["resamples"]), (1, 3, 50))
        # The untouched head is its own reference: no difference on any draw.
        self.assertEqual(report["trajectory_weighted"]["ece"]["after_minus_before"]["interval"], [0.0, 0.0])


if __name__ == "__main__":
    unittest.main()

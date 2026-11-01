"""Head-only calibration preserves deployed policy/memory and learner provenance."""
import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import torch
from torch import nn
import model4 as M
from export_onnx4 import make_dummy_inputs
from value_calibration4.cache import FeatureCache, identity, key_for, record_dtype
from value_calibration4.checkpoint import collection_plan, merged_checkpoint, file_digest, publish
from value_calibration4.extract import extract
from value_calibration4.features import neural_decisions, predict, seat_masks
from value_calibration4.fit import fit
from value_calibration4.metrics import assert_disjoint_games, trajectory_weights


def checkpoint(net):
    return dict(config=net.cfg.to_dict(), structure=net.rules.data, state_dict=net.state_dict(),
                encoder_version=M.ENCODER_VERSION, model_version=M.MODEL_VERSION,
                trajectory_version=M.TRAJECTORY_VERSION, sampling_version=M.SAMPLING_VERSION,
                optimizer={"stale": True}, h=torch.ones(1), val_metrics={"stale": True})


def signal_cache(directory, width, seed):
    rng = np.random.default_rng(seed)
    n = 80
    records = np.zeros(n, dtype=record_dtype(width))
    signal = rng.normal(size=n).astype(np.float32)
    records["features"][:, 0, 0] = signal
    records["features"][:, 1, 0] = -signal
    records["active"][:, :2] = True
    records["allied"][:, 0] = True
    records["players"] = 2
    records["trajectory"] = np.arange(n) // 4
    records["outcome"] = signal > 0
    records["source_value"] = .5
    records.tofile(directory / "records.bin")
    (directory / "complete.json").write_text(json.dumps(dict(records=n, records_sha256=file_digest(directory / "records.bin"),
                                                               eligible_trajectories=list(range(n//4)))))
    return FeatureCache(directory, width)


class TinyDataset:
    """Two three-frame trajectories, one learner identified only in its last frame."""
    def __init__(self, width, first_cursor=0):
        self.unroll_len = 2
        self.trajectories = [[0, 1], [2, 3]]
        self.segments = [np.array([0, 1]), np.array([2]), np.array([3, 4]), np.array([5])]
        index = np.zeros(6, dtype=[("game", "<u4"), ("seat", "<u4"), ("cursor", "<u8")])
        index["game"] = [10, 10, 10, 11, 11, 11]
        index["cursor"] = np.array([0, 1, 2, 0, 1, 2]) + first_cursor
        self.store = SimpleNamespace(index=index)
        args = make_dummy_inputs(width, B=1, E=3, M_edges=1, C=3, P=1, K=1)
        self.frame = {name: value[0].numpy() for name, value in zip(M.INPUT_NAMES[:-2], args[:-2])}

    def __len__(self):
        return len(self.segments)

    def __getitem__(self, idx):
        rows = self.segments[max(idx, 0)]
        out = {key: np.stack([value, value]) for key, value in self.frame.items()}
        for prefix in ("cand", "pair"):
            for kind in ("target", "free"):
                out[f"{prefix}_{kind}_mask"] = np.zeros_like(out[f"{prefix}_mask"], dtype=np.float32)
        valid = np.arange(2) < len(rows)
        if idx < 0:
            valid[:] = False
        out.update(step_valid=valid, is_new_traj=np.array(idx in (0, 2, -1)),
                   segment_index=np.array(idx), is_decision=np.zeros(2, dtype=bool),
                   source_kind=np.ones(2, dtype=np.int64), has_logprob=np.zeros(2, dtype=bool),
                   temperature=np.zeros(2, dtype=np.float32), thinking_steps=np.zeros(2, dtype=np.int64),
                   is_terminal=np.array([idx in (1, 3), False]), outcome=np.full(2, .5, dtype=np.float32))
        if idx == 1:
            out["is_decision"][0] = True
            out["source_kind"][0] = 0
            out["has_logprob"][0] = True
            out["temperature"][0] = .6
            out["thinking_steps"][0] = 2
        return out


class ValueCalibration(unittest.TestCase):
    def test_native_cached_features_reuses_sealed_records_without_reextracting(self):
        from value_calibration4.extract import cached_features
        with tempfile.TemporaryDirectory() as directory:
            dataset=TinyDataset(16)
            dataset.fingerprint='fixture-data'
            dataset.structure=self.net.rules
            cache=cached_features(self.net,dataset,directory,'source',64,2,0,100)
            with patch('value_calibration4.extract.extract',side_effect=AssertionError('unexpected re-extraction')):
                reused=cached_features(self.net,dataset,directory,'source',64,2,0,100)
            self.assertEqual(cache.path,reused.path)
            self.assertEqual(cache.indices.tolist(),reused.indices.tolist())

    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(91)
        self.net = M.PlayerNet4(M.Config4(d_model=16, n_heads=4, n_layers=1, d_ff=32)).eval()

    def test_cached_readout_matches_actual_model_for_missing_eliminated_and_team_seats(self):
        args = list(make_dummy_inputs(16, B=4, E=3, M_edges=1, C=3, P=1, K=2))
        seats = args[3]
        seats[:, :, :5] = 0
        seats[0, :2, M.SEAT_FEAT_PRESENT] = 1
        seats[0, 0, M.SEAT_FEAT_SELF] = 1
        seats[1, :4, M.SEAT_FEAT_PRESENT] = 1
        seats[1, 1, M.SEAT_FEAT_TEAMMATE] = 1
        seats[1, 2, M.SEAT_FEAT_LOST] = 1
        seats[2, :3, M.SEAT_FEAT_PRESENT] = 1
        seats[2, :3, M.SEAT_FEAT_LOST] = 1
        # Lane3 has no present seats and uses deployed seat0 fallback.
        captured = []
        hook = self.net.seat_value_head.register_forward_pre_hook(lambda m, x: captured.append(x[0].detach().clone()))
        with torch.no_grad():
            expected = self.net(*args)[4]
        hook.remove()
        active, allied = seat_masks(seats)
        actual = predict(self.net.seat_value_head, captured[0], active, allied)
        torch.testing.assert_close(actual, expected, rtol=0, atol=0)
        self.assertFalse(bool(active[1, 2]))
        self.assertTrue(bool(allied[1, 1]))
        self.assertEqual(float(actual[2].detach()), 1)
        self.assertEqual(float(actual[3].detach()), 1)

    def test_full_history_extraction_keeps_earlier_passive_samples_only_for_learner(self):
        dataset = TinyDataset(16)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            meta = extract(self.net, dataset, path, stride=64, batch_size=2, board_chunk=0)
            (path / "complete.json").write_text(json.dumps(meta))
            cache = FeatureCache(path, 16)
            self.assertEqual(meta["scanned_frames"], 6)
            self.assertEqual(meta["eligible_trajectories"], [0])
            self.assertEqual(cache.records["cursor"][cache.indices].tolist(), [0, 2])
            self.assertEqual(cache.records["neural_decision"][cache.indices].tolist(), [False, True])
            x, active, allied, _ = cache.tensor_batch(cache.indices, "cpu")
            torch.testing.assert_close(predict(self.net.seat_value_head, x, active, allied),
                                       torch.from_numpy(cache.records["source_value"][cache.indices].copy()), rtol=1e-6, atol=1e-6)
            # The terminal sample must include every preceding passive observation.
            h = torch.zeros(1, 9, 16)
            capture = []
            hook = self.net.seat_value_head.register_forward_pre_hook(lambda m, x: capture.append(x[0].detach().clone()))
            with torch.no_grad():
                for cursor in range(3):
                    frame = [torch.from_numpy(dataset.frame[name][None].copy()) for name in M.INPUT_NAMES[:-2]]
                    out = self.net(*frame, h, torch.tensor([2 if cursor == 2 else 0]))
                    h = out[0]
            hook.remove()
            torch.testing.assert_close(x[-1], capture[-1][0], atol=2e-6, rtol=2e-6)

    def test_only_head_learns_signal_and_policy_memory_source_checkpoint_stay_identical(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for split in ("train", "val"):
                (root / split).mkdir()
            train = signal_cache(root / "train", 16, 1)
            val = signal_cache(root / "val", 16, 2)
            saved = checkpoint(self.net)
            torch.save(saved, root / "source.pt")
            before_hash = file_digest(root / "source.pt")
            original = {k: v.clone() for k, v in self.net.state_dict().items()}
            head, report = fit(self.net.seat_value_head, train, val, "cpu", epochs=10, lr=.02, batch_size=20)
            self.assertLess(report["validation"]["all"]["calibrated"]["trajectory_weighted"]["mse"], .12)
            self.assertGreater(report["selected_epoch"], 0)
            self.assertTrue(all(p.grad is not None and torch.isfinite(p.grad).all() for p in head.parameters()))
            merged = merged_checkpoint(saved, head, {"parent": before_hash})
            for stale in ("optimizer", "h", "val_metrics"):
                self.assertNotIn(stale, merged)
            grown = copy.deepcopy(self.net)
            grown.load_state_dict(merged["state_dict"])
            args = list(make_dummy_inputs(16, B=2, E=3, M_edges=1, C=3, P=1, K=0))
            left_h = right_h = torch.randn(2, 9, 16)
            with torch.no_grad():
                for k in (0, 2, 0, 4):
                    args[-1].fill_(k)
                    left = self.net(*args[:-2], left_h, args[-1])
                    right = grown(*args[:-2], right_h, args[-1])
                    for a, b in zip(left[:4], right[:4]):
                        torch.testing.assert_close(a, b, atol=0, rtol=0)
                    left_h, right_h = left[0], right[0]
            for k, v in self.net.state_dict().items():
                torch.testing.assert_close(v, original[k], atol=0, rtol=0)
            self.assertEqual(before_hash, file_digest(root / "source.pt"))

    def test_first_observation_is_selected_even_with_nonmultiple_cursor(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            dataset = TinyDataset(16, first_cursor=5)
            meta = extract(self.net, dataset, path, stride=64, batch_size=2, board_chunk=0)
            (path / "complete.json").write_text(json.dumps(meta))
            cache = FeatureCache(path, 16)
            self.assertEqual(cache.records["cursor"][cache.indices].tolist(), [5, 7])
            self.assertEqual(meta["scanned_frames"], 6)

    def test_cache_identity_tracks_implementation_device_and_record_integrity(self):
        dataset = SimpleNamespace(fingerprint="data", structure=SimpleNamespace(digest="rules"), unroll_len=16)
        with patch("value_calibration4.cache.extraction_sources_digest", return_value="revision-A"):
            a = identity("source", dataset, 64, 4, "cpu")
            cuda = identity("source", dataset, 64, 4, "cuda")
        with patch("value_calibration4.cache.extraction_sources_digest", return_value="revision-B"):
            b = identity("source", dataset, 64, 4, "cpu")
        self.assertNotEqual(key_for(a), key_for(b))
        self.assertNotEqual(key_for(a), key_for(cuda))
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            signal_cache(path, 16, 1)
            # Same length corruption must be rejected, not just truncated files.
            with (path / "records.bin").open("r+b") as stream:
                stream.write(b"altered")
            with self.assertRaisesRegex(ValueError, "digest"):
                FeatureCache(path, 16)

    def test_collection_plan_and_atomic_nonoverwriting_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "plan.json"
            path.write_text(json.dumps(dict(model_sha256="parent")))
            plan = collection_plan(path, "parent")
            self.assertEqual(plan["sha256"], file_digest(path))
            with self.assertRaisesRegex(ValueError, "model_sha256"):
                collection_plan(path, "other")
            output = root / "calibrated"
            def attempt():
                try:
                    publish(output, {"tensor": torch.ones(1)}, {"plan": plan})
                    return "published"
                except FileExistsError:
                    return "preserved"
            with ThreadPoolExecutor(max_workers=2) as executor:
                results = list(executor.map(lambda _: attempt(), range(2)))
            self.assertCountEqual(results, ["published", "preserved"])
            self.assertEqual(sorted(p.name for p in output.iterdir()), ["net.pt", "report.json"])
            self.assertEqual(list(root.glob("calibrated.partial-*")), [])

    def test_provenance_leakage_and_equal_trajectory_weights(self):
        batch = {key: torch.tensor([[value]]) for key, value in
                 dict(step_valid=True, is_decision=True, source_kind=0, has_logprob=True, temperature=.6).items()}
        self.assertTrue(bool(neural_decisions(batch)))
        for key, bad in (("source_kind", 1), ("has_logprob", False), ("is_decision", False), ("temperature", 0.)):
            changed = {k: v.clone() for k, v in batch.items()}
            changed[key].fill_(bad)
            self.assertFalse(bool(neural_decisions(changed)))
        data = lambda games: SimpleNamespace(store=SimpleNamespace(index={"game": np.array(games)}))
        assert_disjoint_games(data([1, 1]), data([2]))
        with self.assertRaises(ValueError):
            assert_disjoint_games(data([1, 2]), data([2, 3]))
        weights = trajectory_weights(np.array([0, 1, 1, 1]))
        np.testing.assert_allclose(weights, [.5, 1/6, 1/6, 1/6])

    def test_validation_can_retain_unchanged_source_head(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for split in ("train", "val"):
                (root / split).mkdir()
            train = signal_cache(root / "train", 16, 1)
            validation = signal_cache(root / "val", 16, 2)
            # Held-out outcomes oppose the training signal: never force adoption.
            rows = np.memmap(validation.path / "records.bin", dtype=record_dtype(16), mode="r+")
            rows["outcome"] = 1 - rows["outcome"]
            rows.flush()
            source = nn.Linear(32, 1, bias=False)
            nn.init.zeros_(source.weight)
            head, report = fit(source, train, validation, "cpu", epochs=3, lr=.02, batch_size=20)
            self.assertEqual(report["selected_epoch"], 0)
            torch.testing.assert_close(head.weight, source.weight, atol=0, rtol=0)


if __name__ == "__main__":
    unittest.main()

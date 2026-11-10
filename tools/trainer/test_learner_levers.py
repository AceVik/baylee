"""Opt-in learner throughput levers: burn-in windows, decision windows, rule closure, profile."""
import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import torch

import dataset4 as D4
import model4 as M
from test_training_execution4 import batch_fixture
from training4.burn_in import GradWindow, encode_boards_windowed, window_mask
from training4.deferred import deferred_readouts
from training4.device_loss import compute_unroll_device_loss
from training4.execution import encode_boards, plan_batch
from training4.loss_config import SupervisedLossConfig
from training4 import windows as W
from training4.phase_profile import NullProfile, PhaseProfile

CPU = torch.device("cpu")


def tiny_net(seed=5):
    torch.manual_seed(seed)
    net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64, activation_checkpointing=True))
    net.cache_thinking_kv = True
    net.prune_unused_policy_heads = True
    return net


def window_batch():
    batch = batch_fixture(b=4, u=8)
    decisions = torch.zeros(4, 8, dtype=torch.bool)
    decisions[0, [2, 7]] = True
    decisions[1, [4, 5, 7]] = True
    decisions[2, [7]] = True
    decisions[3, [1, 5]] = True
    batch["is_decision"] = decisions
    batch["step_valid"][3, 6:] = False
    return batch


def step(net, batch, window=None, detach_reference=None):
    """One production deferred update on CPU; returns loss and parameter gradients."""
    net.zero_grad(set_to_none=True)
    plans = plan_batch(batch, 0, thinking_steps=2)
    grad_window = None if window is None else GradWindow(window_mask(batch, window), CPU)
    if grad_window is None:
        boards = encode_boards(net, batch, 3, CPU)
    else:
        boards = encode_boards_windowed(net, batch, 3, CPU, grad_window)
    if detach_reference is not None:
        # Reference semantics of a gradient window by detaching after the fact.
        boards = [(torch.where(detach_reference[:, t, None, None], board, board.detach()), pad)
                  for t, (board, pad) in enumerate(boards)]
        grad_window = GradWindow(detach_reference, CPU)
    out, hidden = deferred_readouts(net, batch, boards, torch.zeros(4, 9, 32), plans, 3, CPU,
                                    grad_window=grad_window)
    aux = net.aux_head(torch.stack([o[0][:, 0] for o in out], 1))
    loss, _ = compute_unroll_device_loss(out, aux.float(), batch, plans,
                                         SupervisedLossConfig(version=5, normalization="unroll_per_head"))
    loss.backward()
    return loss.detach(), hidden.detach(), {n: None if p.grad is None else p.grad.clone()
                                            for n, p in net.named_parameters()}


class BurnInWindow(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def test_mask_marks_frames_up_to_w_before_each_decision(self):
        batch = window_batch()
        self.assertEqual(window_mask(batch, 0).tolist(), (batch["is_decision"] & batch["step_valid"]).tolist())
        mask = window_mask(batch, 2)
        self.assertEqual(mask[0].tolist(), [True, True, True, False, False, True, True, True])
        self.assertEqual(mask[1].tolist(), [False, False, True, True, True, True, True, True])
        self.assertEqual(mask[3].tolist(), [True, True, False, True, True, True, False, False])
        self.assertTrue(window_mask(batch, 8)[:3].all())

    def test_full_window_equals_default_loss_memory_and_every_gradient(self):
        # Unpadded lanes whose last frame is a decision: every row is differentiable
        # and encoded in the default chunks, so the update is bit-identical.
        batch = window_batch()
        batch["step_valid"][:] = True
        batch["is_decision"][3, 7] = True
        reference_net = tiny_net()
        windowed_net = copy.deepcopy(reference_net)
        loss_a, hidden_a, grads_a = step(reference_net, batch)
        loss_b, hidden_b, grads_b = step(windowed_net, batch, window=batch["step_valid"].shape[1])
        self.assertTrue(window_mask(batch, 8).all())
        torch.testing.assert_close(loss_b, loss_a, rtol=0, atol=0)
        torch.testing.assert_close(hidden_b, hidden_a, rtol=0, atol=0)
        for name, grad in grads_a.items():
            self.assertEqual(grad is None, grads_b[name] is None, name)
            if grad is not None:
                torch.testing.assert_close(grads_b[name], grad, rtol=0, atol=0, msg=name)

    def test_full_window_with_padding_matches_default_within_rounding(self):
        # Padded frames carry no loss; encoding them without gradient only
        # changes the row count of the trunk's weight-gradient sums.
        batch = window_batch()
        batch["is_decision"][3, 5] = True
        reference_net = tiny_net()
        loss_a, hidden_a, grads_a = step(reference_net, batch)
        loss_b, hidden_b, grads_b = step(copy.deepcopy(tiny_net()), batch, window=8)
        torch.testing.assert_close(loss_b, loss_a, rtol=0, atol=0)
        torch.testing.assert_close(hidden_b, hidden_a, rtol=0, atol=0)
        for name, grad in grads_a.items():
            self.assertEqual(grad is None, grads_b[name] is None, name)
            if grad is not None:
                torch.testing.assert_close(grads_b[name], grad, rtol=1e-5, atol=1e-7, msg=name)

    def test_no_grad_frames_match_detached_reference_for_each_window(self):
        batch = window_batch()
        for window in (0, 1, 2):
            with self.subTest(window=window):
                net = tiny_net()
                loss_a, hidden_a, grads_a = step(copy.deepcopy(net), batch,
                                                 detach_reference=window_mask(batch, window))
                loss_b, hidden_b, grads_b = step(copy.deepcopy(net), batch, window=window)
                torch.testing.assert_close(loss_b, loss_a, rtol=1e-5, atol=1e-6)
                torch.testing.assert_close(hidden_b, hidden_a, rtol=1e-5, atol=1e-6)
                for name, grad in grads_a.items():
                    self.assertEqual(grad is None, grads_b[name] is None, name)
                    if grad is not None:
                        torch.testing.assert_close(grads_b[name], grad, rtol=1e-4, atol=1e-6, msg=name)

    def test_window_zero_cuts_recurrent_cell_and_passive_trunk_gradient(self):
        batch = window_batch()
        _, _, zero = step(tiny_net(), batch, window=0)
        _, _, one = step(tiny_net(), batch, window=1)
        cell = [n for n in zero if n.startswith("recurrent_cell.")]
        self.assertTrue(all(zero[n] is None or zero[n].abs().sum() == 0 for n in cell))
        self.assertTrue(any(one[n] is not None and one[n].abs().sum() > 0 for n in cell))
        # Heads still learn from passive value/aux readouts on detached memory.
        self.assertTrue(zero["aux_head.weight"].abs().sum() > 0)
        self.assertTrue(zero["seat_value_head.0.weight"].abs().sum() > 0)


class FakeTrajectories(D4.TrajectoryDataset4):
    """Real segment/__getitem__ logic over synthetic decoded frames."""

    def __init__(self, lengths, unroll, decisions, seed=0):
        rng = np.random.default_rng(seed)
        template = {name: tensor.numpy() for name, tensor in zip(
            M.INPUT_NAMES[:-2], __import__("export_onnx4").make_dummy_inputs(32, E=5, M_edges=3, C=7, P=4, K=1)[:-2])}
        self.unroll_len = unroll
        self.frames, self.segments, self.segment_is_new_traj, self.trajectories = [], [], [], []
        flags = []
        for length, decision in zip(lengths, decisions):
            rows = np.arange(len(self.frames), len(self.frames) + length)
            for cursor in range(length):
                frame = {k: v[0].copy() for k, v in template.items()}
                frame["entity_feat"] = rng.normal(size=frame["entity_feat"].shape).astype(np.float32)
                frame["entity_mask"][:] = True
                frame["entity_mask"][rng.integers(1, 5):] = False
                frame["global_feat"] = rng.normal(size=frame["global_feat"].shape).astype(np.float32)
                is_decision = bool(decision[cursor])
                frame.update(is_decision=np.array(is_decision), loss_weight=np.array(float(is_decision), np.float32),
                             is_terminal=np.array(cursor == length - 1), head=np.array(0),
                             single_idx=np.array(int(rng.integers(0, 3))),
                             cand_target_mask=np.zeros(7, np.float32), cand_free_mask=np.ones(7, np.float32),
                             pair_target_mask=np.zeros(4, np.float32), pair_free_mask=np.ones(4, np.float32),
                             scalar_target=np.array(.5, np.float32), outcome=np.array(1., np.float32),
                             aux_target=np.zeros(4, np.float32), behavior_logprob=np.array(0., np.float32),
                             has_logprob=np.array(False), source_kind=np.array(0), temperature=np.array(0., np.float32),
                             thinking_steps=np.array(2))
                frame["cand_mask"][:] = False
                frame["cand_mask"][:3] = True
                self.frames.append(frame)
                flags.append(int(is_decision) | (2 if cursor == length - 1 else 0))
            segments = []
            for start in range(0, length, unroll):
                segments.append(len(self.segments))
                self.segments.append(rows[start:start + unroll])
                self.segment_is_new_traj.append(start == 0)
            self.trajectories.append(segments)
        self.store = SimpleNamespace(index=np.array(flags, dtype=[("flags", "<u4")]))
        self.structure = None
        self.fingerprint = "fake"

    def encode_single_step(self, row):
        return copy.deepcopy(self.frames[int(row)])


def reference_unroll(net, dataset, trajectory):
    """Default deferred path over one whole trajectory; H entering and outputs per frame."""
    rows = np.concatenate([dataset.segments[s] for s in dataset.trajectories[trajectory]])
    batch = D4.collate_trajectories([{**{k: np.stack([dataset.frames[r][k] for r in rows]) for k in dataset.frames[0]},
                                      "step_valid": np.ones(len(rows), bool), "is_new_traj": np.array(True)}])
    plans = plan_batch(batch, 0, thinking_steps=2)
    boards = encode_boards(net, batch, 2, CPU)
    entering, hidden = [], torch.zeros(1, 9, 32)
    for t, plan in enumerate(plans):
        entering.append(hidden.clone())
        k = batch["is_decision"][:, t].long() * plan.thinking
        hidden = torch.where(batch["step_valid"][:, t, None, None], net.observe_memory(boards[t][0], hidden, k), hidden)
    out, _ = deferred_readouts(net, batch, boards, torch.zeros(1, 9, 32), plans, 2, CPU)
    return entering, out


class DecisionWindows(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def test_plan_merges_overlaps_and_splits_long_runs(self):
        flags = [np.array([0, 0, 1, 0, 1, 0, 0, 0, 0, 1], bool), np.zeros(4, bool), np.array([1, 1, 1, 1, 1], bool)]
        pieces = W.plan_pieces(flags, 1, 3)
        self.assertEqual(pieces.tolist(), [(0, 1, 3), (0, 4, 1), (0, 8, 2), (2, 0, 3), (2, 3, 2)])
        self.assertTrue(W.covered_frames(flags[0], 0).tolist() == flags[0].tolist())
        statistics = W.plan_statistics(flags, pieces)
        self.assertEqual((statistics["decisions"], statistics["window_frames"]), (8, 11))

    def test_packing_places_every_piece_once_within_capacity(self):
        lengths = np.array([3, 5, 1, 7, 2, 8, 8, 4, 1, 1])
        lanes = W.pack_lanes(list(range(10)), lengths, 8)
        self.assertEqual(sorted(p for lane in lanes for p in lane), list(range(10)))
        self.assertTrue(all(sum(lengths[p] for p in lane) <= 8 for lane in lanes))
        self.assertEqual(len(lanes), 5)

    def test_stored_states_and_window_outputs_equal_full_history(self):
        decisions = [np.array([0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0], bool), np.array([0, 1, 0, 0, 0, 1, 1], bool),
                     np.array([0, 0, 0, 0, 0, 1], bool)]
        dataset = FakeTrajectories([11, 7, 6], 4, decisions)
        net = tiny_net().eval()
        for burn_in in (0, 1, 2):
            with self.subTest(burn_in=burn_in):
                plan = W.WindowTraining(dataset, burn_in, 5, 2, seed=3)
                with torch.no_grad():
                    plan.refresh(net, CPU, lanes=2, thinking_steps=2, bf16=False, workers=0, token_budget=0)
                    references = [reference_unroll(net, dataset, t) for t in range(3)]
                    for pid, (trajectory, start, _) in enumerate(plan.pieces.tolist()):
                        torch.testing.assert_close(plan.states[pid], references[trajectory][0][start][0],
                                                   rtol=1e-5, atol=1e-6)
                    batches = list(plan.sampler)
                    self.assertEqual(sorted(p for b in batches for lane in b for p in lane), list(range(len(plan.pieces))))
                    for lanes in batches:
                        host = D4.collate_trajectories([plan.lanes[lane] for lane in lanes])
                        plans = plan_batch(host, 0, thinking_steps=2)
                        boards = encode_boards(net, host, 2, CPU)
                        out, _ = deferred_readouts(net, host, boards, torch.zeros(len(lanes), 9, 32), plans, 2, CPU,
                                                   resets=plan.resets(host, CPU))
                        for lane_index, lane in enumerate(lanes):
                            t = 0
                            for pid in lane:
                                trajectory, start, length = plan.pieces[pid].tolist()
                                for j in range(length):
                                    expected = references[trajectory][1][start + j]
                                    actual = out[t]
                                    torch.testing.assert_close(actual[4][lane_index], expected[4][0], rtol=1e-5, atol=1e-6)
                                    torch.testing.assert_close(actual[0][lane_index], expected[0][0], rtol=1e-5, atol=1e-6)
                                    if dataset.frames[int(np.concatenate([dataset.segments[s] for s in dataset.trajectories[trajectory]])[start + j])]["is_decision"]:
                                        width = min(actual[1].shape[1], expected[1].shape[1])
                                        torch.testing.assert_close(actual[1][lane_index, :width], expected[1][0, :width],
                                                                   rtol=1e-5, atol=1e-5)
                                    t += 1

    def test_reset_gradient_stops_at_piece_start(self):
        decisions = [np.array([0, 1, 0, 0, 1], bool)]
        dataset = FakeTrajectories([5], 5, decisions)
        net = tiny_net()
        plan = W.WindowTraining(dataset, 0, 5, 1, seed=1)
        plan.states = torch.randn(len(plan.pieces), 9, 32, requires_grad=True)
        host = D4.collate_trajectories([plan.lanes[(0, 1)]])
        plans = plan_batch(host, 0, thinking_steps=2)
        resets = plan.resets(host, CPU)
        out, _ = deferred_readouts(net, host, encode_boards(net, host, 2, CPU), torch.zeros(1, 9, 32), plans, 2, CPU,
                                   resets=resets)
        self.assertTrue(all(r is None or not r[1].requires_grad for r in resets))
        sum(o[4].sum() for o in out).backward()
        self.assertIsNone(plan.states.grad)


class RuleClosure(unittest.TestCase):
    def test_vectorized_index_arrays_and_table_are_identical(self):
        rng = np.random.default_rng(7)
        nodes = [{"tag": 0, "bytes": [], "children": []}]
        for i in range(1, 400):
            children = sorted(set(rng.integers(1, i, size=rng.integers(0, 4)).tolist())) if i > 1 else []
            nodes.append({"tag": int(rng.integers(1, 6)), "bytes": rng.integers(0, 256, size=rng.integers(0, 5)).tolist(),
                          "children": children})
        data = {"version": 1, "dsl_source_sha256": "", "tags": ["Unknown", "A", "B", "C", "D", "E"],
                "nodes": nodes, "roots": {}}
        from model4.structure import RuleEncoder
        torch.manual_seed(3)
        loop = RuleEncoder(data, 16)
        vector = copy.deepcopy(loop)
        vector.vectorized_closure = True
        refs = torch.stack((torch.tensor(rng.integers(0, 400, size=(3, 7, 4))), torch.zeros(3, 7, 4, dtype=torch.long)), -1)
        for active in ((refs,), (refs[:1, :2],), (torch.arange(400).view(1, -1, 1, 1).expand(1, 400, 1, 2).clone(),)):
            a, b = loop.encode_active(active), vector.encode_active(active)
            torch.testing.assert_close(b, a, rtol=0, atol=0)
            self.assertTrue(torch.equal(loop._remap, vector._remap))


class Profile(unittest.TestCase):
    def test_null_profile_is_inert_and_profile_writes_phases(self):
        null = NullProfile()
        self.assertEqual(list(null.wrap([1, 2])), [1, 2])
        null.begin_step(0, None); null.mark("x"); null.end_step(); null.finish()
        batch = batch_fixture(b=2, u=3)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "profile.json"
            profile = PhaseProfile(path, 1, 2, False, CPU, decisions_per_game=10.)
            for host in profile.wrap([batch] * 4):
                profile.begin_step(0, host)
                profile.mark("work")
                profile.end_step()
            profile.finish(command={})
            report = json.loads(path.read_text())
        self.assertEqual(report["measured_steps"], 2)
        self.assertIn("work", report["phase_s"])
        self.assertGreater(report["games_per_hour"], 0)


class DriverRoute(unittest.TestCase):
    def test_full_gradient_window_update_equals_default_update(self):
        from training4 import driver
        from model4.structure import structure_digest
        batch = batch_fixture(b=4, u=4)
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        batch["step_valid"][:] = True
        forest = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data

        class Dataset:
            def __init__(self, *args, split, **kw):
                self.structure = SimpleNamespace(data=forest, digest=structure_digest(forest))
                self.fingerprint = split
                self.trajectories = [[0]]
                self.segments = [np.arange(4)]
                self.store = SimpleNamespace(index=np.array([1, 1, 1, 3], dtype=[("flags", "<u4")]))

            def __len__(self):
                return 1

        class Sampler:
            VERSION = driver.D4.StatefulBatchSampler4.VERSION

            def __init__(self, *args, **kw):
                self.epoch = 0
                self.start_batch = 0

        metrics = dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)
        states = []
        with tempfile.TemporaryDirectory() as temporary:
            for extra in ([], ["--passive-grad-window", "4", "--phase-profile", str(Path(temporary) / "p.json"),
                               "--phase-profile-warmup", "0", "--phase-profile-steps", "1", "--vectorized-rule-closure"]):
                output = Path(temporary) / str(len(extra))
                argv = ["train5", "--data", "fake", "--out", str(output), "--d-model", "32", "--layers", "1",
                        "--heads", "4", "--d-ff", "64", "--batch-size", "4", "--unroll", "4", "--board-chunk", "2",
                        "--decision-chunk", "2", "--max-steps", "1", "--epochs", "1", "--loss-version", "5",
                        "--loss-normalization", "unroll_per_head", "--activation-checkpointing", *extra]
                with patch("sys.argv", argv), patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                        patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                        patch.object(driver, "DataLoader", return_value=[]), \
                        patch.object(driver, "prefetch_batches", return_value=[(copy.deepcopy(batch), None)]), \
                        patch.object(driver, "evaluate_device", return_value=metrics), \
                        patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(io.StringIO()):
                    driver.main("v5")
                states.append(torch.load(output / "last.pt", map_location="cpu", weights_only=False))
            self.assertTrue((Path(temporary) / "p.json").exists())
        self.assertEqual(states[1]["passive_grad_window"], 4)
        for name, value in states[0]["state_dict"].items():
            torch.testing.assert_close(states[1]["state_dict"][name], value, rtol=0, atol=0, msg=name)


if __name__ == "__main__":
    unittest.main()

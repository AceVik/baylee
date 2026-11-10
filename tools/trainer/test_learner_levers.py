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
from training4.burn_in import GradWindow, encode_boards_windowed, encode_trimmed, trimmed_groups, window_mask
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

    def test_trimmed_groups_cover_rows_once_within_budget(self):
        counts = np.array([5, 1, 3, 3, 0, 2, 5, 4])
        rows = torch.tensor([0, 2, 3, 4, 5, 6, 7])
        groups = trimmed_groups(rows, counts, 3 * (9 + 3))
        self.assertEqual(sorted(int(r) for g, _ in groups for r in g), rows.tolist())
        for group, width in groups:
            self.assertEqual(width, max(int(counts[group].max()), 1))
            self.assertTrue(len(group) == 1 or len(group) * (9 + width) <= 3 * (9 + 3))

    def test_trimmed_encoding_keeps_every_visible_token(self):
        from training4.burn_in import _gather_rows
        batch = window_batch()
        torch.manual_seed(2)
        counts = torch.randint(1, 6, (4, 8))
        batch["entity_mask"] = torch.arange(5)[None, None, :] < counts[..., None]
        batch["entity_feat"] = torch.randn_like(batch["entity_feat"]) * batch["entity_mask"][..., None]
        net = tiny_net().eval()
        rows = torch.arange(32)
        with torch.no_grad():
            full, full_pad = net.encode_board(*_gather_rows(batch, rows, CPU))
            for group, width in trimmed_groups(rows, counts.reshape(-1).numpy(), 4 * 14):
                board, pad = encode_trimmed(net, batch, group, width, CPU)
                self.assertEqual(board.shape[1:], full.shape[1:])
                self.assertTrue(torch.equal(pad, full_pad[group]))
                for i, row in enumerate(group.tolist()):
                    visible = 9 + int(counts.reshape(-1)[row])
                    torch.testing.assert_close(board[i, :visible], full[row, :visible], rtol=1e-5, atol=1e-6)
                    self.assertTrue((board[i, 9 + width:] == 0).all())

    def test_trimmed_no_grad_rows_match_detached_reference(self):
        batch = window_batch()
        counts = torch.randint(1, 6, (4, 8), generator=torch.Generator().manual_seed(4))
        batch["entity_mask"] = torch.arange(5)[None, None, :] < counts[..., None]
        net = tiny_net()
        mask = window_mask(batch, 1)
        loss_a, hidden_a, grads_a = step(copy.deepcopy(net), batch, detach_reference=mask)
        net_b = copy.deepcopy(net)
        net_b.zero_grad(set_to_none=True)
        plans = plan_batch(batch, 0, thinking_steps=2)
        window = GradWindow(mask, CPU, entity_counts=counts.reshape(-1).numpy())
        boards = encode_boards_windowed(net_b, batch, 3, CPU, window)
        out, hidden_b = deferred_readouts(net_b, batch, boards, torch.zeros(4, 9, 32), plans, 3, CPU, grad_window=window)
        aux = net_b.aux_head(torch.stack([o[0][:, 0] for o in out], 1))
        loss_b, _ = compute_unroll_device_loss(out, aux.float(), batch, plans,
                                               SupervisedLossConfig(version=5, normalization="unroll_per_head"))
        loss_b.backward()
        torch.testing.assert_close(loss_b.detach(), loss_a, rtol=1e-5, atol=1e-6)
        torch.testing.assert_close(hidden_b.detach(), hidden_a, rtol=1e-5, atol=1e-6)
        for name, parameter in net_b.named_parameters():
            if grads_a[name] is not None:
                torch.testing.assert_close(parameter.grad, grads_a[name], rtol=1e-4, atol=1e-6, msg=name)

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

    def __init__(self, lengths, unroll, decisions, seed=0, present_seats=None):
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
                if present_seats is not None:
                    frame["seat_feat"] = rng.normal(size=frame["seat_feat"].shape).astype(np.float32)
                    frame["seat_feat"][:, :5] = 0
                    frame["seat_feat"][:present_seats, M.SEAT_FEAT_PRESENT] = 1
                    frame["seat_feat"][0, M.SEAT_FEAT_SELF] = 1
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

    DECISIONS = [np.array([0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0], bool), np.array([0, 1, 0, 0, 0, 1, 1], bool),
                 np.array([0, 0, 0, 0, 0, 1], bool)]

    def check_window_parity(self, dataset, burn_in, slots=9, rtol=1e-5, atol=1e-6, **store):
        """Stored H and every window output against the full-history unroll.

        Slots past `slots` hold no state any output reads, so memory is compared
        on the stored slots; values and policy logits are compared in full.
        """
        net = tiny_net().eval()
        plan = W.WindowTraining(dataset, burn_in, 5, 2, seed=3)
        with torch.no_grad():
            plan.refresh(net, CPU, lanes=2, thinking_steps=2, bf16=False, workers=0, token_budget=0, **store)
            references = [reference_unroll(net, dataset, t) for t in range(3)]
            for pid, (trajectory, start, _) in enumerate(plan.pieces.tolist()):
                stored = plan.states.read([pid])[0]
                torch.testing.assert_close(stored[:slots], references[trajectory][0][start][0][:slots],
                                           rtol=rtol, atol=atol)
                self.assertTrue((stored[slots:] == 0).all())
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
                        rows = np.concatenate([dataset.segments[s] for s in dataset.trajectories[trajectory]])
                        for j in range(length):
                            expected = references[trajectory][1][start + j]
                            actual = out[t]
                            torch.testing.assert_close(actual[4][lane_index], expected[4][0], rtol=rtol, atol=atol)
                            torch.testing.assert_close(actual[0][lane_index, :slots], expected[0][0, :slots],
                                                       rtol=rtol, atol=atol)
                            if dataset.frames[int(rows[start + j])]["is_decision"]:
                                width = min(actual[1].shape[1], expected[1].shape[1])
                                torch.testing.assert_close(actual[1][lane_index, :width], expected[1][0, :width],
                                                           rtol=rtol, atol=max(atol, 1e-5))
                            t += 1
        return plan

    def test_stored_states_and_window_outputs_equal_full_history(self):
        dataset = FakeTrajectories([11, 7, 6], 4, self.DECISIONS)
        for burn_in in (0, 1, 2):
            with self.subTest(burn_in=burn_in):
                self.check_window_parity(dataset, burn_in)

    def test_duel_store_keeps_three_slots_and_every_output(self):
        dataset = FakeTrajectories([11, 7, 6], 4, self.DECISIONS, present_seats=2)
        with torch.no_grad():   # the full history does carry state in the slots the store drops
            entering, _ = reference_unroll(tiny_net().eval(), dataset, 0)
        self.assertGreater(float(entering[5][0, 3:].abs().min(-1).values.min()), 0)
        for burn_in in (0, 2):
            with self.subTest(burn_in=burn_in):
                plan = self.check_window_parity(dataset, burn_in, slots=3, seats=2)
                self.assertEqual(plan.states.shape, (len(plan.pieces), 3, 32))
                self.assertEqual(plan.states.dtype, np.float32)

    def test_a_seat_beyond_the_store_fails_instead_of_losing_its_state(self):
        dataset = FakeTrajectories([11, 7, 6], 4, self.DECISIONS, present_seats=3)
        plan = W.WindowTraining(dataset, 0, 5, 2, seed=3)
        with torch.no_grad(), self.assertRaisesRegex(ValueError, "beyond --window-state-seats 2"):
            plan.refresh(tiny_net().eval(), CPU, 2, 2, False, 0, 0, seats=2)

    def test_bf16_store_halves_the_bytes_within_bf16_rounding(self):
        dataset = FakeTrajectories([11, 7, 6], 4, self.DECISIONS, present_seats=2)
        plan = self.check_window_parity(dataset, 1, slots=3, rtol=2e-2, atol=2e-2, seats=2, store_bf16=True)
        self.assertEqual(plan.states.dtype, np.int16)
        self.assertEqual(plan.store_bytes(32, 2, True) * 2, plan.store_bytes(32, 2, False))

    def test_file_store_equals_the_ram_store_for_any_write_and_read_order(self):
        rng = np.random.default_rng(4)
        values = torch.randn(23, 9, 16)
        with tempfile.TemporaryDirectory() as temporary:
            for seats, bf16 in ((8, False), (2, False), (2, True)):
                ram = W.StateStore(23, seats, 16, bf16)
                disk = W.StateStore(23, seats, 16, bf16, Path(temporary) / f"{seats}{bf16}.raw")
                # Scattered writes with runs of consecutive pieces, as the state pass makes them.
                for pids in (np.array([5, 6, 7, 1, 2, 9]), np.array([0, 3, 4, 8]), np.arange(22, 9, -1)):
                    ram.write(pids, values[torch.from_numpy(pids)])
                    disk.write(pids, values[torch.from_numpy(pids)])
                disk.flush()
                ids = rng.permutation(23)[:17]
                torch.testing.assert_close(disk.read(ids), ram.read(ids), rtol=0, atol=0)
                self.assertEqual(Path(disk.path).stat().st_size, W.StateStore.bytes_for(23, seats, 16, bf16))
                again = W.StateStore(23, seats, 16, bf16, disk.path, mode="r")
                torch.testing.assert_close(again.read(np.arange(23)), ram.read(np.arange(23)), rtol=0, atol=0)
                again.close()
                with self.assertRaisesRegex(ValueError, "does not hold 24 stored states"):
                    W.StateStore(24, seats, 16, bf16, disk.path, mode="r")
                disk.close(delete=True)
                self.assertFalse(Path(temporary, f"{seats}{bf16}.raw").exists())

    def test_run_store_file_is_never_mapped_and_is_removed_on_close(self):
        dataset = FakeTrajectories([6, 4], 4, [np.array([0, 0, 1, 0, 1, 0], bool), np.array([0, 1, 0, 0], bool)])
        with tempfile.TemporaryDirectory() as temporary:
            plan = W.WindowTraining(dataset, 1, 4, 2, seed=0)
            with torch.no_grad():
                plan.refresh(tiny_net().eval(), CPU, 2, 2, False, 0, 0, run_dir=temporary)
            path = Path(temporary) / "window-states.raw"
            self.assertEqual(path.stat().st_size, plan.store_bytes(32))
            ram = W.WindowTraining(dataset, 1, 4, 2, seed=0)
            with torch.no_grad():
                ram.refresh(tiny_net().eval(), CPU, 2, 2, False, 0, 0)
            self.assertIsNone(ram.states.path)
            every = np.arange(len(plan.pieces))
            torch.testing.assert_close(plan.states.read(every), ram.states.read(every), rtol=0, atol=0)
            # Its pages are page cache, not the process's resident set.
            self.assertNotIn(str(path), Path("/proc/self/maps").read_text())
            with torch.no_grad():   # a second refresh reuses the same file
                plan.refresh(tiny_net().eval(), CPU, 2, 2, False, 0, 0, run_dir=temporary)
            torch.testing.assert_close(plan.states.read(every), ram.states.read(every), rtol=0, atol=0)
            plan.close()
            self.assertFalse(path.exists())

    def test_state_cache_reuses_only_the_same_plan_and_weights(self):
        decisions = [np.array([0, 0, 1, 0, 1, 0], bool), np.array([0, 1, 0, 0], bool)]
        dataset = FakeTrajectories([6, 4], 4, decisions)
        net = tiny_net().eval()
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary) / "states.json"
            first = W.WindowTraining(dataset, 1, 4, 2, seed=0)
            with torch.no_grad():
                self.assertEqual(first.refresh(net, CPU, 2, 2, False, 0, 0, cache=cache, source="a"), "computed")
            self.assertTrue(Path(str(cache) + ".states").exists() and Path(str(cache) + ".entities.npy").exists())
            second = W.WindowTraining(dataset, 1, 4, 2, seed=0)
            with patch.object(W, "window_states", side_effect=AssertionError("recomputed")):
                self.assertEqual(second.refresh(net, CPU, 2, 2, False, 0, 0, cache=cache, source="a"), "cache")
                self.assertEqual(second.states.path, Path(str(cache) + ".states"))
                with self.assertRaisesRegex(ValueError, "another plan"):
                    second.refresh(net, CPU, 2, 2, False, 0, 0, cache=cache, source="b")
                with self.assertRaisesRegex(ValueError, "another plan"):
                    W.WindowTraining(dataset, 2, 4, 2, seed=0).refresh(net, CPU, 2, 2, False, 0, 0, cache=cache, source="a")
                with self.assertRaisesRegex(ValueError, "store layout"):   # same plan, other layout
                    second.refresh(net, CPU, 2, 2, False, 0, 0, cache=cache, source="a", store_bf16=True)
            all_pieces = np.arange(len(first.pieces))
            torch.testing.assert_close(second.states.read(all_pieces), first.states.read(all_pieces), rtol=0, atol=0)
            np.testing.assert_array_equal(second.entities, first.entities)
            # A cache of the old layout (a Torch file holding every slot) is refused unread.
            old = Path(temporary) / "old.pt"
            torch.save(dict(key=dict(version=1), states=torch.zeros(2, 9, 32)), old)
            with patch.object(torch, "load", side_effect=AssertionError("loaded")), \
                    self.assertRaisesRegex(ValueError, "older Torch window-state cache"):
                second.refresh(net, CPU, 2, 2, False, 0, 0, cache=old, source="a")
            Path(temporary, "junk.json").write_text("not json")
            with self.assertRaisesRegex(ValueError, "not a version 2"):
                second.refresh(net, CPU, 2, 2, False, 0, 0, cache=Path(temporary, "junk.json"), source="a")

    def test_reset_gradient_stops_at_piece_start(self):
        decisions = [np.array([0, 1, 0, 0, 1], bool)]
        dataset = FakeTrajectories([5], 5, decisions)
        net = tiny_net()
        plan = W.WindowTraining(dataset, 0, 5, 1, seed=1)
        plan.states = W.StateStore(len(plan.pieces), 8, 32)
        plan.states.write(np.arange(len(plan.pieces)), torch.randn(len(plan.pieces), 9, 32, requires_grad=True))
        host = D4.collate_trajectories([plan.lanes[(0, 1)]])
        plans = plan_batch(host, 0, thinking_steps=2)
        resets = plan.resets(host, CPU)
        out, _ = deferred_readouts(net, host, encode_boards(net, host, 2, CPU), torch.zeros(1, 9, 32), plans, 2, CPU,
                                   resets=resets)
        self.assertTrue(all(r is None or not r[1].requires_grad for r in resets))
        loss = sum(o[4].sum() for o in out)
        loss.backward()   # the gradient ends at the stored state
        self.assertTrue(any(p.grad is not None for p in net.parameters()))


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


class MemoryBudget(unittest.TestCase):
    def cgroup_tree(self, root, limits):
        """A cgroup v2 tree a/b/c with `limits` as the memory.max of root, a, a/b and a/b/c."""
        path = Path(root)
        for name, limit in zip(("", "a", "b", "c"), limits):
            path = path / name if name else path
            path.mkdir(parents=True, exist_ok=True)
            if limit is not None:
                (path / "memory.max").write_text(f"{limit}\n")
        cgroup = Path(root) / "self-cgroup"
        cgroup.write_text("0::/a/b/c\n")
        return cgroup

    def test_limit_is_the_flag_else_the_tightest_cgroup_else_memtotal(self):
        from training4 import memory_budget as B
        with tempfile.TemporaryDirectory() as temporary:
            meminfo = Path(temporary) / "meminfo"
            meminfo.write_text("MemTotal:       65536 kB\nMemFree:  1 kB\n")
            # A job unit with MemoryMax under a slice with a larger one: the job's limit wins.
            cgroup = self.cgroup_tree(Path(temporary) / "fs", [None, "max", 80 * B.GIB, 56 * B.GIB])
            self.assertEqual(B.cgroup_limit(cgroup, Path(temporary) / "fs"), 56 * B.GIB)
            self.assertEqual(B.memory_limit(0., cgroup, Path(temporary) / "fs", meminfo), (56 * B.GIB, "cgroup"))
            self.assertEqual(B.memory_limit(12., cgroup, Path(temporary) / "fs", meminfo), (12 * B.GIB, "flag"))
            # A flag only tightens the cgroup's limit.
            self.assertEqual(B.memory_limit(100., cgroup, Path(temporary) / "fs", meminfo), (56 * B.GIB, "cgroup"))
            # The unit's limit below a tighter slice: the slice wins.
            cgroup = self.cgroup_tree(Path(temporary) / "fs2", [None, 30 * B.GIB, "max", 56 * B.GIB])
            self.assertEqual(B.cgroup_limit(cgroup, Path(temporary) / "fs2"), 30 * B.GIB)
            # No numeric limit anywhere, or no cgroup file: MemTotal.
            cgroup = self.cgroup_tree(Path(temporary) / "fs3", ["max", "max", None, "max"])
            self.assertIsNone(B.cgroup_limit(cgroup, Path(temporary) / "fs3"))
            self.assertEqual(B.memory_limit(0., cgroup, Path(temporary) / "fs3", meminfo), (65536 * 1024, "MemTotal"))
            self.assertEqual(B.memory_limit(0., Path(temporary) / "missing", Path(temporary) / "fs3", meminfo),
                             (65536 * 1024, "MemTotal"))

    def test_round0_store_is_refused_in_ram_and_fits_as_a_duel_memory_map(self):
        from training4 import memory_budget as B
        # The round-0 dataset: 6,743,989 frames, 1,895,988 pieces, d_model 640, a 56 GiB unit.
        frames, pieces, unit, guard = 6_743_989, 1_895_988, 56 * B.GIB, 48 * B.GIB
        every_slot = W.StateStore.bytes_for(pieces, 8, 640)
        duel = W.StateStore.bytes_for(pieces, 2, 640)
        self.assertEqual(every_slot, 43_683_563_520)   # the store the OOM run held in RAM
        self.assertEqual(duel * 3, every_slot)
        shape = dict(state_workers=6, state_lanes=32, unroll=32, train_workers=2, batch=8, length=32)
        # The OOM run's layout is estimated above the whole unit; the guard's limit is a flag.
        with self.assertRaisesRegex(MemoryError, r"Estimated host memory 57.3 GiB exceeds 0.5 of the 56.0 GiB "
                                                 r"limit: state_store 40.68 GiB"):
            B.check(B.window_estimate(frames, pieces, every_slot, True, **shape), unit, .5)
        with self.assertRaisesRegex(MemoryError, "30.2 GiB exceeds 0.5 of the 48.0 GiB"):
            B.check(B.window_estimate(frames, pieces, duel, True, **shape), guard, .5)
        with tempfile.TemporaryDirectory() as temporary:
            for scale in (1, 3):   # the data may grow two to three times
                on_disk = B.window_estimate(scale * frames, scale * pieces, scale * duel, False, **shape)
                self.assertEqual(on_disk["state_store"], 0)
                total = B.check(on_disk, guard, .5, [(Path(temporary) / "not" / "yet", 1)])
                self.assertEqual(total, sum(on_disk.values()))
                self.assertLess(total, 18 * B.GIB)

    def test_stores_on_one_filesystem_add_up_against_free_disk(self):
        from training4 import memory_budget as B
        free = SimpleNamespace(free=10 * B.GIB)
        with tempfile.TemporaryDirectory() as temporary, \
                patch.object(B.shutil, "disk_usage", return_value=free):
            first, second = Path(temporary) / "cache", Path(temporary) / "run"
            B.check({}, B.GIB, .5, [(first, 5 * B.GIB)])
            with self.assertRaisesRegex(MemoryError, "needs 10.0 GiB; .* has 10.0 GiB free"):
                B.check({}, B.GIB, .5, [(first, 5 * B.GIB), (second, 5 * B.GIB)])

    def test_peak_memory_reads_hwm_rusage_and_the_cgroup_peak(self):
        from training4 import memory_budget as B
        with tempfile.TemporaryDirectory() as temporary:
            status = Path(temporary) / "status"
            status.write_text("Name: python\nVmHWM:\t 2097152 kB\nVmRSS:\t 1 kB\n")
            cgroup = self.cgroup_tree(Path(temporary) / "fs", [None, None, None, 1])
            (Path(temporary) / "fs" / "a" / "b" / "c" / "memory.peak").write_text(f"{3 * B.GIB}\n")
            peak = B.peak_memory(status, cgroup, Path(temporary) / "fs")
            self.assertEqual(peak["vm_hwm_gib"], 2.)
            self.assertEqual(peak["cgroup_peak_gib"], 3.)
            self.assertGreater(peak["self_maxrss_gib"], 0)
            self.assertIn("children_maxrss_gib", peak)
            live = B.peak_memory()   # the real process: no file is required to exist
            self.assertGreater(live["self_maxrss_gib"], 0)


def fake_driver_classes(driver, forest):
    from model4.structure import structure_digest

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

    return Dataset, Sampler


class DriverRoute(unittest.TestCase):
    TINY = ["--d-model", "32", "--layers", "1", "--heads", "4", "--d-ff", "64", "--batch-size", "4", "--unroll", "4",
            "--board-chunk", "2", "--decision-chunk", "2", "--max-steps", "1", "--epochs", "1", "--loss-version", "5",
            "--loss-normalization", "unroll_per_head"]

    def test_window_run_over_its_memory_budget_stops_before_the_state_pass(self):
        from training4 import driver
        from training4 import memory_budget as budget_module
        forest = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data
        Dataset, Sampler = fake_driver_classes(driver, forest)
        window = ["--window-burn-in", "0", "--window-length", "4", "--window-state-seats", "2", "--report-peak-rss"]
        with tempfile.TemporaryDirectory() as temporary:
            for limit, raised in (("0.001", MemoryError), ("1000", RuntimeError)):
                argv = ["train5", "--data", "fake", "--out", str(Path(temporary) / limit), *self.TINY, *window,
                        "--memory-limit-gib", limit]
                stdout = io.StringIO()
                with self.subTest(limit=limit), patch("sys.argv", argv), \
                        patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                        patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                        patch.object(driver, "DataLoader", return_value=[]), \
                        patch.object(W, "window_states", side_effect=RuntimeError("state pass")), \
                        patch.object(budget_module, "cgroup_limit", return_value=None), \
                        patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(stdout), \
                        self.assertRaises(raised) as caught:
                    driver.main("v5")
                if raised is MemoryError:
                    self.assertRegex(str(caught.exception), r"exceeds 0.5 of the 0.0 GiB limit")
                    self.assertNotIn("[memory] budget", stdout.getvalue())
                else:   # inside the budget, the run reaches the state pass
                    self.assertEqual(str(caught.exception), "state pass")
                    budget = json.loads(stdout.getvalue().split("[memory] budget ", 1)[1].splitlines()[0])
                    self.assertEqual((budget["limit_source"], budget["limit_gib"]), ("flag", 1000.))
                    self.assertEqual(budget["store_dirs"], [str(Path(temporary) / limit)])   # no cache: the run's own

    def test_full_gradient_window_update_equals_default_update(self):
        from training4 import driver
        batch = batch_fixture(b=4, u=4)
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        batch["step_valid"][:] = True
        forest = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data
        Dataset, Sampler = fake_driver_classes(driver, forest)
        metrics = dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)
        states = []
        with tempfile.TemporaryDirectory() as temporary:
            for extra in ([], ["--passive-grad-window", "4", "--phase-profile", str(Path(temporary) / "p.json"),
                               "--phase-profile-warmup", "0", "--phase-profile-steps", "1", "--vectorized-rule-closure",
                               "--skip-final-validation"]):
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
                meta = json.loads((output / "meta.json").read_text())
                self.assertEqual(bool(meta["val_metrics"]), not extra)
            self.assertTrue((Path(temporary) / "p.json").exists())
        self.assertEqual(states[1]["passive_grad_window"], 4)
        for name, value in states[0]["state_dict"].items():
            torch.testing.assert_close(states[1]["state_dict"][name], value, rtol=0, atol=0, msg=name)


if __name__ == "__main__":
    unittest.main()

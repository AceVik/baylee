"""Causal attribution fixtures for independent recurrence, layout and padding."""

import copy
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import numpy as np
import torch

import dataset4 as D
import model5 as M
from recurrent_ppo4.forward import episode_prefixes
from recurrent_ppo_chunk_diagnostic4.comparison import scalar_error, tensor_error
from recurrent_ppo_chunk_diagnostic4.execution import diagnose
from recurrent_ppo_probe4.measurement import frame_identities, measure_pass
from test_recurrent_collection5 import UnevenHistories
from test_training_execution4 import batch_fixture


class DiagnosticDataset:
    def __init__(self):
        host = batch_fixture(b=1, u=64)
        host['step_valid'].fill_(True)
        host['is_decision'].fill_(True)
        host['is_decision'][:, :3] = False
        host['thinking_steps'] = host['is_decision'].long()*2
        host['actor_mask'] = host['is_decision'].clone()
        host['behavior_lp'] = torch.zeros(1, 64)
        host['temperature'] = torch.full((1, 64), .7)
        self.rows, self.segments, self.trajectories = [], [], []
        native = []
        for game, lengths in enumerate(((64, 64), (5,), (64,), (64,), (5,))):
            history = []
            cursor = 0
            for length in lengths:
                index = len(self.rows)
                history.append(index)
                row = {name: tensor[0].numpy().copy() for name, tensor in host.items()}
                row['step_valid'][length:] = False
                row['actor_mask'][length:] = False
                row['entity_feat'][:, 0, 0] = 0.
                row['entity_feat'][:, 0, 1] = game+1
                row['segment_index'] = np.array(index)
                self.rows.append(row)
                self.segments.append(list(range(len(native), len(native)+length)))
                native.extend((game+11, game % 2, cursor+t) for t in range(length))
                cursor += length
            self.trajectories.append(history)
        self.store = SimpleNamespace(index=np.array(native,
            dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')]))

    def __getitem__(self, index):
        if index < 0:
            row = {name: np.zeros_like(value) for name, value in self.rows[0].items()}
            row['segment_index'] = np.array(-1)
            row['cand_mask'][:, 0] = True
            row['pair_mask'][:, 0] = True
            return row
        return self.rows[index]


class ArithmeticNet(torch.nn.Module):
    """Known recurrence: H'=2H+x, with a tiny merged-batch-only board perturbation."""
    def __init__(self, perturbation=0., corrupt_padding=False, corrupt_valid=False):
        super().__init__()
        self.weight = torch.nn.Parameter(torch.ones(1))
        self.cfg = SimpleNamespace(d_model=1)
        self.perturbation = perturbation
        self.corrupt_padding = corrupt_padding
        self.corrupt_valid = corrupt_valid
        self.calls = []

    def clear_structure(self):
        pass

    def prepare_structure(self, _refs):
        pass

    def encode_board(self, entity, _dsl, mask, *_inputs):
        value = entity[:, 0, :1]
        if entity.shape[0] > 4:
            value = value+self.perturbation
        board = value[:, None, :].expand(-1, 9+entity.shape[1], -1).clone()
        padded = torch.zeros(board.shape[:2], dtype=torch.bool)
        if entity.shape[0] > 4:
            if self.corrupt_padding:
                padded[~mask.any(1), 0] = True
            if self.corrupt_valid:
                padded[mask.any(1), 0] = True
        return board, padded

    def forward(self, *inputs, encoded_board):
        h, k = inputs[-2:]
        self.calls.append((inputs[0][:, 0, 1].clone(), h.clone()))
        observed = 2*h+encoded_board[0][:, :9]
        next_h = torch.where((k == 0)[:, None, None], observed, h)
        batch = h.shape[0]
        return (next_h, torch.zeros(batch, inputs[8].shape[1]),
                torch.zeros(batch, inputs[12].shape[1]), torch.zeros(batch), next_h[:, 0, 0]*.01)


class ChunkDiagnostic(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        self.data = DiagnosticDataset()
        self.identities = frame_identities(self.data)

    def run_diagnostic(self, net):
        return diagnose(net, self.data, episode_prefixes(self.data), self.identities, torch.device('cpu'))

    def test_tiny_local_error_amplifies_only_with_independent_history_and_is_attributed(self):
        net = ArithmeticNet(perturbation=5e-6)
        before = copy.deepcopy(net.state_dict())
        result = self.run_diagnostic(net)
        first = result['first_failure']
        self.assertEqual((first['game_id'], first['seat'], first['cursor']), (11, 0, 2))
        self.assertEqual(first['kind'], 'hidden')
        self.assertEqual(first['coordinate'], [0, 0])
        self.assertEqual(first['reference'], 0.)
        self.assertAlmostEqual(first['current'], 35e-6, delta=1e-10)
        self.assertEqual(first['allowed'], 2e-5)
        self.assertAlmostEqual(first['normalized_error'], 1.75, places=5)
        self.assertTrue(first['shared_reference_h']['hidden']['passed'])
        self.assertTrue(first['contiguous_shared_reference_h']['hidden']['passed'])
        self.assertFalse(first['free_running_h']['hidden']['passed'])
        self.assertTrue(first['board_masks_equal'])
        self.assertFalse(first['chunk_board_contiguous'])
        layouts = first['board_layout']
        self.assertEqual(layouts['reference']['tokens']['shape'], layouts['chunked']['tokens']['shape'])
        self.assertEqual(layouts['chunked']['tokens']['stride'][0], 4*layouts['reference']['tokens']['stride'][0])
        self.assertEqual(layouts['contiguous']['tokens']['stride'], layouts['reference']['tokens']['stride'])
        self.assertEqual(layouts['reference']['tokens']['dtype'], 'torch.float32')
        self.assertFalse(result['full_scan'])
        self.assertEqual(result['visited_frames'], 9)
        self.assertTrue(torch.equal(net.weight, before['weight']))
        self.assertIsNone(net.weight.grad)

    def test_history_refill_resets_both_memories_and_padding_never_causes_failure(self):
        for row in self.data.rows:
            row['entity_feat'][:, 0, 0] = 1.
        # Make actual padded timesteps identifiable to the fake encoder.
        for row in self.data.rows:
            row['entity_mask'][~row['step_valid']] = False
        net = ArithmeticNet(corrupt_padding=True)
        result = self.run_diagnostic(net)
        self.assertIsNone(result['first_failure'])
        self.assertTrue(result['full_scan'])
        self.assertEqual(result['visited_frames'], len(self.identities))
        self.assertEqual(len(result['completed_histories']), 5)
        self.assertEqual(result['board_mask_mismatches'], 0)
        refill_inputs = [(tags, hidden) for tags, hidden in net.calls if bool((tags == 5).any())]
        self.assertTrue(refill_inputs)
        for tags, hidden in refill_inputs[:4]:
            self.assertTrue(torch.equal(hidden[tags == 5], torch.zeros_like(hidden[tags == 5])))

    def test_valid_board_mask_mismatch_stops_at_exact_native_frame(self):
        result = self.run_diagnostic(ArithmeticNet(corrupt_valid=True))
        first = result['first_failure']
        self.assertEqual((first['game_id'], first['seat'], first['cursor']), (11, 0, 0))
        self.assertEqual(first['kind'], 'board_mask')
        self.assertFalse(first['board_masks_equal'])
        self.assertEqual(result['visited_frames'], 1)

    def test_unchanged_relative_tolerance_reports_offending_coordinate(self):
        expected = torch.tensor([[1., 0.]])
        current = torch.tensor([[1.+1e-4, 3e-5]])
        error = tensor_error(expected, current)
        self.assertFalse(error['passed'])
        self.assertEqual(error['coordinate'], [0, 1])
        self.assertAlmostEqual(error['normalized_error'], 1.5, places=5)
        self.assertTrue(scalar_error(1., 1.+1e-4)['passed'])

    def test_actual_v5_manual_reference_matches_production_at_every_valid_frame(self):
        torch.manual_seed(717)
        net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1,
            d_ff=64, relational_revision=2, candidate_context_revision=2)).eval()
        data = UnevenHistories(net)
        native, data.segments = [], []
        for game, history in enumerate(data.trajectories):
            cursor = 0
            for index in history:
                count = int(data.rows[index]['step_valid'].sum())
                data.segments.append(list(range(len(native), len(native)+count)))
                native.extend((game+1, game % 2, cursor+t) for t in range(count))
                cursor += count
        data.store = SimpleNamespace(index=np.array(native,
            dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')]))
        identities, prefixes = frame_identities(data), episode_prefixes(data)
        recorded, calls = [], 0
        original = net.forward

        def capture_reference(*args, **kwargs):
            nonlocal calls
            output = original(*args, **kwargs)
            if calls % 4 == 0:
                recorded.append((output[0].clone(), output[4].clone()))
            calls += 1
            return output

        with patch.object(net, 'forward', side_effect=capture_reference):
            result = diagnose(net, data, prefixes, identities, torch.device('cpu'))
        self.assertTrue(result['full_scan'])
        self.assertIsNone(result['first_failure'])
        production = measure_pass(net, data, prefixes, identities, torch.device('cpu'), 4, 0)
        position = 0
        for indices in D.StatefulBatchSampler4(data, 4, shuffle=False):
            for t in range(3):
                hidden, value = recorded[position]
                position += 1
                for lane, index in enumerate(indices):
                    if (index, t) in identities:
                        expected_h, expected_v = production.frames[identities[index, t]]
                        self.assertTrue(torch.equal(hidden[lane], expected_h))
                        self.assertEqual(float(value[lane]), expected_v)
        self.assertEqual(position, len(recorded))
        self.assertEqual(result['visited_frames'], len(production.frames))


if __name__ == '__main__':
    unittest.main()

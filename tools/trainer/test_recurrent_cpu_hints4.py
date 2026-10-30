"""CPU-known C/P/K metadata must preserve every per-frame model operation."""

import copy
from contextlib import redirect_stderr
import io
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import numpy as np
import torch

import model5 as M
from recurrent_ppo4.execution import recorded_step_hints
from recurrent_ppo4.forward import advance, episode_prefixes
from recurrent_ppo_probe4.driver import parse_args, probe_configurations
from recurrent_ppo_probe4.measurement import frame_identities, measure_pass
from test_recurrent_collection5 import UnevenHistories
from test_training_execution4 import batch_fixture


class RecordedCPUHints(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(119)
        self.net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1,
            d_ff=64, relational_revision=2, candidate_context_revision=2)).eval()

    def varied_batch(self):
        batch = batch_fixture(b=4, u=6)
        batch['cand_mask'].fill_(False)
        batch['pair_mask'].fill_(False)
        batch['cand_mask'][0, 1, 6] = True
        batch['cand_mask'][1, 2, 3] = True
        batch['cand_mask'][2, 3, 5] = True
        batch['pair_mask'][3, 2, 3] = True
        batch['pair_mask'][0, 4, 2] = True
        batch['is_decision'].fill_(False)
        batch['is_decision'][:, 1] = True
        batch['is_decision'][0, 2] = True
        batch['is_decision'][:3, 4] = True
        batch['thinking_steps'] = batch['is_decision'].long()*2
        batch['thinking_steps'][2, 4] = 0  # Original Forced decision uses K1.
        batch['thinking_steps'][3, 4] = 9  # Passive K remains zero, even in padding.
        return batch

    @torch.no_grad()
    def test_actual_v5_outputs_and_board_shapes_are_bit_equal_for_uneven_extents_and_k(self):
        batch = self.varied_batch()
        hints = recorded_step_hints(batch)
        self.assertEqual([hint for _, hint in hints],
                         [(1, 1, 0), (7, 1, 2), (4, 4, 2), (6, 1, 0), (1, 3, 2), (1, 1, 0)])
        self.assertEqual(hints[4][0].tolist(), [2, 2, 1, 0])
        before = copy.deepcopy(self.net.state_dict())
        h = torch.zeros(4, 9, 32)
        with patch.object(self.net, 'encode_board', wraps=self.net.encode_board) as board:
            expected, expected_h = advance(self.net, batch, h, torch.device('cpu'))
            expected_shapes = [tuple(tuple(value.shape) for value in call.args) for call in board.call_args_list]
        with patch.object(self.net, 'encode_board', wraps=self.net.encode_board) as board:
            actual, actual_h = advance(self.net, batch, h, torch.device('cpu'), cpu_execution_hints=True)
            actual_shapes = [tuple(tuple(value.shape) for value in call.args) for call in board.call_args_list]
        self.assertEqual(actual_shapes, expected_shapes)
        self.assertEqual(len(actual_shapes), 6)
        self.assertTrue(all(shape[0][0] == 4 for shape in actual_shapes))
        self.assertTrue(torch.equal(actual_h, expected_h))
        for reference, current in zip(expected, actual):
            for expected_tensor, actual_tensor in zip(reference, current):
                self.assertTrue(torch.equal(actual_tensor, expected_tensor))
        for key, value in self.net.state_dict().items():
            self.assertTrue(torch.equal(value, before[key]))

    @torch.no_grad()
    def test_complete_uneven_histories_preserve_every_actor_hidden_value_and_kl(self):
        data = UnevenHistories(self.net)
        native, data.segments = [], []
        for game, history in enumerate(data.trajectories):
            cursor = 0
            for index in history:
                length = int(data.rows[index]['step_valid'].sum())
                data.segments.append(list(range(len(native), len(native)+length)))
                native.extend((game+1, game % 2, cursor+t) for t in range(length))
                cursor += length
        data.store = SimpleNamespace(index=np.array(native,
            dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')]))
        ids, prefixes = frame_identities(data), episode_prefixes(data)
        baseline = measure_pass(self.net, data, prefixes, ids, torch.device('cpu'), 4, 0)
        hinted = measure_pass(self.net, data, prefixes, ids, torch.device('cpu'), 4, 0,
                              cpu_execution_hints=True)
        self.assertEqual(hinted.actions, baseline.actions)
        self.assertEqual(set(hinted.frames), set(baseline.frames))
        for key, (hidden, value) in baseline.frames.items():
            self.assertTrue(torch.equal(hinted.frames[key][0], hidden))
            self.assertEqual(hinted.frames[key][1], value)
        self.assertEqual(hinted.report['collection_behavior_kl'], baseline.report['collection_behavior_kl'])
        self.assertEqual(hinted.report['padded_frames'], baseline.report['padded_frames'])
        self.assertIn('cpu_execution_metadata', hinted.report['phase_seconds'])
        self.assertNotIn('cpu_execution_metadata', baseline.report['phase_seconds'])

    def test_invalid_recorded_k_is_rejected_with_and_without_hints(self):
        batch = self.varied_batch()
        batch['thinking_steps'][0, 1] = 9
        for hinted in (False, True):
            with self.subTest(hinted=hinted), self.assertRaisesRegex(ValueError, 'thinking depth exceeds'):
                advance(self.net, batch, torch.zeros(4, 9, 32), torch.device('cpu'),
                        cpu_execution_hints=hinted)
        with self.assertRaisesRegex(ValueError, 'per-frame board arithmetic'):
            advance(self.net, self.varied_batch(), torch.zeros(4, 9, 32), torch.device('cpu'),
                    cpu_execution_hints=True, board_chunk=4)

    def test_opt_in_cli_requires_fixed_b4_and_no_spatial_batching(self):
        with tempfile.TemporaryDirectory() as temporary:
            argv = ['--source', 'source.pt', '--candidate', 'candidate.pt', '--rollout-data', 'data',
                    '--admission', 'admission.json', '--out', str(Path(temporary)/'report.json'),
                    '--expected-histories', '8', '--expected-frames', '6626', '--expected-choices', '1575']
            args = parse_args(argv+['--cpu-execution-hints', '--audit-lanes', '4', '--board-chunks', '0'])
            self.assertEqual(probe_configurations(args), [(4, 0, False), (4, 0, True)])
            for invalid in (['--cpu-execution-hints'],
                            ['--cpu-execution-hints', '--audit-lanes', '4', '--board-chunks', '0', '4']):
                with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    parse_args(argv+invalid)
            self.assertFalse(parse_args(argv).cpu_execution_hints)


if __name__ == '__main__':
    unittest.main()

"""Every-history CPU tensor preflight, including fail-closed receipt behavior."""

import copy
from contextlib import ExitStack, redirect_stdout
import io
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, patch

import numpy as np
import torch

from recurrent_ppo_probe4.driver import main
from recurrent_ppo_probe4.preflight import inspect_histories
from test_training_execution4 import batch_fixture


class PreflightDataset:
    def __init__(self):
        host = batch_fixture(b=2, u=64)
        host['step_valid'].fill_(True)
        host['head'].zero_()
        host['single_idx'].zero_()
        host['source_kind'] = torch.zeros(2, 64, dtype=torch.long)
        host['has_logprob'] = host['is_decision'].clone()
        host['behavior_logprob'] = torch.zeros(2, 64)
        host['temperature'] = torch.full((2, 64), .7)
        host['thinking_steps'] = host['is_decision'].long()*2
        self.rows, self.segments, self.trajectories = [], [], []
        native_index = []
        for game, lengths in enumerate(((64, 17, 64), (33,))):
            history = []
            cursor = 0
            for length in lengths:
                segment = len(self.rows)
                history.append(segment)
                row = {name: tensor[game].numpy().copy() for name, tensor in host.items()}
                row['segment_index'] = np.array(segment)
                row['step_valid'][length:] = False
                self.rows.append(row)
                self.segments.append(list(range(len(native_index), len(native_index)+length)))
                native_index.extend((game+7, game, cursor+t) for t in range(length))
                cursor += length
            self.trajectories.append(history)
        self.store = SimpleNamespace(close=MagicMock(), index=np.array(native_index,
            dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')]))
        self.structure = SimpleNamespace(digest='catalog')
        self.fingerprint = 'data'

    def __getitem__(self, index):
        return self.rows[index]


class TensorPreflight(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        self.dataset = PreflightDataset()

    def test_loads_complete_histories_and_original_actor_masks(self):
        result = inspect_histories(self.dataset)
        self.assertEqual(result, [dict(game_id=7, seat=0, segments=3, frames=145, choices=142),
                                  dict(game_id=8, seat=1, segments=1, frames=33, choices=32)])

    def test_rejects_nonfinite_recorded_likelihood_wrong_k_and_truncated_load(self):
        for corruption in ('logprob', 'thinking', 'temperature', 'frames', 'identity'):
            dataset = copy.deepcopy(self.dataset)
            row = dataset.rows[-1]
            if corruption == 'logprob':
                row['behavior_logprob'][1] = np.nan
            elif corruption == 'thinking':
                row['thinking_steps'][1] = 1
            elif corruption == 'temperature':
                row['temperature'][1] = .9
            elif corruption == 'frames':
                row['step_valid'][32] = False
            else:
                row['segment_index'] = np.array(0)
            with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                inspect_histories(dataset)

    def test_actual_dispatch_is_cpu_no_forward_and_failed_coverage_is_not_complete(self):
        source = dict(encoder_version=7, config={'d_model': 32}, structure_sha256='catalog')
        candidate = dict(source, state_dict={'weight': torch.ones(1)},
            full_ppo_progress={'final_gate': 'accepted'},
            full_ppo_contract=dict(version=2, behavior_probability_reference='recorded_executed_action',
                source_checkpoint_sha256='source.pt', dataset_fingerprint='data',
                collection_audit='current_policy_streaming_v1', precision='float32',
                batch_size=4, unroll=64, target_kl=.02))
        for failure in ('none', 'count', 'catalog'):
            writes = []
            source_net, candidate_net = MagicMock(), MagicMock()
            for net in (source_net, candidate_net):
                net.requires_grad_.return_value = net
                net.rules.digest = 'catalog'
                net.forward.side_effect = AssertionError('Preflight must not execute the model')
            if failure == 'catalog':
                candidate_net.rules.digest = 'other catalog'
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
                prefix = 'recurrent_ppo_probe4.preflight.'
                stack.enter_context(patch(prefix+'torch.set_num_interop_threads'))
                stack.enter_context(patch(prefix+'native_source_digest', side_effect=lambda path: path.name))
                stack.enter_context(patch(prefix+'native_verify_admission'))
                stack.enter_context(patch(prefix+'torch.load', side_effect=[source, candidate]))
                loader = stack.enter_context(patch(prefix+'load_policy', side_effect=[source_net, candidate_net]))
                stack.enter_context(patch(prefix+'D.TrajectoryDataset4', return_value=self.dataset))
                stack.enter_context(patch(prefix+'select_learner_histories'))
                stack.enter_context(patch(prefix+'atomic_json', side_effect=lambda _path, report: writes.append(copy.deepcopy(report))))
                stack.enter_context(patch('torch.cuda.get_device_properties', side_effect=AssertionError('No CUDA')))
                stack.enter_context(patch('torch.cuda.set_per_process_memory_fraction', side_effect=AssertionError('No CUDA')))
                stack.enter_context(patch('recurrent_ppo_probe4.driver.measure_pass', side_effect=AssertionError('No forward')))
                stack.enter_context(redirect_stdout(io.StringIO()))
                argv = ['--source', 'source.pt', '--candidate', 'candidate.pt', '--rollout-data', 'data',
                        '--admission', 'admission.json', '--out', str(Path(temporary)/'report.json'),
                        '--preflight-only', '--device', 'cpu', '--expected-histories', '2',
                        '--expected-frames', '179' if failure == 'count' else '178', '--expected-choices', '174']
                if failure != 'none':
                    message = 'complete collection coverage' if failure == 'count' else 'rule catalogs differ'
                    with self.assertRaisesRegex(ValueError, message):
                        main(argv)
                    self.assertEqual(writes[-1]['status'], 'failed')
                    self.assertFalse(writes[-1]['all_histories_loaded_and_collated'])
                else:
                    main(argv)
                    result = writes[-1]
                    self.assertEqual(result['status'], 'complete')
                    self.assertEqual(result['mode'], 'recurrent_ppo_tensor_preflight_v1')
                    self.assertEqual((result['learner_histories'], result['frames'], result['choices']), (2, 178, 174))
                    self.assertTrue(result['all_histories_loaded_and_collated'])
                    self.assertTrue(result['source_candidate_contract_compatible'])
                    self.assertFalse(result['cuda_used'])
                self.assertEqual(loader.call_count, 2)
                for call in loader.call_args_list:
                    self.assertEqual(call.args[1], torch.device('cpu'))
                source_net.forward.assert_not_called()
                candidate_net.forward.assert_not_called()


if __name__ == '__main__':
    unittest.main()

"""Diagnostic receipts remain distinct from parity acceptance and preserve source."""

import copy
from contextlib import ExitStack, redirect_stdout
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

import torch

from recurrent_ppo_chunk_diagnostic4.driver import main
from test_recurrent_ppo_preflight4 import PreflightDataset


class DiagnosticDriver(unittest.TestCase):
    def test_cpu_preflight_never_initializes_cuda_or_forwards_and_checks_all_histories(self):
        self.run_adapter(preflight=True)

    def test_completed_first_violation_is_still_not_parity_acceptance(self):
        self.run_adapter()

    def test_source_tensor_mutation_cannot_complete_diagnostic(self):
        self.run_adapter(mutate=True)

    def run_adapter(self, preflight=False, mutate=False):
        dataset = PreflightDataset()
        dataset._policy_targets_directory = MagicMock()
        source = dict(encoder_version=7)
        net = MagicMock()
        net.requires_grad_.return_value = net
        net.rules.digest = 'catalog'
        net.state_dict.side_effect = [{'weight': torch.ones(1)},
                                     {'weight': torch.full((1,), 2. if mutate else 1.)}]
        net.parameters.return_value = []
        writes = []
        result = dict(full_scan=False, first_failure=dict(game_id=7, seat=0, cursor=0,
                      kind='hidden', coordinate=[0, 0], reference=0., current=.1,
                      allowed=2e-5, normalized_error=5000.), visited_frames=1,
                      visited_choices=0, visited_histories=[dict(game_id=7, seat=0)])
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            prefix = 'recurrent_ppo_chunk_diagnostic4.driver.'
            stack.enter_context(patch(prefix+'torch.set_num_interop_threads'))
            initialize = stack.enter_context(patch(prefix+'initialize_device', return_value=torch.device('cpu')))
            stack.enter_context(patch(prefix+'native_source_digest', return_value='source'))
            stack.enter_context(patch(prefix+'native_verify_admission'))
            stack.enter_context(patch(prefix+'torch.load', return_value=source))
            stack.enter_context(patch(prefix+'load_policy', return_value=net))
            stack.enter_context(patch(prefix+'D.TrajectoryDataset4', return_value=dataset))
            stack.enter_context(patch(prefix+'select_learner_histories', return_value={'decisions': 174, 'learner_histories': 2}))
            prepare = stack.enter_context(patch(prefix+'prepare_policy_targets', return_value=dict(
                learner_trajectories=2, frames=178, decisions_checked=174)))
            diagnostic = stack.enter_context(patch(prefix+'diagnose', return_value=result))
            stack.enter_context(patch(prefix+'atomic_json', side_effect=lambda _path, report: writes.append(copy.deepcopy(report))))
            stack.enter_context(patch('torch.cuda.current_device', side_effect=AssertionError('No CUDA')))
            stack.enter_context(patch('torch.cuda.get_device_properties', side_effect=AssertionError('No CUDA')))
            stack.enter_context(redirect_stdout(io.StringIO()))
            argv = ['--source', 'source.pt', '--rollout-data', 'data', '--admission', 'admission.json',
                    '--out', str(Path(temporary)/'report.json'), '--device', 'cpu',
                    '--expected-histories', '2', '--expected-frames', '178', '--expected-choices', '174']
            if preflight:
                argv.append('--preflight-only')
            if mutate:
                with self.assertRaisesRegex(ValueError, 'mutated source state'):
                    main(argv)
                self.assertEqual(writes[-1]['status'], 'failed')
            else:
                main(argv)
                self.assertEqual(writes[-1]['status'], 'complete')
                self.assertTrue(writes[-1]['source_model_state_unchanged'])
            for receipt in writes:
                self.assertTrue(receipt['diagnostic_only'])
                self.assertFalse(receipt['parity_accepted'])
                self.assertFalse(receipt['optimizer_constructed'])
                self.assertEqual(receipt['parameter_updates'], 0)
                self.assertEqual(receipt['model_writes'], 0)
            if preflight:
                initialize.assert_not_called()
                prepare.assert_not_called()
                diagnostic.assert_not_called()
                net.forward.assert_not_called()
                self.assertEqual((writes[-1]['frames'], writes[-1]['choices']), (178, 174))
            else:
                self.assertEqual(writes[-1]['diagnostic_completion'], 'first_violation_located')

    def test_cuda_initialization_failure_writes_failed_receipt_before_source_access(self):
        writes = []
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            prefix = 'recurrent_ppo_chunk_diagnostic4.driver.'
            stack.enter_context(patch(prefix+'torch.set_num_interop_threads'))
            stack.enter_context(patch(prefix+'initialize_device', side_effect=RuntimeError('CUDA unavailable')))
            source = stack.enter_context(patch(prefix+'native_source_digest'))
            stack.enter_context(patch(prefix+'atomic_json', side_effect=lambda _path, report: writes.append(copy.deepcopy(report))))
            with self.assertRaisesRegex(RuntimeError, 'CUDA unavailable'):
                main(['--source', 'source.pt', '--rollout-data', 'data', '--admission', 'admission.json',
                    '--out', str(Path(temporary)/'report.json'), '--device', 'cuda',
                    '--expected-histories', '8', '--expected-frames', '6626', '--expected-choices', '1575'])
            self.assertEqual([row['status'] for row in writes], ['running', 'failed'])
            self.assertFalse(writes[-1]['parity_accepted'])
            source.assert_not_called()


if __name__ == '__main__':
    unittest.main()

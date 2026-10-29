"""CPU fixtures for the fixed-weight probe's actual scheduler and parity gates."""

import copy
import io
from contextlib import ExitStack, redirect_stderr, redirect_stdout
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, patch

import numpy as np
import torch

import model5 as M
from recurrent_ppo4.forward import episode_prefixes
from recurrent_ppo4.optimization import collection_divergence
from recurrent_ppo_probe4.comparison import compare, validate_coverage
from recurrent_ppo_probe4.driver import main, parse_args, timing_delta, validate_candidate
from recurrent_ppo_probe4.measurement import Snapshot, frame_identities, measure_pass
from test_recurrent_collection5 import UnevenHistories


class FixedWeightProbe(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(418)
        self.net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1,
                            d_ff=64, relational_revision=2, candidate_context_revision=2)).eval()
        self.data = UnevenHistories(self.net)
        self.prefixes = episode_prefixes(self.data)
        self.identities = {}
        for game, history in enumerate(self.data.trajectories):
            for cursor, segment in enumerate(history):
                for t, valid in enumerate(self.data.rows[segment]['step_valid']):
                    if valid:
                        self.identities[segment, t] = (game, game % 2, cursor*3+t)
        self.device = torch.device('cpu')

    def measure(self, lanes=4, chunk=0):
        return measure_pass(self.net, self.data, self.prefixes, self.identities,
                            self.device, lanes, chunk)

    def test_actual_lane_and_spatial_sweep_matches_production_complete_kl(self):
        before = copy.deepcopy(self.net.state_dict())
        reference = self.measure()
        choices = len(reference.actions)
        validate_coverage(reference, 3, len(self.identities), choices)
        expected_kl = collection_divergence(self.net, self.data, self.prefixes, self.device, 4)
        self.assertEqual(reference.report['collection_behavior_kl'], expected_kl)
        for lanes, chunk in ((4, 4), (8, 0), (8, 4)):
            with self.subTest(lanes=lanes, chunk=chunk):
                actual = self.measure(lanes, chunk)
                parity = compare(reference, actual)
                self.assertTrue(parity['passed'])
                self.assertEqual(len(parity['per_action']), choices)
                self.assertEqual(sum(row['frames'] for row in parity['per_history']), len(self.identities))
                self.assertGreater(actual.report['padded_frames'], 0)
                self.assertEqual(actual.report['allocated_slots'], actual.report['frames']+actual.report['padded_frames'])
                self.assertIn('structure', actual.report['phase_seconds'])
                self.assertIn('likelihood_and_divergence', actual.report['phase_seconds'])
                self.assertEqual(timing_delta(actual.report, actual.report)['wall_time_ratio'], 1.)
        for key, value in self.net.state_dict().items():
            self.assertTrue(torch.equal(value, before[key]))
        self.assertTrue(all(parameter.grad is None for parameter in self.net.parameters()))

    def test_per_action_history_drift_missing_rows_and_recorded_denominator_fail(self):
        reference = self.measure()
        key = next(iter(reference.actions))
        for field in ('logprob', 'hidden', 'value', 'kl', 'near_limit'):
            changed = copy.deepcopy(reference)
            with self.subTest(field=field):
                if field == 'logprob':
                    lp, behavior = changed.actions[key]
                    changed.actions[key] = (lp+.01, behavior)
                elif field == 'hidden':
                    hidden, value = changed.frames[key]
                    changed.frames[key] = (hidden+.1, value)
                elif field == 'value':
                    hidden, value = changed.frames[key]
                    changed.frames[key] = (hidden, value+.1)
                elif field == 'kl':
                    changed.report['collection_behavior_kl'] += 1e-5
                else:
                    reference.report['collection_behavior_kl'] = .02
                    changed.report['collection_behavior_kl'] = .02
                self.assertFalse(compare(reference, changed)['passed'])
        missing = copy.deepcopy(reference)
        del missing.actions[key]
        with self.assertRaisesRegex(ValueError, 'identities differ'):
            compare(reference, missing)
        changed = copy.deepcopy(reference)
        lp, behavior = changed.actions[key]
        changed.actions[key] = (lp, behavior+.001)
        with self.assertRaisesRegex(ValueError, 'Recorded executed-action'):
            compare(reference, changed)
        with self.assertRaisesRegex(ValueError, 'complete declared collection'):
            validate_coverage(reference, 3, len(self.identities)+1, len(reference.actions))

    def test_nonfinite_and_broken_history_clear_cache_on_failure(self):
        original = self.net.forward

        def nonfinite(*args, **kwargs):
            output = list(original(*args, **kwargs))
            output[4] = torch.full_like(output[4], float('nan'))
            return tuple(output)

        with patch.object(self.net, 'forward', side_effect=nonfinite):
            with patch.object(self.net, 'clear_structure', wraps=self.net.clear_structure) as clear:
                with self.assertRaisesRegex(FloatingPointError, 'Nonfinite recurrent'):
                    self.measure()
                self.assertGreater(clear.call_count, 0)
        self.prefixes[1] = (1000,)
        with self.assertRaisesRegex(ValueError, 'contiguous history'):
            self.measure()


class ProbeContracts(unittest.TestCase):
    def test_atomic_progress_survives_later_failure_and_completion_requires_all_configs(self):
        source = dict(encoder_version=7, config={'d_model': 32}, structure_sha256='catalog')
        candidate = dict(source, state_dict={'weight': torch.ones(1)},
            full_ppo_progress={'final_gate': 'accepted'},
            full_ppo_contract=dict(version=2, behavior_probability_reference='recorded_executed_action',
                source_checkpoint_sha256='source.pt', dataset_fingerprint='data',
                collection_audit='current_policy_streaming_v1', precision='float32', batch_size=4, unroll=64, target_kl=.02))
        index = np.array([(1, 0, 0)], dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')])
        dataset = SimpleNamespace(structure=SimpleNamespace(digest='catalog'), fingerprint='data',
            trajectories=[[0]], segments=[[0]], store=SimpleNamespace(close=MagicMock(), index=index),
            _policy_targets_directory=SimpleNamespace(cleanup=MagicMock()))
        net = MagicMock()
        net.requires_grad_.return_value = net
        net.rules.digest = 'catalog'
        net.state_dict.return_value = {'weight': torch.ones(1)}
        net.parameters.return_value = []
        snapshot = Snapshot({(1, 0, 0): (0., 0.)}, {(1, 0, 0): (torch.zeros(9, 32), .5)},
            dict(wall_seconds=1., phase_seconds={'advance': .5}, choices=1, frames=1,
                 padded_frames=0, allocated_slots=1, collection_behavior_kl=0.))
        for fail in (False, True):
            writes = []
            with self.subTest(fail=fail), tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
                prefix = 'recurrent_ppo_probe4.driver.'
                stack.enter_context(patch(prefix+'torch.set_num_interop_threads'))
                stack.enter_context(patch(prefix+'native_source_digest', side_effect=lambda path: path.name))
                stack.enter_context(patch(prefix+'native_verify_admission'))
                stack.enter_context(patch(prefix+'torch.load', side_effect=[source, candidate]))
                stack.enter_context(patch(prefix+'load_policy', return_value=net))
                stack.enter_context(patch(prefix+'D.TrajectoryDataset4', return_value=dataset))
                stack.enter_context(patch(prefix+'select_learner_histories', return_value={'decisions': 1, 'learner_histories': 1}))
                stack.enter_context(patch(prefix+'prepare_policy_targets', return_value=dict(
                    learner_trajectories=1, frames=1, decisions_checked=1, tolerance=.001)))
                worker = stack.enter_context(patch(prefix+'measure_pass',
                    side_effect=[copy.deepcopy(snapshot) for _ in range(4)]+[RuntimeError('injected failure')]
                                if fail else None, return_value=snapshot))
                stack.enter_context(patch(prefix+'atomic_json', side_effect=lambda _path, report: writes.append(copy.deepcopy(report))))
                stack.enter_context(redirect_stdout(io.StringIO()))
                argv = ['--source', 'source.pt', '--candidate', 'candidate.pt', '--rollout-data', 'data',
                        '--admission', 'admission.json', '--out', str(Path(temporary)/'report.json'),
                        '--device', 'cpu', '--expected-histories', '1', '--expected-frames', '1', '--expected-choices', '1']
                if fail:
                    with self.assertRaisesRegex(RuntimeError, 'injected failure'):
                        main(argv)
                    self.assertEqual(writes[-1]['status'], 'failed')
                    self.assertEqual(writes[-1]['completed_configurations'], 4)
                    self.assertEqual(len(writes[-1]['passes']), 4)
                else:
                    main(argv)
                    self.assertEqual(writes[-1]['status'], 'complete')
                    self.assertEqual(writes[-1]['completed_configurations'], 8)
                    self.assertEqual(worker.call_count, 8)
                    self.assertTrue(writes[-1]['source_model_state_unchanged'])
                    self.assertTrue(writes[-1]['candidate_model_state_unchanged'])
                self.assertTrue(any(row['status'] == 'running' and row['completed_configurations'] == 4
                                    for row in writes))
                self.assertTrue(all(not row['optimizer_constructed'] for row in writes))

    def test_native_frame_identity_binding_and_duplicates(self):
        index = np.array([(9, 1, 0), (9, 1, 1), (10, 0, 0)],
                         dtype=[('game', 'u4'), ('seat', 'u4'), ('cursor', 'u8')])
        dataset = SimpleNamespace(trajectories=[[0], [1]], segments=[[0, 1], [2]],
                                  store=SimpleNamespace(index=index))
        self.assertEqual(frame_identities(dataset), {(0, 0): (9, 1, 0),
                         (0, 1): (9, 1, 1), (1, 0): (10, 0, 0)})
        index[2] = index[0]
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            frame_identities(dataset)

    def test_candidate_contract_binds_original_source_and_b4_u64(self):
        source = dict(encoder_version=7, config={'d_model': 32}, structure_sha256='catalog')
        candidate = dict(source, full_ppo_progress={'final_gate': 'accepted'},
            full_ppo_contract=dict(version=2, behavior_probability_reference='recorded_executed_action',
                source_checkpoint_sha256='source', dataset_fingerprint='data',
                collection_audit='current_policy_streaming_v1', precision='float32',
                batch_size=4, unroll=64, target_kl=.02))
        validate_candidate(candidate, source, 'source', 'data')
        for name, value in (('batch_size', 8), ('unroll', 32),
                            ('target_kl', .2), ('dataset_fingerprint', 'other'),
                            ('source_checkpoint_sha256', 'other')):
            changed = copy.deepcopy(candidate)
            changed['full_ppo_contract'][name] = value
            with self.assertRaisesRegex(ValueError, 'B4/U64'):
                validate_candidate(changed, source, 'source', 'data')
        changed = copy.deepcopy(candidate)
        changed['config']['d_model'] = 64
        with self.assertRaisesRegex(ValueError, 'B4/U64'):
            validate_candidate(changed, source, 'source', 'data')

    def test_cli_requires_baseline_and_bounds_no_optimizer_batch_option(self):
        with tempfile.TemporaryDirectory() as temporary:
            argv = ['--source', 'source.pt', '--candidate', 'candidate.pt', '--rollout-data', 'data',
                    '--admission', 'admission.json', '--out', str(Path(temporary)/'new.json'),
                    '--expected-histories', '8', '--expected-frames', '6626', '--expected-choices', '1575']
            args = parse_args(argv+['--warmups', '0', '--repeats', '1'])
            self.assertEqual(args.audit_lanes, [4, 8])
            self.assertEqual(args.board_chunks, [0, 4])
            for extra in (['--audit-lanes', '8'], ['--board-chunks', '4'],
                          ['--repeats', '3'], ['--batch-size', '8'], ['--unroll', '32'],
                          ['--preflight-only']):
                with self.subTest(extra=extra), redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        parse_args(argv+extra)


if __name__ == '__main__':
    unittest.main()

"""Research utility, recurrent replay, transactional gradients and resume."""
import copy
import io
import json
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import torch
import model5 as M

from checkpoint4 import rng_state
from research_ppo4.targets import frozen_advantages, prepare_targets, OBJECTIVE, TARGET_CONTRACT
from research_ppo4.checkpoint import (training_payload, restore_training, validate_resume,
                                    VERSION, MEMORY, PROBABILITY)
from research_ppo4.dataset import decode_response
from recurrent_ppo4.forward import episode_prefixes, segment_outputs
from recurrent_ppo4.optimization import ppo_loss, transactional_update
from test_recurrent_ppo5 import EpisodeDataset
from test_policy_training4 import assert_state_equal


class ResearchFixture(EpisodeDataset):
    """Actual recurrent model fixture; no claim of native collection admission."""
    def __init__(self, net):
        super().__init__(net)
        self.unroll_len = 3
        for row in self.rows:
            row['behavior_logprob'] = row.pop('behavior_lp')
            row['training_utility'] = row.pop('returns')
            row['training_terminal'] = row.pop('is_terminal')
            row.pop('advantages')
            row['has_logprob'] = row['actor_mask'].copy()
            row['source_kind'] = np.where(row['actor_mask'], 0, 2).astype(np.int64)
        self.admission = dict(frames=12, genuine_choices=8, learner_histories=2)

    def __getitem__(self, index):
        if index == -1:
            row = {k: np.zeros_like(v) for k, v in self[0].items()}
            row['cand_mask'][:, 0] = True
            row['pair_mask'][:, 0] = True
            row['segment_index'] = np.array(-1, dtype=np.int64)
            row['is_new_traj'] = np.array(True)
            return row
        row = dict(self.rows[index])
        # Parent fixture computes initial recorded log probabilities before
        # this subclass switches its label contract.
        if hasattr(self, 'ppo_cache'):
            row.update({k: v[index] for k, v in self.ppo_cache.items()})
        return row


class ResearchPPO(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(814)
        self.device = torch.device('cpu')
        self.net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64,
                                         relational_revision=2, candidate_context_revision=2)).eval()
        self.data = ResearchFixture(self.net)

    def prepare(self):
        audit = prepare_targets(self.net, self.data, self.device, batch_size=2)
        self.addCleanup(self.data._policy_targets_directory.cleanup)
        return audit

    def test_execution_excludes_tf32_and_bounds_cuda_before_model_load(self):
        from research_ppo4.driver import configure_execution
        with patch('research_ppo4.driver.configure_precision', return_value={'checked': True}) as precision, \
                patch('research_ppo4.driver.initialize_device', return_value=torch.device('cuda:0')) as device:
            actual_device, actual_precision = configure_execution('cuda')
        precision.assert_called_once_with('highest', False)
        device.assert_called_once_with('cuda')
        self.assertEqual(str(actual_device), 'cuda:0')
        self.assertEqual(actual_precision, {'checked': True})
        self.assertTrue(torch.are_deterministic_algorithms_enabled())
        self.assertTrue(torch.backends.cudnn.deterministic)
        self.assertFalse(torch.backends.cudnn.benchmark)

    def test_cpu_preflight_decodes_all_segments_without_forward_or_targets(self):
        from research_ppo4.preflight import check_tensors
        with patch.object(self.net, 'forward', side_effect=AssertionError('No forward')):
            report = check_tensors(self.data, 2)
        self.assertEqual((report['frames'], report['genuine_choices'], report['segments']), (12, 8, 4))
        self.assertFalse(report['optimizer_constructed'])
        self.assertFalse(hasattr(self.data, 'ppo_cache'))
        self.data.admission['frames'] += 1
        with self.assertRaisesRegex(ValueError, 'coverage differs'):
            check_tensors(self.data, 2)

    def test_preflight_report_separates_host_phases_without_model_forward(self):
        from research_ppo4 import driver
        source = dict(encoder_version=7)
        self.data.metadata = dict(source_checkpoint_sha256='source', encoder_version=7,
                                  structure_sha256=self.net.rules.digest, catalog={})
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'preflight'
            args = SimpleNamespace(resume=None, out=str(out), device='cpu', seed=31431,
                                   init='source.pt', cache_dir='cache', conversion_sha256='conversion',
                                   cache_sha256='cache', pins='pins', pins_sha256='pins',
                                   admission='admission', unroll=3, batch_size=2, preflight_only=True)
            with patch.object(driver, 'arguments', return_value=args), \
                    patch.object(driver, 'configure_execution', return_value=(self.device, {})), \
                    patch.object(driver.torch, 'set_num_interop_threads'), \
                    patch.object(driver.torch, 'load', return_value=source), \
                    patch.object(driver, 'native_source_digest', return_value='source'), \
                    patch.object(driver, 'load_policy', return_value=self.net), \
                    patch.object(driver, 'ResearchDataset', return_value=self.data), \
                    patch.object(self.data, 'close', create=True) as close, \
                    patch.object(self.data, 'verify', create=True) as verify, \
                    patch.object(driver, 'require_unchanged'), \
                    patch.object(self.net, 'forward', side_effect=AssertionError('No forward')):
                driver.main()
            report = json.loads((out / 'report.json').read_text())
        self.assertEqual((report['frames'], report['genuine_choices']), (12, 8))
        close.assert_called_once_with()
        verify.assert_called_once_with()
        timings = report['timings']
        self.assertFalse(timings['cuda_synchronized'])
        self.assertEqual(set(timings['phase_seconds']), {
            'source_checkpoint_digest', 'source_checkpoint_load', 'policy_construction',
            'native_handshake_dataset_preparation', 'preflight_tensor_materialization_validation',
            'final_verification'})
        self.assertTrue(all(seconds >= 0 for seconds in timings['phase_seconds'].values()))
        self.assertFalse(hasattr(self.data, 'ppo_cache'))

    def test_phase_timings_accumulate_and_snapshot_without_changing_exception(self):
        from research_ppo4.timing import PhaseTimings
        with patch('research_ppo4.timing.perf_counter', side_effect=[0., 1., 3., 4., 7., 8.]):
            timings = PhaseTimings()
            with timings.measure('publication'):
                pass
            with self.assertRaisesRegex(ValueError, 'preserved'):
                with timings.measure('publication'):
                    raise ValueError('preserved')
            snapshot = timings.snapshot()
        self.assertEqual(snapshot['phase_seconds'], {'publication': 5.})
        self.assertEqual(snapshot['elapsed_seconds'], 8.)
        snapshot['phase_seconds']['publication'] = 99
        self.assertEqual(timings.seconds['publication'], 5.)

    def test_zero_returns_do_not_center_negative_advantages_into_rewards(self):
        utility = torch.zeros(1, 4)
        value = torch.tensor([[.1, .2, .3, .4]], requires_grad=True)
        valid = torch.tensor([[True, True, True, False]])
        actor = torch.tensor([[False, True, True, True]])
        advantage, returns = frozen_advantages(utility, value.detach(), valid, actor)
        torch.testing.assert_close(advantage, torch.tensor([[0., -.2, -.3, 0.]]))
        self.assertEqual(float(returns.sum()), 0)
        logits = torch.zeros(2, 2, requires_grad=True)
        lp = logits.log_softmax(-1)[:, 0]
        out = dict(logprob=lp, behavior=lp.detach(), advantage=advantage[actor & valid],
                   value=torch.zeros(1, requires_grad=True), returns=torch.zeros(1))
        ppo_loss(out).backward()
        self.assertGreater(float(logits.grad[:, 0].sum()), 0)
        self.assertLess(float(logits.grad[:, 1].sum()), 0)

    def test_complete_replay_preserves_recorded_lp_and_frozen_source_targets(self):
        before = copy.deepcopy(self.net.state_dict())
        audit = self.prepare()
        self.assertEqual((audit['frames'], audit['decisions_checked'], audit['segments']), (12, 8, 4))
        self.assertFalse(audit['advantage_centering'])
        assert_state_equal(self, before, self.net.state_dict())
        for index, row in enumerate(self.data.rows):
            np.testing.assert_array_equal(self.data.ppo_cache['behavior_lp'][index], row['behavior_logprob'])
            self.assertFalse(self.data.ppo_cache['actor_mask'][index, 0])
            np.testing.assert_array_equal(self.data.ppo_cache['returns'][index], row['training_utility'])
        frozen = self.data.ppo_cache['advantages'].copy()
        with torch.no_grad():
            self.net.global_proj[0].weight.mul_(1.7)
        np.testing.assert_array_equal(self.data.ppo_cache['advantages'], frozen)
        with self.assertRaisesRegex(ValueError, 'already exists'):
            self.prepare()

    def test_bad_recorded_probability_fails_before_target_publication(self):
        self.data.rows[3]['behavior_logprob'][2] += .01
        before = copy.deepcopy(self.net.state_dict())
        with self.assertRaisesRegex(ValueError, 'probability replay'):
            prepare_targets(self.net, self.data, self.device, 2)
        self.assertFalse(hasattr(self.data, 'ppo_cache'))
        assert_state_equal(self, before, self.net.state_dict())

    def test_actor_or_coverage_mismatch_fails_closed(self):
        self.data.rows[2]['actor_mask'][1] = False
        with self.assertRaisesRegex(ValueError, 'actor masks'):
            prepare_targets(self.net, self.data, self.device, 2)
        self.data = ResearchFixture(self.net)
        self.data.admission['frames'] += 1
        with self.assertRaisesRegex(ValueError, 'coverage'):
            prepare_targets(self.net, self.data, self.device, 2)

    def test_nonfinite_utility_never_becomes_a_zero_return(self):
        self.data.rows[3]['training_utility'][2] = np.nan
        with self.assertRaisesRegex(ValueError, 'Nonfinite research'):
            prepare_targets(self.net, self.data, self.device, 2)

    def test_uneven_histories_preserve_tail_and_exclude_exhausted_lane_padding(self):
        self.data.rows = [self.data.rows[0], *self.data.rows[2:]]
        self.data.rows[0]['training_terminal'][-1] = True
        for index, row in enumerate(self.data.rows):
            row['segment_index'] = np.array(index, dtype=np.int64)
        self.data.trajectories = [[0], [1, 2]]
        self.data.admission.update(frames=9, genuine_choices=6)
        audit = self.prepare()
        self.assertEqual((audit['frames'], audit['decisions_checked'], audit['segments']), (9, 6, 3))
        np.testing.assert_array_equal(self.data.ppo_cache['behavior_lp'][2],
                                      self.data.rows[2]['behavior_logprob'])

    def test_full_model_gradients_and_rejected_update_restore_rng_weights_adam(self):
        self.prepare()
        prefixes = episode_prefixes(self.data)
        output = segment_outputs(self.net, self.data, [1, 3], prefixes, self.device)
        ppo_loss(output).backward()
        for prefix in ('relational', 'trunk_layers', 'recurrent_cell', 'thinking_block'):
            self.assertGreater(sum(float(p.grad.abs().sum()) for name, p in self.net.named_parameters()
                                   if prefix in name and p.grad is not None), 0, prefix)
        optimizer = torch.optim.AdamW(self.net.parameters(), lr=.0001)
        accepted = transactional_update(self.net, optimizer, self.data, [1], prefixes, self.device,
                                        target_kl=.02, audit_batch_size=2)
        self.assertTrue(accepted['accepted'])
        optimizer.param_groups[0]['lr'] = 1
        weights = copy.deepcopy(self.net.state_dict())
        adam = copy.deepcopy(optimizer.state_dict())
        rng = rng_state()
        rejected = transactional_update(self.net, optimizer, self.data, [3], prefixes, self.device,
                                        target_kl=1e-12, audit_batch_size=2)
        self.assertFalse(rejected['accepted'])
        assert_state_equal(self, weights, self.net.state_dict())
        assert_state_equal(self, adam, optimizer.state_dict())
        actual_rng = rng_state()
        self.assertEqual(rng['python'], actual_rng['python'])
        torch.testing.assert_close(rng['torch'], actual_rng['torch'], atol=0, rtol=0)

    def test_research_resume_next_update_exact_and_ordinary_resume_rejected(self):
        self.prepare()
        prefixes = episode_prefixes(self.data)
        optimizer = torch.optim.AdamW(self.net.parameters(), lr=.0001)
        contract = dict(version=VERSION, objective=OBJECTIVE, target_contract=TARGET_CONTRACT,
                        behavior_probability_reference=PROBABILITY, encoder_version=7,
                        dataset='fixture', source='fixture')
        source = dict(model_version=5, config=self.net.cfg.to_dict(), encoder_version=7,
                      trajectory_version=M.TRAJECTORY_VERSION, sampling_version=M.SAMPLING_VERSION,
                      architecture=M.architecture_for_config(self.net.cfg), structure={})
        first = transactional_update(self.net, optimizer, self.data, [1], prefixes, self.device,
                                     target_kl=.02, audit_batch_size=2)
        self.assertTrue(first['accepted'])
        buffer = io.BytesIO()
        torch.save(training_payload(self.net, optimizer, source, contract, dict(attempts=1)), buffer)
        expected = transactional_update(self.net, optimizer, self.data, [3], prefixes, self.device,
                                        target_kl=.02, audit_batch_size=2)
        weights, adam = copy.deepcopy(self.net.state_dict()), copy.deepcopy(optimizer.state_dict())
        buffer.seek(0)
        saved = torch.load(buffer, weights_only=False)
        fresh = M.PlayerNet5(self.net.cfg).eval()
        freshopt = torch.optim.AdamW(fresh.parameters(), lr=.0001)
        with patch.object(fresh, 'load_state_dict') as load, patch.object(freshopt, 'load_state_dict') as opt:
            for invalid in ({**saved, 'research_ppo_contract': {**contract, 'objective': 'normalized'}},
                            {**saved, 'full_ppo_contract': {}},
                            {**saved, 'memory_reconstruction': 'cached_behavior_H'}):
                with self.assertRaises(ValueError):
                    restore_training(invalid, fresh, freshopt, contract, self.device)
            load.assert_not_called()
            opt.assert_not_called()
        from recurrent_ppo4.checkpoint import validate_resume_probability
        with self.assertRaises(ValueError):
            validate_resume_probability(saved)
        self.assertEqual(restore_training(saved, fresh, freshopt, contract, self.device), {'attempts': 1})
        actual = transactional_update(fresh, freshopt, self.data, [3], prefixes, self.device,
                                      target_kl=.02, audit_batch_size=2)
        self.assertEqual(actual, expected)
        assert_state_equal(self, weights, fresh.state_dict())
        assert_state_equal(self, adam, freshopt.state_dict())

    def test_driver_rejects_legacy_resume_before_rng_model_or_optimizer(self):
        from research_ppo4.driver import main
        args = ['train_research_ppo4']
        for name in ('init', 'cache-dir', 'conversion-sha256', 'cache-sha256', 'pins',
                     'pins-sha256', 'admission', 'out', 'resume'):
            args += ['--' + name, 'unused']
        with patch('sys.argv', args), patch('torch.load', return_value={'full_ppo_contract': {'version': 2}}), \
             patch('torch.manual_seed') as seed, patch('research_ppo4.driver.load_policy') as load:
            with self.assertRaisesRegex(ValueError, 'research|Research'):
                main()
            seed.assert_not_called()
            load.assert_not_called()


class TensorProtocol(unittest.TestCase):
    @staticmethod
    def wire(descriptor, payload):
        descriptor.setdefault('schema', 'failure_terminal_research_tensors_v1')
        header = json.dumps(descriptor).encode()
        return io.BytesIO(struct.pack('<Q', len(header)) + header + struct.pack('<Q', len(payload)) + payload)

    def test_typed_tensor_decode_and_no_ordinary_target_backdoor(self):
        item = dict(name='training_utility', dtype='f32', shape=[2], offset=0, nbytes=8)
        raw = np.array([0., .5], dtype='<f4').tobytes()
        _, arrays = decode_response(self.wire(dict(arrays=[item]), raw))
        np.testing.assert_array_equal(arrays['training_utility'], [0., .5])
        for change in ({'name': 'outcome'}, {'offset': 1}, {'nbytes': 4}, {'dtype': 'object'}):
            with self.assertRaises(ValueError):
                decode_response(self.wire(dict(arrays=[{**item, **change}]), raw))
        with self.assertRaises(ValueError):
            decode_response(self.wire(dict(arrays=[item]), raw + b'x'))
        with self.assertRaises(ValueError):
            decode_response(io.BytesIO(b'bad'))


if __name__ == '__main__':
    unittest.main()

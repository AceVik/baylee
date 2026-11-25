"""Actual driver publication with tiny models; no real-game PPO certification.

Dataset admission/actor extraction are synthetic fixtures. Model forward, head
updates, KL rollback, payload assembly, atomic publish and strict reload are real.
"""
import contextlib
import copy
import io
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import numpy as np
import torch
import torch.nn.functional as F
import model4 as V4
import model5 as V5
from export_onnx4 import load_export_checkpoint, make_dummy_inputs
from model4 import INPUT_NAMES as M_INPUTS
from policy_training4 import driver
from policy_training4.optimization import POLICY_PREFIXES, accept_final_state
from value_calibration4.checkpoint import file_digest, load_source


class PolicyPublication5(unittest.TestCase):
    def setUp(self):
        self.previous_threads = torch.get_num_threads()
        torch.set_num_threads(1)
        torch.manual_seed(827)

    def tearDown(self):
        torch.set_num_threads(self.previous_threads)

    def run_fixture(self, family, mode):
        options = dict(d_model=16, n_heads=2, n_layers=1, d_ff=32, structure_width=16)
        if family == 'v4':
            net = V4.PlayerNet4(V4.Config4(**options)).eval()
        else:
            net = V5.PlayerNet5(V5.Config5(**options, relational_width=8,
                                          relational_revision=2 if family == 'v5_r2' else 1)).eval()
        config = net.cfg.to_dict()
        if family == 'v5_legacy_r1':
            config.pop('relational_revision')
        saved = dict(config=config, structure=net.rules.data, state_dict=copy.deepcopy(net.state_dict()),
                     model_version=V4.MODEL_VERSION if family == 'v4' else 5,
                     model_family='v4' if family == 'v4' else 'v5',
                     encoder_version=V4.ENCODER_VERSION, trajectory_version=V4.TRAJECTORY_VERSION,
                     sampling_version=V4.SAMPLING_VERSION, structure_sha256=net.rules.digest,
                     dsl_version=7, optimizer={'stale': True}, h=torch.ones(1), val_metrics={'stale': True})
        if family != 'v4':
            saved['architecture'] = V5.architecture_for_config(net.cfg)
        inputs = make_dummy_inputs(16, B=1, E=3, M_edges=1, C=4, P=1, K=2)
        with torch.no_grad():
            original = net(*inputs)
            choice = original[1].argmin(-1)
            old = F.log_softmax(original[1], -1).gather(1, choice[:, None]).flatten()
        count = 127 if mode == 'zero' else 128
        dataset = SimpleNamespace(structure=SimpleNamespace(digest=net.rules.digest),
                                  fingerprint='synthetic-publication-fixture',
                                  store=SimpleNamespace(close=lambda: None),
                                  trajectories=[[0]],
                                  _policy_targets_directory=SimpleNamespace(cleanup=lambda: None),
                                  ppo_cache={'actor_mask': np.ones((1, count), dtype=bool)})

        def actor_outputs(current, batch, board_chunk):
            # Repeated synthetic categorical choices supply real head gradients.
            lp = F.log_softmax(current(*inputs)[1], -1).gather(1, choice[:, None]).flatten()
            return lp.expand(count), old.expand(count), torch.ones(count)

        # One attack (a subset row) beside the certified choices: recorded from the source, then
        # measured after the step and at the end. The drifted modes move it far at that point.
        cand_mask = inputs[M_INPUTS.index('cand_mask')]
        attack = dict(step_valid=torch.ones(1, 1, dtype=torch.bool), is_decision=torch.ones(1, 1, dtype=torch.bool),
                      source_kind=torch.zeros(1, 1, dtype=torch.long), actor_mask=torch.zeros(1, 1, dtype=torch.bool),
                      head=torch.ones(1, 1, dtype=torch.long), single_idx=torch.zeros(1, 1, dtype=torch.long),
                      temperature=torch.full((1, 1), .7), cand_mask=cand_mask[:, None],
                      pair_mask=inputs[M_INPUTS.index('pair_mask')][:, None],
                      cand_free_mask=cand_mask[:, None].float(), cand_target_mask=torch.zeros(1, 1, 4),
                      pair_free_mask=torch.zeros(1, 1, 1), pair_target_mask=torch.zeros(1, 1, 1))
        passes = []

        def observe_outside(current, data, indices, device, board_chunk, observe):
            passes.append(indices)
            with torch.no_grad():
                outputs = list(current(*inputs))
            if len(passes) == dict(drifted=2, final_drifted=3, measured=2).get(mode):
                outputs[1] = outputs[1] + 9.
            observe(indices, attack, 0, outputs)

        def final_gate(current, source, measured_kl, target_kl):
            return accept_final_state(current, source,
                                      float('inf') if mode == 'final_rejected' else measured_kl, target_kl)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, output = root / 'source.pt', root / 'published'
            torch.save(saved, source)
            source_hash = file_digest(source)
            args = ['train_policy4.py', '--model', str(source), '--rollout-data', str(root / 'rollout'),
                    '--admission', str(root / 'synthetic-admission.json'),
                    '--out', str(output), '--epochs', '1', '--max-steps', '1',
                    '--lr', '.01' if mode == 'rejected' else '.001',
                    '--target-kl', '1e-12' if mode == 'rejected' else '.5', '--device', 'cpu']
            # Without a limit of its own the drift is measured, once at the end, and refuses nothing.
            args += [] if mode == 'measured' else ['--uncertified-target-kl', '.5']
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(sys, 'argv', args))
                # Keep one thread even though the real CLI requests four.
                stack.enter_context(patch.object(driver.torch, 'set_num_threads'))
                stack.enter_context(patch.object(driver.D, 'TrajectoryDataset4', return_value=dataset))
                stack.enter_context(patch('recurrent_ppo4.checkpoint.native_verify_admission'))
                stack.enter_context(patch.object(driver, 'select_learner_histories',
                                               return_value={'decisions': count, 'learner_histories': 1}))
                audit = stack.enter_context(patch('policy_training4.targets.prepare_policy_targets',
                                                 return_value={'max_logprob_error': 0.0,
                                                               'decisions_checked': count,
                                                               'learner_trajectories': 1}))
                stack.enter_context(patch.object(driver, 'batch_rows', return_value=None))
                stack.enter_context(patch.object(driver, 'actor_outputs', side_effect=actor_outputs))
                stack.enter_context(patch.object(driver, 'observe_outside', side_effect=observe_outside))
                stack.enter_context(patch.object(driver, 'accept_final_state', side_effect=final_gate))
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                driver.main()
                audit.assert_called_once()
            _, published, restored = load_source(output)
            exported, export_net, _ = load_export_checkpoint(output / 'net.pt')
            self.assertEqual(exported['config'], saved['config'])
            for key, tensor in published['state_dict'].items():
                self.assertTrue(torch.equal(export_net.state_dict()[key], tensor), key)
            report = json.loads((output / 'report.json').read_text())
            self.assertEqual(file_digest(source), source_hash)
            for key in ('config', 'model_version', 'model_family', 'architecture', 'encoder_version',
                        'trajectory_version', 'sampling_version', 'structure_sha256', 'dsl_version'):
                self.assertEqual(published.get(key), saved.get(key), key)
                self.assertEqual(key in published, key in saved, key)
            self.assertEqual(restored.rules.digest, net.rules.digest)
            self.assertEqual(published['structure'].keys(), saved['structure'].keys())
            for key, value in saved['structure'].items():
                if isinstance(value, torch.Tensor):
                    self.assertTrue(torch.equal(published['structure'][key], value), key)
                else:
                    self.assertEqual(published['structure'][key], value, key)
            self.assertFalse({'optimizer', 'h', 'val_metrics'} & published.keys())
            changed = []
            for key, original_tensor in saved['state_dict'].items():
                if not torch.equal(original_tensor, published['state_dict'][key]):
                    changed.append(key)
                    self.assertTrue(key.startswith(POLICY_PREFIXES), key)
            drift = report['uncertified_drift']['rollout_wide']
            self.assertEqual(drift['subset']['decisions'], 1)
            self.assertEqual([drift[kind]['decisions'] for kind in ('single', 'pair', 'scalar')], [0, 0, 0])
            if mode == 'accepted':
                self.assertGreater(drift['subset']['mean_kl'], 0)   # the candidate scorer moved it, within the gate
                self.assertEqual(report['updates'][0]['uncertified_drift'], drift)
                self.assertTrue(changed)
                self.assertEqual(published['provenance']['accepted_steps'], 1)
                self.assertFalse(report['checkpoint_is_unchanged_source'])
            elif mode == 'measured':
                self.assertEqual(len(passes), 2)
                self.assertGreater(drift['subset']['mean_kl'], .5)
                self.assertIsNone(report['uncertified_drift']['limit'])
                self.assertNotIn('uncertified_drift', report['updates'][0])
                self.assertTrue(changed)
                self.assertEqual(published['provenance']['accepted_steps'], 1)
            else:
                self.assertFalse(changed)
                self.assertEqual(published['provenance']['accepted_steps'], 0)
                self.assertTrue(report['checkpoint_is_unchanged_source'])
                if mode == 'zero':
                    self.assertEqual(report['updates'], [])
                    self.assertIn('insufficient_policy_decisions', published['provenance']['reason'])
                elif mode == 'rejected':
                    self.assertFalse(report['updates'][0]['accepted'])
                    self.assertIn('post_step_behavior_kl', report['updates'][0]['reason'])
                elif mode == 'drifted':
                    self.assertFalse(report['updates'][0]['accepted'])
                    self.assertIn('post_step_uncertified_drift: subset=', report['updates'][0]['reason'])
                elif mode == 'final_drifted':
                    self.assertTrue(report['updates'][0]['accepted'])
                    self.assertGreater(drift['subset']['mean_kl'], .5)
                    self.assertIn('rollout_wide_uncertified_drift_rejected (subset)', published['provenance']['reason'])
                else:
                    self.assertTrue(report['updates'][0]['accepted'])
                    self.assertEqual(report['attempted_accepted_steps'], 1)
                    self.assertIn('rollout_wide_kl_rejected', published['provenance']['reason'])
            with torch.no_grad():
                reloaded_output = restored(*inputs)
            self.assertTrue(torch.equal(original[0], reloaded_output[0]))
            self.assertTrue(torch.equal(original[4], reloaded_output[4]))

    def test_old_identity_whitelist_reproduces_v5_reload_failure(self):
        # Negative control only: primary matrix never replaces identity assembly.
        def old_identity(saved):
            return {key: saved[key] for key in ('encoder_version', 'model_version',
                    'trajectory_version', 'sampling_version', 'config')}
        with patch.object(driver, 'source_identity', side_effect=old_identity):
            with self.assertRaisesRegex(ValueError, 'V5 architecture'):
                self.run_fixture('v5_r2', 'accepted')

    def test_driver_roundtrip_preserves_supported_identities_and_update_outcomes(self):
        for family in ('v4', 'v5_r1', 'v5_legacy_r1', 'v5_r2'):
            for mode in ('accepted', 'zero', 'rejected', 'final_rejected', 'drifted', 'final_drifted',
                         'measured'):
                with self.subTest(family=family, mode=mode):
                    self.run_fixture(family, mode)


if __name__ == '__main__':
    unittest.main()

"""Symmetric categorical loss5: payment/pass semantics cannot hide target weights."""
import copy
import io
import unittest
from types import SimpleNamespace

import torch
import torch.nn.functional as F

from training4.device_loss import compute_unroll_device_loss
from training4.loss_config import SupervisedLossConfig
from training4.objective import check_resume_objective


class PassWeighting5(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    @staticmethod
    def fixture():
        batch = {
            'step_valid': torch.ones(3, 2, dtype=torch.bool),
            'is_decision': torch.ones(3, 2, dtype=torch.bool),
            'head': torch.zeros(3, 2, dtype=torch.long),
            'single_idx': torch.tensor([[0, 0], [1, 1], [0, 0]]),
            'loss_weight': torch.tensor([[1., 1.], [1., 1.], [.25, .25]]),
            'outcome': torch.tensor([[0., 0.], [.5, .5], [1., 1.]]),
            'aux_target': torch.zeros(3, 2, 6),
            'cand_mask': torch.tensor([[[True, True, True, False]]] * 3).expand(-1, 2, -1).clone(),
            'cand_feat': torch.zeros(3, 2, 4, 56),
            'global_feat': torch.zeros(3, 2, 96),
        }
        batch['cand_feat'][:, :, 0, 0] = 1.  # PassPriority
        logits = torch.tensor([[[1., -1., .3, -1e9], [-.2, .5, .3, -1e9]],
                               [[.7, .2, -.8, -1e9], [.1, -.3, .4, -1e9]],
                               [[-.5, .1, .2, -1e9], [.8, .1, -.4, -1e9]]], requires_grad=True)
        return batch, logits

    def evaluate(self, batch, logits, config):
        prepared = [(None, logits[:, t], torch.zeros(3, 1), torch.zeros(3),
                     batch['outcome'][:, t]) for t in range(2)]
        plans = [SimpleNamespace(valid=True, heads=(True, False, False, False))] * 2
        loss, _ = compute_unroll_device_loss(prepared, torch.zeros(3, 2, 6), batch, plans, config)
        return loss, torch.autograd.grad(loss, logits)[0]

    @staticmethod
    def expected(batch, logits, config):
        effective = batch['loss_weight']
        if config.outcome_weighting:
            effective = effective * (.35 + 1.30 * batch['outcome'])
        ce = torch.stack([F.cross_entropy(logits[:, t], batch['single_idx'][:, t], reduction='none')
                          for t in range(2)], dim=1)
        if config.normalization == 'unroll_per_head':
            return (ce * effective).sum() / effective.sum()
        return ((ce * effective).sum(0) / effective.sum(0)).mean()

    def test_pass_and_payment_have_only_declared_label_and_outcome_weights(self):
        for normalization in ('legacy_timestep', 'unroll_per_head'):
            for outcome_weighting in (False, True):
                config = SupervisedLossConfig(version=5, normalization=normalization,
                                              outcome_weighting=outcome_weighting)
                baseline, baseline_gradient = None, None
                for context in ('idle', 'casting29', 'fixed31', 'any32', 'announcement36', 'cancel', 'cast', 'land', 'poison_padding'):
                    with self.subTest(normalization=normalization, outcome=outcome_weighting, context=context):
                        batch, logits = self.fixture()
                        if context in ('casting29', 'fixed31', 'any32', 'announcement36'):
                            index = {'casting29': 29, 'fixed31': 31, 'any32': 32, 'announcement36': 36}[context]
                            batch['global_feat'][:, :, index] = 1.
                        elif context in ('cancel', 'cast', 'land'):
                            batch['cand_feat'][:, :, 1, {'cancel': 1, 'cast': 3, 'land': 2}[context]] = 1.
                        elif context == 'poison_padding':
                            batch['cand_feat'][:, :, 3] = float('nan')
                            batch['cand_feat'][:, :, 3, 2:4] = 1.
                        expected = self.expected(batch, logits, config)
                        expected_gradient = torch.autograd.grad(expected, logits, retain_graph=True)[0]
                        actual, gradient = self.evaluate(batch, logits, config)
                        torch.testing.assert_close(actual, expected)
                        torch.testing.assert_close(gradient, expected_gradient)
                        if baseline is None:
                            baseline, baseline_gradient = actual.detach(), gradient
                        else:
                            torch.testing.assert_close(actual, baseline, atol=0, rtol=0)
                            torch.testing.assert_close(gradient, baseline_gradient, atol=0, rtol=0)

    def test_loss4_canonical_and_exact_resume_are_unchanged(self):
        config = SupervisedLossConfig()
        expected = dict(version=4, outcome_weighting=True, normalization='legacy_timestep',
                        policy_head_weights=[1., 1., 1., 1.], detach_value_input=False,
                        detach_aux_input=False, value_weight=.5, auxiliary_weight=.25)
        self.assertEqual(config.to_dict(), expected)
        for recorded in (None, expected, {'version': 4}):
            check_resume_objective(dict(training_loss_version=4, supervised_loss_config=recorded), 'combined', config)
        self.assertEqual(SupervisedLossConfig.from_dict({'version': 4}).to_dict(), expected)

    def test_loss5_marker_roundtrip_and_ambiguous_resume_rejected(self):
        config = SupervisedLossConfig(version=5)
        saved = dict(training_loss_version=5, checkpoint_selection='policy',
                     supervised_loss_config=config.to_dict())
        stream = io.BytesIO()
        torch.save(saved, stream)
        stream.seek(0)
        restored = torch.load(stream, weights_only=False)
        self.assertEqual(restored['supervised_loss_config']['categorical_weighting'], 'symmetric_v1')
        self.assertEqual(SupervisedLossConfig.from_dict(restored['supervised_loss_config']), config)
        check_resume_objective(restored, 'policy', config)
        for marker in (None, 'legacy_no_cast', True):
            bad = copy.deepcopy(saved)
            bad['supervised_loss_config']['categorical_weighting'] = marker
            with self.assertRaises(ValueError):
                check_resume_objective(bad, 'policy', config)
        old = copy.deepcopy(saved)
        old['supervised_loss_config'].pop('categorical_weighting')
        with self.assertRaisesRegex(ValueError, 'categorical_weighting'):
            check_resume_objective(old, 'policy', config)
        with self.assertRaisesRegex(ValueError, 'categorical_weighting'):
            SupervisedLossConfig.from_dict(old['supervised_loss_config'])
        # Weight-only transfer does not call the resume check; a new explicit
        # objective is constructible regardless of the parent's old metadata.
        self.assertEqual(SupervisedLossConfig(version=5).categorical_weighting, 'symmetric_v1')


if __name__ == '__main__':
    unittest.main()

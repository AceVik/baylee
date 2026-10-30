"""Native statistics integration for Tensor prediction/advantage outputs."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import torch
from legacy_evaluation import native_metrics


class LegacyMetrics(unittest.TestCase):
    def setUp(self):
        binary_dir = Path(__file__).resolve().parents[2] / 'target/debug'
        self.environment = patch.dict(os.environ, PATH=str(binary_dir) + os.pathsep + os.environ['PATH'])
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def test_torch_probability_proper_scores_and_draws(self):
        prediction = torch.tensor([.25, .75], dtype=torch.float64)
        truth = torch.tensor([0., 1.], dtype=torch.float64)
        result = native_metrics('metrics', prediction=prediction.tolist(), truth=truth.tolist())
        self.assertEqual(result['brier'], .0625)
        self.assertEqual(result['accuracy'], 1.)
        self.assertEqual(result['ece'], .25)
        self.assertIsNone(native_metrics('metrics', prediction=[.5], truth=[.5])['accuracy'])

    def test_value_plan_and_report_preserve_first_decision_game_split(self):
        plan = native_metrics('value-plan', no_deck=True, games=[], rows=dict(
            game=[0, 1, 2, 3, 3], seat=[0, 0, 0, 1, 0], turn=[1, 1, 1, 1, 1],
            result=[2, 0, 0, 2, 0], step=[0, 0, 0, 0, 1]))
        self.assertEqual(plan['indices'], [1, 3])
        with tempfile.TemporaryDirectory() as temporary:
            result = native_metrics('value-report', plan=plan,
                prediction=torch.tensor([.25, .75]).tolist(), model=temporary, data='fixture')
            self.assertEqual(result['report']['all']['n'], 2)
            self.assertEqual(result['report']['all']['net_brier'], .0625)
            self.assertTrue((Path(temporary) / 'eval-fixture.json').is_file())



class LegacyTrainingReportIntegration(unittest.TestCase):
    setUp = LegacyMetrics.setUp
    def test_training_metric_and_group_callbacks_preserve_torch_arrays(self):
        import numpy as np
        from train_value import metrics, report_by
        prediction = torch.full((400,), .75, dtype=torch.float32)
        truth = torch.ones(400, dtype=torch.float32)
        initial = prediction.clone()
        summary = metrics(prediction.numpy(), truth.numpy())
        self.assertEqual(summary['brier'], .0625)
        groups = report_by('by_progress', np.array(['early third'] * 201 + ['last third'] * 199),
                           prediction.numpy(), truth.numpy(), ['early third', 'last third'])
        self.assertEqual(list(groups['by_progress']), ['early third'])
        self.assertEqual(groups['by_progress']['early third']['n'], 201)
        torch.testing.assert_close(prediction, initial, rtol=0, atol=0)

    def test_native_training_report_handles_draw_only_accuracy(self):
        from train_value import markdown
        metric = native_metrics('metrics', prediction=[.5], truth=[.5])
        report = dict(model='fixture', measures='CPU Tensor fixture only', dataset='fixture',
            working_hashes=['fixture'], train_samples=1, held_out_samples=1, held_out_games=1,
            overall={name: metric for name in ('coin', 'logistic (life, hand, library, board, turn)', 'value net')},
            by_progress={}, by_turn={}, by_profile={}, curve=[])
        with tempfile.TemporaryDirectory() as temporary:
            result = native_metrics('training-report', report=report, out=temporary)
            self.assertIn('n/a', result['markdown'])
            self.assertEqual(markdown(report), result['markdown'])
            self.assertTrue((Path(temporary) / 'report.json').is_file())
            self.assertTrue((Path(temporary) / 'report.md').is_file())

    def test_v3_option_tensor_replay_preserves_live_learner_choices(self):
        import numpy as np
        from types import SimpleNamespace
        from eval_shift import log_probs
        batch = SimpleNamespace(idx=np.array([0, 1]), cards=torch.zeros(1))
        class Dataset:
            def col(self, name):
                return np.array([-1, 2]) if name == 'profile' else np.zeros(2, dtype=np.int32)
        class Policy:
            def __call__(self, *unused):
                return torch.tensor([.1, .2, .3, .4]), None
        options = (np.zeros((4, 3), dtype=np.int16), np.array([0, 0, 1, 1]),
                   np.ones(4, dtype=bool), np.array([1, 2]), np.array([2, 2]), np.array([0, 2]))
        with patch('eval_shift.gather_options', return_value=options):
            lp, chosen, usable = log_probs(Policy(), Dataset(), batch, 8, np.array([True, False]))
        torch.testing.assert_close(lp, torch.log_softmax(torch.tensor([[.1, .2], [.3, .4]]), dim=1))
        self.assertEqual(chosen.tolist(), [1, 0])
        self.assertEqual(usable.tolist(), [True, False])



class LegacyPolicyReport(unittest.TestCase):
    setUp = LegacyMetrics.setUp

    def test_native_report_accepts_torch_masks_and_preserves_schema_and_publication(self):
        import json
        import numpy as np
        from train3 import value_report
        class Dataset:
            def col(self, name):
                return np.asarray(dict(step=[0, 0, 1], result=[0, 2, 0], team_rel=[1, 1, 1],
                    winners_rel=[1, 1, 1], seats=[2, 2, 2], turn=[1, 1, 1])[name])
        value = value_report(Dataset(), np.arange(3), torch.tensor([.25, .75, .99]).numpy())
        self.assertEqual(value['all']['n'], 2)
        self.assertAlmostEqual(value['all']['brier'], .0625)
        with tempfile.TemporaryDirectory() as temporary:
            result = native_metrics('policy-report', v3=True, out=temporary,
                hits=torch.tensor([True, False, True]).tolist(),
                usable=torch.tensor([True, True, False]).tolist(), question=[0, 0, 0],
                profile=[0, 0, 0], options=[2, 4, 99], kinds=['choice'], profiles=['house'],
                report=dict(model='fixture', params=7, best_step=1000, held_loss=.1,
                            curve=[], value=value, verification={'preserved': True}))
            report = result['report']
            self.assertEqual(report['agreement'], .5)
            self.assertEqual(report['uniform_chance'], .375)
            self.assertEqual(report['verification'], {'preserved': True})
            self.assertEqual(report['by_kind'], {})
            self.assertEqual(json.loads((Path(temporary)/'report.json').read_text()), report)
            self.assertEqual((Path(temporary)/'report.md').read_text(), result['markdown'])
            self.assertIn('Model: step 1,000.', result['markdown'])


if __name__ == '__main__':
    unittest.main()

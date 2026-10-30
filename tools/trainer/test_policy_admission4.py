"""Only explicitly admitted learner trajectories reach PyTorch target replay."""
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from policy_training4.admission import select_learner_histories, check_audit


class AdmissionSelectionTest(unittest.TestCase):
    def fixture(self, root):
        dataset = SimpleNamespace(fingerprint='bound', trajectories=[[0], [1], [2]],
                                  segments=[[0], [1], [2]], store=SimpleNamespace(index=[
                                      {'game': 4, 'seat': 0}, {'game': 4, 'seat': 1},
                                      {'game': 5, 'seat': 0}]))
        receipt = dict(version=1, admitted=True, source_checkpoint_sha256='source',
                       dataset_fingerprint='bound', collection_contract=dict(mode='raw', actor_role='learner',
                           precision='float32', thinking_steps=2, temperature=.7),
                       admitted_learner_histories=[dict(game_id=4, seat=0)],
                       learner_histories=1, completed_games=1, decisions=3)
        path = Path(root) / 'receipt.json'
        path.write_text(json.dumps(receipt))
        return dataset, receipt, path

    def test_opponents_and_unadmitted_games_removed_before_replay(self):
        with tempfile.TemporaryDirectory() as root:
            dataset, _, path = self.fixture(root)
            receipt = select_learner_histories(dataset, path, 'source')
            self.assertEqual(dataset.trajectories, [[0]])
            check_audit(receipt, dict(decisions_checked=3, learner_trajectories=1))
            with self.assertRaises(ValueError):
                check_audit(receipt, dict(decisions_checked=4, learner_trajectories=1))

    def test_missing_duplicate_and_mismatched_bindings_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            for failure in ('source', 'dataset', 'duplicate', 'missing', 'boolean_seat', 'count'):
                dataset, receipt, path = self.fixture(root)
                if failure == 'source': receipt['source_checkpoint_sha256'] = 'other'
                elif failure == 'dataset': dataset.fingerprint = 'other'
                elif failure == 'duplicate': dataset.trajectories.append([0])
                elif failure == 'missing': dataset.trajectories = [[1]]
                elif failure == 'boolean_seat': receipt['admitted_learner_histories'][0]['seat'] = False
                else: receipt['completed_games'] = 2
                path.write_text(json.dumps(receipt))
                with self.subTest(failure=failure), self.assertRaises(ValueError):
                    select_learner_histories(dataset, path, 'source')

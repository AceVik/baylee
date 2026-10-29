"""Counterfactual label admission cannot become an executed-policy claim."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import torch

from trajectory_store4 import reject_shadow_policy_gradient, validate_bc_dataset as validate_shadow_dataset
from policy_training4.targets import prepare_policy_targets
from ppo_targets4 import prepare_targets
from test_policy_targets4 import Dataset, Frozen
from trajectory_store4 import TrajectoryStore4


def fixture(root):
    source = dict(converted_game_id=1, converted_labels='shadow-bc/game-000001.jsonl',
                  source_namespace='a'*64, source_game='seed17-seat0', run_sha256='a'*64,
                  labels_sha256='b'*64, record_sha256='c'*64)
    header = dict(kind='header', schema=1, converted_game_id=1,
                  source_namespace='a'*64, source_game='seed17-seat0', run_sha256='a'*64,
                  original_labels_sha256='b'*64, record_sha256='c'*64, label_use='bc_only', ppo_eligible=False)
    rows = [header, dict(kind='label', seat=0, step_cursor=2, target={'single_idx':1}),
            dict(kind='completed', labels=1, ppo_eligible=False)]
    path = root / source['converted_labels']
    path.parent.mkdir(parents=True)
    path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
    meta = dict(shadow_bc_version=1, shadow_bc_contract='verified_view_only_teacher_v1',
                shadow_bc_status='complete', ppo_eligible=False, policy_provenance_version=None,
                encoder_version=7, trajectory_version=2, shadow_bc_sources=[source],
                shadow_bc_label_sha256={source['converted_labels']:hashlib.sha256(path.read_bytes()).hexdigest()})
    return meta, path, rows


class ShadowDatasets(unittest.TestCase):
    def test_complete_binding_and_ordinary_dataset_compatibility(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            meta, _, _ = fixture(root)
            validate_shadow_dataset(root, meta)
            validate_shadow_dataset(root, {'encoder_version':7})
            (root/'meta.json').write_text(json.dumps(meta))
            # Admission happens even when a prebuilt/missing shard cache exists.
            cache = TrajectoryStore4(root, 'train', [])
            self.assertEqual(len(cache.index), 0)
            cache.close()

    def test_tamper_is_rejected_before_existing_cache_access(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            meta, path, _ = fixture(root)
            (root/'meta.json').write_text(json.dumps(meta))
            TrajectoryStore4(root, 'train', []).close()
            path.write_text(path.read_text().replace('"single_idx": 1', '"single_idx": 0'))
            with self.assertRaisesRegex(ValueError, 'digest mismatch'):
                TrajectoryStore4(root, 'train', [])

    def test_status_namespace_duplicates_and_footer_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            meta, path, rows = fixture(root)
            mutations = [dict(shadow_bc_status='in_progress'), dict(ppo_eligible=True),
                         dict(policy_provenance_version=2), dict(trajectory_version=1),
                         dict(shadow_bc_sources=meta['shadow_bc_sources']*2)]
            for change in mutations:
                with self.subTest(change=change), self.assertRaises(ValueError):
                    validate_shadow_dataset(root, {**meta, **change})
            for changed in [rows[:-1], [rows[0], rows[1], rows[1], rows[2]],
                            [{**rows[0], 'source_namespace':'d'*64}, *rows[1:]]]:
                path.write_text(''.join(json.dumps(row)+'\n' for row in changed))
                altered = copy.deepcopy(meta)
                altered['shadow_bc_label_sha256'][str(path.relative_to(root))] = hashlib.sha256(path.read_bytes()).hexdigest()
                with self.assertRaises(ValueError):
                    validate_shadow_dataset(root, altered)

    def test_both_ppo_paths_reject_before_model_or_cache_use(self):
        for marker in ({'shadow_bc_version':1}, {'ppo_eligible':False}):
            dataset, model = Dataset(), Frozen()
            dataset.meta.update(marker)
            with self.assertRaisesRegex(ValueError, 'BC-only'):
                prepare_policy_targets(model, dataset, torch.device('cpu'))
            with self.assertRaisesRegex(ValueError, 'BC-only'):
                prepare_targets(model, dataset, torch.device('cpu'), 2, .99, .95, None, None)
            self.assertEqual(model.calls, [])
            self.assertFalse(hasattr(dataset, 'ppo_cache'))
        reject_shadow_policy_gradient({'policy_provenance_version':2})


if __name__ == '__main__':
    unittest.main()

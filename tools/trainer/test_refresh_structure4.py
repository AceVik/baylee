"""Catalog refresh preserves model family and learned weights without hiding new tags."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import torch
import model4 as V4
import model5 as V5
from export_onnx4 import load_export_checkpoint
from refresh_structure4 import refresh_checkpoint
from test_structure4 import small_forest


class RefreshCheckpoint(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(813)

    def source(self, path, revision):
        module = V4 if revision is None else V5
        options = dict(d_model=16, n_layers=1, n_heads=2, d_ff=32, structure_width=16)
        cfg = V4.Config4(**options) if revision is None else V5.Config5(**options, relational_revision=revision)
        net = V4.PlayerNet4(cfg) if revision is None else V5.PlayerNet5(cfg)
        forest = small_forest()
        net.attach_structure(forest)
        payload = dict(config=cfg.to_dict(), model_version=module.MODEL_VERSION,
            encoder_version=7, trajectory_version=2, sampling_version=2, state_dict=net.state_dict(),
            structure=forest, step_num=317, training_loss_version=4, checkpoint_selection='combined',
            val_metrics={'val_pol_loss': .2}, provenance={'kind': 'trusted-training'},
            optimizer_schedule={'kind': 'test-preserved'}, optimizer={'stale': True},
            rng={'stale': True}, h=torch.zeros(1))
        if revision is not None:
            payload['architecture'] = V5.architecture_for_config(cfg)
        path.mkdir()
        torch.save(payload, path / 'net.pt')
        (path / 'meta.json').write_text(json.dumps({'custom_training_evidence': {'epochs': 2},
                                                   'step_num': 317, 'val_metrics': payload['val_metrics']}))
        return net, payload

    def test_all_families_exact_shared_mapping_and_metadata(self):
        for revision in (None, 1, 2):
            with self.subTest(revision=revision), tempfile.TemporaryDirectory() as root:
                root = Path(root)
                net, source = self.source(root / 'source', revision)
                original_bytes = (root / 'source/net.pt').read_bytes()
                data = copy.deepcopy(source['structure'])
                data['tags'].reverse()
                for node in data['nodes']:
                    node['tag'] = len(data['tags']) - 1 - node['tag']
                catalog = root / 'catalog.json'
                catalog.write_text(json.dumps(data))
                lineage = refresh_checkpoint(root / 'source', catalog, root / 'new')
                saved, refreshed, _ = load_export_checkpoint(root / 'new/net.pt')
                self.assertEqual(saved['config'], source['config'])
                self.assertEqual(saved['catalog_refresh']['discarded_runtime_state'], ['optimizer', 'rng', 'h'])
                self.assertFalse(any(key in saved for key in ('optimizer', 'rng', 'h')))
                self.assertEqual(saved['structure_sha256'], refreshed.rules.digest)
                for key in ('step_num', 'training_loss_version', 'checkpoint_selection', 'val_metrics',
                            'provenance', 'optimizer_schedule', 'architecture'):
                    self.assertEqual(saved.get(key), source.get(key))
                for name, tensor in source['state_dict'].items():
                    if not name.startswith('rules.'):
                        torch.testing.assert_close(saved['state_dict'][name], tensor, rtol=0, atol=0)
                with torch.no_grad():
                    torch.testing.assert_close(net.rules.encode_catalog(), refreshed.rules.encode_catalog(),
                                               rtol=0, atol=0)
                metadata = json.loads((root / 'new/meta.json').read_text())
                self.assertEqual(metadata['custom_training_evidence'], {'epochs': 2})
                self.assertEqual(metadata['step_num'], 317)
                self.assertEqual(metadata['val_metrics'], source['val_metrics'])
                self.assertEqual(metadata['catalog_refresh']['initialized_new_tags'], [])
                self.assertEqual(lineage['operator_mapping'], 'exact tag names')
                self.assertEqual(lineage['source_sha256'], hashlib.sha256(original_bytes).hexdigest())
                self.assertEqual((root / 'source/net.pt').read_bytes(), original_bytes)
                with self.assertRaisesRegex(ValueError, 'existing output'):
                    refresh_checkpoint(root / 'source', catalog, root / 'new')

    def test_unknown_operators_reject_or_record_actual_donor_initialization(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            net, source = self.source(root / 'source', 2)
            data = copy.deepcopy(source['structure'])
            data['tags'].append('Creature.NewField')
            data['nodes'].append({'tag': len(data['tags']) - 1, 'bytes': [], 'children': []})
            catalog = root / 'catalog.json'
            catalog.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'require training'):
                refresh_checkpoint(root / 'source', catalog, root / 'reject')
            self.assertFalse((root / 'reject').exists())
            lineage = refresh_checkpoint(root / 'source', catalog, root / 'fine-tune', allow_new_operators=True)
            self.assertEqual(lineage['initialized_new_tags'], ['Creature.NewField'])
            self.assertEqual(lineage['new_tag_donors'], {'Creature.NewField': ['Creature']})
            self.assertTrue(lineage['requires_new_operator_training'])
            self.assertIn('donor-initialized', lineage['operator_mapping'])
            _, refreshed, _ = load_export_checkpoint(root / 'fine-tune/net.pt')
            old_idx = source['structure']['tags'].index('Creature')
            torch.testing.assert_close(refreshed.rules.symbol.weight[-1], net.rules.symbol.weight[old_idx],
                                       rtol=0, atol=0)

    def test_wrong_v5_architecture_is_rejected_before_output(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            _, source = self.source(root / 'source', 2)
            source['architecture'] = V5.ARCHITECTURES[1]
            torch.save(source, root / 'source/net.pt')
            catalog = root / 'catalog.json'
            catalog.write_text(json.dumps(source['structure']))
            with self.assertRaisesRegex(ValueError, 'architecture'):
                refresh_checkpoint(root / 'source', catalog, root / 'reject')
            self.assertFalse((root / 'reject').exists())


if __name__ == '__main__':
    unittest.main()

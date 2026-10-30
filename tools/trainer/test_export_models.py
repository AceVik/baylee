"""CPU ONNX parity for both model families, including learned relation messages."""
import copy
import hashlib
import json
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch

import model4 as M
from export_onnx4 import (export_v4_onnx, load_export_checkpoint, make_dummy_inputs, main,
                          export_critic_semantics)

FOREST = {"version": 1, "dsl_source_sha256": "synthetic", "tags": ["Unknown"],
          "nodes": [{"tag": 0, "bytes": [], "children": []}], "roots": {}}


class ExportModels(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(711)

    def checkpoint(self, module, config, model):
        result = dict(model_version=module.MODEL_VERSION, encoder_version=module.ENCODER_VERSION,
                      trajectory_version=module.TRAJECTORY_VERSION, sampling_version=module.SAMPLING_VERSION,
                      config=config.to_dict(), structure=FOREST, state_dict=model.state_dict())
        if module.MODEL_VERSION == 5:
            result['architecture'] = module.architecture_for_config(config)
        return result

    def research_checkpoint(self, source):
        from research_ppo4.checkpoint import VERSION, MEMORY, PROBABILITY
        from research_ppo4.targets import OBJECTIVE, TARGET_CONTRACT
        return {**source, 'memory_reconstruction': MEMORY,
                'research_ppo_contract': dict(version=VERSION, objective=OBJECTIVE,
                    target_contract=TARGET_CONTRACT, behavior_probability_reference=PROBABILITY)}

    def test_export_critic_semantics_validates_research_before_model_construction(self):
        ordinary = dict(kind='mtg_side_outcome_utility_v1')
        self.assertEqual(export_critic_semantics({}), ordinary)
        self.assertEqual(export_critic_semantics(dict(critic_semantics=ordinary)), ordinary)
        research = self.research_checkpoint({})
        expected = dict(kind='failure_terminal_research_utility_v1',
                        objective=research['research_ppo_contract']['objective'],
                        target_contract=research['research_ppo_contract']['target_contract'])
        self.assertEqual(export_critic_semantics(research), expected)
        self.assertEqual(export_critic_semantics({**research, 'critic_semantics': expected}), expected)
        broken = [{**research, 'research_ppo_contract': value}
                  for value in (None, [], {}, 'unknown')]
        for key in ('version', 'objective', 'target_contract', 'behavior_probability_reference'):
            value = copy.deepcopy(research)
            value['research_ppo_contract'][key] = 'unknown'
            broken.append(value)
        broken.extend([{**research, 'memory_reconstruction': None},
                       {**research, 'full_ppo_contract': {}},
                       {**research, 'critic_semantics': ordinary},
                       {'critic_semantics': expected},
                       {'research_ppo_progress': {}},
                       {'research_ppo_progress': None},
                       {'critic_semantics': None},
                       {'critic_semantics': {'kind': 'unknown'}}])
        # Missing architecture/state is intentional: critic validation must reject
        # malformed objectives before constructing a model or exporting any graph.
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'net.pt'
            for checkpoint in broken:
                with self.subTest(checkpoint=checkpoint):
                    torch.save(checkpoint, path)
                    with patch('export_onnx4.export_v4_onnx') as export:
                        with self.assertRaisesRegex(ValueError, 'Research PPO|critic_semantics'):
                            load_export_checkpoint(path)
                        export.assert_not_called()

    def test_loader_rejects_wrong_contracts_and_state_shapes(self):
        cfg = M.Config4(d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32)
        net = M.PlayerNet4(cfg)
        net.attach_structure(FOREST)
        good = self.checkpoint(M, cfg, net)
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'net.pt'
            torch.save(good, path)
            _, loaded, module = load_export_checkpoint(path)
            self.assertIs(module, M)
            self.assertEqual(type(loaded), M.PlayerNet4)
            for key, value in [('model_version', 999), ('encoder_version', 6),
                               ('sampling_version', 999), ('trajectory_version', 999)]:
                torch.save({**good, key: value}, path)
                with self.assertRaises(ValueError):
                    load_export_checkpoint(path)
            broken = copy.deepcopy(good)
            broken['state_dict'].pop(next(iter(broken['state_dict'])))
            torch.save(broken, path)
            with self.assertRaises(RuntimeError):
                load_export_checkpoint(path)

    def test_v5_architecture_is_mandatory(self):
        import model5 as V5
        cfg = V5.Config5(d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32)
        net = V5.PlayerNet5(cfg)
        net.attach_structure(FOREST)
        good = self.checkpoint(V5, cfg, net)
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'net.pt'
            torch.save(good, path)
            _, loaded, module = load_export_checkpoint(path)
            self.assertIs(module, V5)
            self.assertIsInstance(loaded, V5.PlayerNet5)
            for architecture in (None, 'playernet4'):
                torch.save({**good, 'architecture': architecture}, path)
                with self.assertRaisesRegex(ValueError, 'architecture'):
                    load_export_checkpoint(path)

    def test_v5_v2_cli_exports_explicit_identity_and_all_depths(self):
        import model5 as V5
        cfg = V5.Config5(relational_revision=2, d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32)
        net = V5.PlayerNet5(cfg)
        net.attach_structure(FOREST)
        with tempfile.TemporaryDirectory() as root:
            path = Path(root)
            torch.save(self.checkpoint(V5, cfg, net), path / 'net.pt')
            digest = hashlib.sha256((path / 'net.pt').read_bytes()).hexdigest()
            with patch('sys.argv', ['export_onnx4.py', '--model', root, '--check', '0']):
                main()
            metadata = json.loads((path / 'model.onnx.json').read_text())
            self.assertEqual(metadata['model_version'], 5)
            self.assertEqual(metadata['architecture'], V5.ARCHITECTURES[2])
            self.assertEqual(metadata['source_checkpoint_sha256'], digest)
            self.assertEqual(metadata['config'], cfg.to_dict())
            self.assertEqual(metadata['critic_semantics'], {'kind': 'mtg_side_outcome_utility_v1'})
            self.assertEqual(set(metadata['graphs']), {'0', '1', '2', '4', '8'})
            for graph in metadata['graphs'].values():
                self.assertTrue((path / graph).is_file())
            self.assertEqual((path / 'model.onnx').read_bytes(), (path / 'model.k2.onnx').read_bytes())
            self.assertLessEqual(metadata['parity_max_abs_diff'], 1e-3)

    def test_research_cli_export_retains_utility_semantics_without_large_contract(self):
        import model5 as V5
        cfg = V5.Config5(relational_revision=2, d_model=16, structure_width=16,
                         n_heads=2, n_layers=1, d_ff=32)
        net = V5.PlayerNet5(cfg)
        net.attach_structure(FOREST)
        checkpoint = self.research_checkpoint(self.checkpoint(V5, cfg, net))
        expected = export_critic_semantics(checkpoint)
        with tempfile.TemporaryDirectory() as root:
            path = Path(root)
            torch.save(checkpoint, path / 'net.pt')
            with patch('sys.argv', ['export_onnx4.py', '--model', root, '--check', '0']):
                main()
            metadata = json.loads((path / 'model.onnx.json').read_text())
            self.assertEqual(metadata['critic_semantics'], expected)
            self.assertNotIn('research_ppo_contract', metadata)
            self.assertEqual(set(metadata['graphs']), {'0', '1', '2', '4', '8'})
            self.assertLessEqual(metadata['parity_max_abs_diff'], 1e-3)

    def test_v4_all_depths_and_dynamic_graph_shapes(self):
        self.assert_parity(M, M.Config4, M.PlayerNet4)

    def test_v5_all_depths_and_dynamic_graph_shapes(self):
        import model5 as V5
        self.assert_parity(V5, V5.Config5, V5.PlayerNet5)

    def test_v5_v2_all_depths_and_dynamic_graph_shapes(self):
        import model5 as V5
        self.assert_parity(V5, V5.Config5, V5.PlayerNet5, relational_revision=2)

    def test_v5_architecture_revision_and_legacy_config_must_agree(self):
        import model5 as V5
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'net.pt'
            for revision in (1, 2):
                cfg = V5.Config5(relational_revision=revision, d_model=16, structure_width=16,
                                 n_heads=2, n_layers=1, d_ff=32)
                net = V5.PlayerNet5(cfg)
                net.attach_structure(FOREST)
                good = self.checkpoint(V5, cfg, net)
                torch.save(good, path)
                _, loaded, _ = load_export_checkpoint(path)
                self.assertEqual(loaded.cfg.relational_revision, revision)
                bad = copy.deepcopy(good)
                bad['architecture'] = V5.ARCHITECTURES[3 - revision]
                torch.save(bad, path)
                with self.assertRaisesRegex(ValueError, 'architecture'):
                    load_export_checkpoint(path)
                legacy = copy.deepcopy(good)
                legacy['config'].pop('relational_revision')
                torch.save(legacy, path)
                if revision == 1:
                    _, loaded, _ = load_export_checkpoint(path)
                    self.assertEqual(loaded.cfg.relational_revision, 1)
                else:
                    with self.assertRaisesRegex(ValueError, 'explicit'):
                        load_export_checkpoint(path)

    def assert_parity(self, module, config_type, model_type, **config_overrides):
        cfg = config_type(d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32, **config_overrides)
        net = model_type(cfg).eval()
        net.attach_structure(FOREST)
        # Zero initialization would hide a broken ONNX message-passing branch.
        # Exercise every learned parameter, including the residual output.
        if module.MODEL_VERSION == 5:
            base_keys = M.PlayerNet4(M.Config4(d_model=16, structure_width=16,
                                              n_heads=2, n_layers=1, d_ff=32)).state_dict()
            with torch.no_grad():
                for name, parameter in net.named_parameters():
                    if name not in base_keys:
                        parameter.uniform_(-.1, .1)
        net.rules = net.rules.freeze()
        options = ort.SessionOptions()
        options.intra_op_num_threads = 1
        with tempfile.TemporaryDirectory() as root:
            for depth in (0, 1, 2, 4, 8):
                path = Path(root) / f'k{depth}.onnx'
                export_v4_onnx(net, path, thinking_steps=depth, fixed_depth=True)
                session = ort.InferenceSession(str(path), sess_options=options,
                                               providers=['CPUExecutionProvider'])
                for entities, edge_case in ((1, 'empty'), (3, 'duplicate'), (7, 'mixed')):
                    edges = 0 if edge_case == 'empty' else 4
                    inputs = list(make_dummy_inputs(16, B=2 if edge_case == 'mixed' else 1, E=entities, M_edges=edges, C=3, P=2, K=depth))
                    if edges:
                        inputs[5][:, :, 0] = 9  # first entity, after global+8 seats
                        inputs[5][:, :, 1] = 9 + entities - 1
                        inputs[6][:] = 2
                        if edge_case == 'mixed':
                            inputs[5][:, -1, :] = 0
                            inputs[7][:, -1] = False
                    inputs[0][..., 122] = torch.arange(1, entities + 1).float()
                    inputs[-2].normal_(std=.1)
                    with torch.inference_mode():
                        expected = net(*inputs)
                    actual = session.run(list(M.OUTPUT_NAMES),
                                         {name: tensor.numpy() for name, tensor in zip(M.INPUT_NAMES, inputs)})
                    for name, want, got in zip(M.OUTPUT_NAMES, expected, actual):
                        with self.subTest(version=module.MODEL_VERSION, depth=depth,
                                          entities=entities, edges=edge_case, output=name):
                            self.assertTrue(np.isfinite(got).all())
                            np.testing.assert_allclose(got, want.numpy(), rtol=2e-4, atol=2e-5)
                    if getattr(cfg, 'relational_revision', None) == 2:
                        # Relabel entity rows and edge order while keeping all
                        # action references attached to the same physical objects.
                        permutation = torch.arange(entities - 1, -1, -1)
                        inverse = torch.empty_like(permutation)
                        inverse[permutation] = torch.arange(entities)
                        remapped = [tensor.clone() for tensor in inputs]
                        for index in (0, 1, 2, 15):
                            remapped[index] = remapped[index][:, permutation]
                        remapped[5] = torch.where(remapped[5] >= 9,
                            inverse[(remapped[5] - 9).clamp(min=0)] + 9, remapped[5])
                        for index in (5, 6, 7):
                            remapped[index] = remapped[index].flip(1)
                        remapped[9] = torch.where(remapped[9] >= 0,
                            inverse[remapped[9].clamp(min=0)], remapped[9])
                        remapped[12] = inverse[remapped[12]]
                        with torch.inference_mode():
                            invariant = net(*remapped)
                        ort_invariant = session.run(list(M.OUTPUT_NAMES),
                            {name: tensor.numpy() for name, tensor in zip(M.INPUT_NAMES, remapped)})
                        for name, base, eager, got in zip(M.OUTPUT_NAMES, actual, invariant, ort_invariant):
                            with self.subTest(depth=depth, edges=edge_case, invariant=name):
                                np.testing.assert_allclose(got, eager.numpy(), rtol=2e-4, atol=2e-5)
                                np.testing.assert_allclose(got, base, rtol=2e-4, atol=2e-5)



if __name__ == '__main__':
    unittest.main()

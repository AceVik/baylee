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
from export_onnx4 import export_v4_onnx, load_export_checkpoint, make_dummy_inputs, main

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
            result['architecture'] = module.ARCHITECTURE
        return result

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

    def test_v5_cli_exports_explicit_identity_and_all_depths(self):
        import model5 as V5
        cfg = V5.Config5(d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32)
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
            self.assertEqual(metadata['architecture'], V5.ARCHITECTURE)
            self.assertEqual(metadata['source_checkpoint_sha256'], digest)
            self.assertEqual(metadata['config'], cfg.to_dict())
            self.assertEqual(set(metadata['graphs']), {'0', '1', '2', '4', '8'})
            for graph in metadata['graphs'].values():
                self.assertTrue((path / graph).is_file())
            self.assertEqual((path / 'model.onnx').read_bytes(), (path / 'model.k2.onnx').read_bytes())
            self.assertLessEqual(metadata['parity_max_abs_diff'], 1e-3)

    def test_v4_all_depths_and_dynamic_graph_shapes(self):
        self.assert_parity(M, M.Config4, M.PlayerNet4)

    def test_v5_all_depths_and_dynamic_graph_shapes(self):
        import model5 as V5
        self.assert_parity(V5, V5.Config5, V5.PlayerNet5)

    def assert_parity(self, module, config_type, model_type):
        cfg = config_type(d_model=16, structure_width=16, n_heads=2, n_layers=1, d_ff=32)
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


if __name__ == '__main__':
    unittest.main()

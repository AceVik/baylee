"""Depth growth must preserve recurrent behavior and admit useful gradients."""
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch

import torch

from deepen4 import deepen_checkpoint, deepen_network
from export_onnx4 import make_dummy_inputs
import model4 as M


class DepthTransfer(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(71)
        self.source = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=2, d_ff=64))

    def test_cpu_growth_does_not_reseed_accelerators(self):
        with patch("torch.cuda.manual_seed_all") as cuda_seed:
            deepen_network(self.source, 3)
        cuda_seed.assert_not_called()

    def test_memory_policy_value_and_aux_preserved_across_observations(self):
        before = torch.get_rng_state().clone()
        deeper, report = deepen_network(self.source, 3)
        torch.testing.assert_close(before, torch.get_rng_state(), rtol=0, atol=0)
        self.assertEqual(report["copied_layer_map"], {0: 0, 1: 1})
        self.assertEqual(report["inserted_layers"], [2])
        self.assertGreater(report["target_parameters"], report["source_parameters"])
        left_h = right_h = torch.randn(2, 9, 32)
        with torch.no_grad():
            for entity_count, depth in ((5, 0), (7, 1), (9, 0), (3, 4), (6, 8)):
                args = list(make_dummy_inputs(32, B=2, E=entity_count, M_edges=3, C=7, P=4, K=depth))
                args[2][1, -1] = False
                args[11][1, -1] = False
                left = self.source(*args[:-2], left_h, args[-1])
                right = deeper(*args[:-2], right_h, args[-1])
                for a, b in zip(left, right):
                    torch.testing.assert_close(a, b, atol=1e-6, rtol=1e-6)
                torch.testing.assert_close(self.source.aux_head(left[0][:, 0]), deeper.aux_head(right[0][:, 0]))
                left_h, right_h = left[0], right[0]
        # Independence: further training must not mutate the source checkpoint.
        with torch.no_grad():
            deeper.trunk_layers[0].attn.q_proj.weight.add_(1)
        self.assertFalse(torch.equal(deeper.trunk_layers[0].attn.q_proj.weight,
                                     self.source.trunk_layers[0].attn.q_proj.weight))

    def test_inserted_blocks_learn_and_unblock_internal_gradients(self):
        deeper, report = deepen_network(self.source, 4)
        self.assertEqual(report["copied_layer_map"], {0: 0, 1: 2})
        self.assertEqual(report["inserted_layers"], [1, 3])
        deeper.cfg.activation_checkpointing = True
        optimizer = torch.optim.SGD(deeper.parameters(), lr=.01)
        args = list(make_dummy_inputs(32, B=2, E=5, M_edges=2, C=7, P=3, K=2))
        args[-2] = torch.randn(2, 9, 32)
        for step in range(2):
            optimizer.zero_grad(set_to_none=True)
            output = deeper(*args)
            loss = output[1].square().mean() + output[4].square().mean()
            loss.backward()
            for index in report["inserted_layers"]:
                layer = deeper.trunk_layers[index]
                for projection in (layer.attn.out_proj, layer.ff[3]):
                    self.assertTrue(torch.isfinite(projection.weight.grad).all())
                    self.assertGreater(float(projection.weight.grad.abs().sum()), 0)
                if step:
                    self.assertGreater(float(layer.attn.q_proj.weight.grad.abs().sum()), 0)
                    self.assertGreater(float(layer.ff[0].weight.grad.abs().sum()), 0)
            optimizer.step()

    def test_rejects_shrinking_dropout_and_preserves_outputs_on_cli_reuse(self):
        for depth in (0, 1, 2):
            with self.assertRaises(ValueError):
                deepen_network(self.source, depth)
        self.source.cfg.dropout = .1
        with self.assertRaises(ValueError):
            deepen_network(self.source, 3)
        self.source.cfg.dropout = 0
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source.pt"
            payload = {"config": self.source.cfg.to_dict(), "structure": self.source.rules.data,
                       "state_dict": self.source.state_dict(), "encoder_version": M.ENCODER_VERSION,
                       "model_version": M.MODEL_VERSION, "trajectory_version": M.TRAJECTORY_VERSION,
                       "sampling_version": M.SAMPLING_VERSION, "optimizer": {"must": "not transfer"},
                       "h": torch.randn(1, 9, 32), "val_metrics": {"stale": 1}}
            torch.save(payload, source)
            out = Path(tmp) / "deeper"
            deepen_checkpoint(source, out, 3)
            saved = torch.load(out / "net.pt", weights_only=False)
            for key in ("optimizer", "h", "val_metrics", "dataset_fingerprint", "step_num"):
                self.assertNotIn(key, saved)
            self.assertEqual(saved["config"]["n_layers"], 3)
            with self.assertRaises(FileExistsError):
                deepen_checkpoint(source, out, 4)
            payload["model_version"] = -1
            torch.save(payload, source)
            with self.assertRaises(ValueError):
                deepen_checkpoint(source, Path(tmp) / "bad", 3)
            self.assertFalse((Path(tmp) / "bad").exists())


if __name__ == "__main__":
    unittest.main()

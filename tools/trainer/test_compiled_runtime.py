"""Opt-in CUDA upgrade check for the real regional compiler and recurrent model.

Synthetic FP32/BF16 parity is a compatibility check, not a throughput or
playing-strength measurement. Run under the shared GPU exclusion lock.
"""
import copy
import os
import unittest

import torch

import model5 as M
from export_onnx4 import make_dummy_inputs
from training4.execution import compile_regions


@unittest.skipUnless(
    os.environ.get("BAYLEE_TEST_COMPILED_CUDA") == "1",
    "exclusive GPU slot and compiler warmup required",
)
class CompiledRuntime(unittest.TestCase):
    def test_dynamic_recurrence_backward_and_optimizer(self):
        self.assertTrue(torch.cuda.is_available(), "requested CUDA check needs a GPU")
        torch.set_num_threads(1)
        previous_tf32 = torch.backends.cuda.matmul.allow_tf32
        try:
            torch.backends.cuda.matmul.allow_tf32 = False
            for bf16 in (False, True):
                with self.subTest(bf16=bf16):
                    self.assert_compiled_parity(bf16)
        finally:
            torch.backends.cuda.matmul.allow_tf32 = previous_tf32
            torch.compiler.reset()

    def assert_compiled_parity(self, bf16):
        torch.manual_seed(215)
        cfg = M.Config5(
            d_model=32, structure_width=32, n_heads=4, n_layers=1,
            d_ff=64, relational_width=8, relational_revision=2,
            candidate_context_revision=2, activation_checkpointing=True,
        )
        reference = M.PlayerNet5(cfg).cuda().train()
        # Exercise the learned relation output, not just its identity warm start.
        with torch.no_grad():
            for name, parameter in reference.named_parameters():
                if name.startswith("relational"):
                    parameter.uniform_(-.1, .1)
        compiled = copy.deepcopy(reference)
        compile_regions(compiled, readouts=True)
        self.assertEqual(reference.state_dict().keys(), compiled.state_dict().keys())
        frames = [
            tuple(t.cuda() for t in make_dummy_inputs(
                32, B=2, E=entities, M_edges=3, C=candidates, P=pairs, K=depth,
            ))
            for entities, candidates, pairs, depth in ((5, 4, 3, 0), (7, 6, 4, 2))
        ]
        observations = []
        for net in (reference, compiled):
            # SGD keeps the update comparison proportional to gradient error.
            # AdamW amplifies a ~1e-8 roundoff gradient on the softmax-invariant
            # shared value bias into a visible first update, including on 2.14.
            # Actual AdamW placement and exact resume have separate CUDA tests.
            optimizer = torch.optim.SGD(net.parameters(), lr=1e-4, momentum=.9)
            h = torch.zeros(2, 9, 32, device="cuda")
            loss = torch.zeros((), device="cuda")
            with torch.autocast("cuda", dtype=torch.bfloat16, enabled=bf16):
                for args in frames:
                    h, candidates, pairs, scalar, value = net(*args[:-2], h, args[-1])
                    loss = (loss + h.float().square().mean()
                            + candidates.masked_fill(~args[11], 0).float().square().mean()
                            + pairs.masked_fill(~args[13], 0).float().square().mean()
                            + scalar.float().square().mean() + value.float().square().mean())
            self.assertTrue(bool(torch.isfinite(loss)))
            loss.backward()
            gradients = {name: p.grad.detach().clone() for name, p in net.named_parameters()
                         if p.grad is not None}
            for prefix in ("relational", "trunk_layers", "recurrent_cell", "thinking_block"):
                parts = [g.abs().sum() for name, g in gradients.items() if name.startswith(prefix)]
                self.assertTrue(parts, prefix)
                self.assertGreater(float(torch.stack(parts).sum()), 0, prefix)
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1, error_if_nonfinite=True)
            optimizer.step()
            observations.append((loss.detach(), h.detach(), gradients))
        atol, rtol = (2e-3, 2e-2) if bf16 else (3e-6, 3e-5)
        for actual, expected in zip(observations[1][:2], observations[0][:2]):
            torch.testing.assert_close(actual, expected, atol=atol, rtol=rtol)
        self.assertEqual(observations[0][2].keys(), observations[1][2].keys())
        for name, expected in observations[0][2].items():
            torch.testing.assert_close(observations[1][2][name], expected,
                                       atol=atol, rtol=rtol, msg=name)
        for (name, actual), (_, expected) in zip(compiled.named_parameters(), reference.named_parameters()):
            torch.testing.assert_close(actual, expected, atol=atol, rtol=rtol, msg=name)


if __name__ == "__main__":
    unittest.main()

"""Compiled trunk layers under activation checkpointing, as stage B runs them.

Stage B (`--trainable-trunk-from-layer L`) freezes trunk layers below L and
trains the rest. Every trunk layer shares one compiled code object
(`PreNormTransformerLayer.forward`), so frozen and trainable layers, frozen
inputs and differentiable ones, and every board shape all compete in one
Dynamo cache. A checkpointed layer is recomputed in backward; the recompute
must run the same compiled graph as its forward, or `torch.utils.checkpoint`
raises a metadata mismatch.

The compile tests run Inductor on the CPU (a C++ compiler is needed) and take
minutes, so they run only with BAYLEE_COMPILE_TESTS=1.
"""
import os
import unittest
from unittest.mock import patch

import torch

from model4.activation_checkpoint import checkpoint_activation
from model4.layers import PreNormTransformerLayer

COMPILE = os.environ.get("BAYLEE_COMPILE_TESTS") == "1"


def stage_b_trunk(layers=4, trainable_from=2, seed=3):
    torch.manual_seed(seed)
    trunk = torch.nn.ModuleList(PreNormTransformerLayer(32, 4, 64) for _ in range(layers))
    for index, layer in enumerate(trunk):
        layer.requires_grad_(index >= trainable_from)
    return trunk


def encode(trunk, x, pad, saved_layers):
    """`PlayerNet.encode_board`'s layer loop: checkpoint all but the last `saved_layers`."""
    for index, layer in enumerate(trunk):
        if torch.is_grad_enabled() and index < len(trunk) - saved_layers:
            x = checkpoint_activation(layer, x, pad)
        else:
            x = layer(x, src_key_padding_mask=pad)
    return x


def chunks(entities, lanes=8, frames=32, budget=32768):
    """`encode_boards`' chunks of one window batch: rows (lanes x frames) and tokens (9 + entities)."""
    tokens = 9 + entities
    chunk = max(1, min(frames, budget // (lanes * tokens)))
    return [(lanes * min(chunk, frames - start), tokens) for start in range(0, frames, chunk)]


# Round 0's boards: up to 127 entities; above 119 the token budget splits a batch in two chunks.
SHAPES = [chunks(e) for e in (40, 63, 127, 22, 88, 121, 57, 31, 100, 127, 70, 45)]


def train_steps(trunk, saved_layers, steps=24, seed=0):
    """Several chunks forward, then one backward, as one training step does."""
    generator = torch.Generator().manual_seed(seed)
    grads = []
    for step in range(steps):
        total = 0
        for rows, tokens in SHAPES[step % len(SHAPES)]:
            x = torch.randn(rows, tokens, 32, generator=generator)   # frozen embeddings: no grad
            pad = torch.zeros(rows, tokens, dtype=torch.bool)
            pad[:, tokens - tokens // 4:] = True
            with torch.autocast("cpu", dtype=torch.bfloat16):
                out = encode(trunk, x, pad, saved_layers)
            total = total + out.float().pow(2).mean()
        total.backward()
        grads.append(torch.cat([p.grad.flatten() for p in trunk.parameters() if p.requires_grad]).clone())
        trunk.zero_grad()
    return grads


class StageBCheckpoints(unittest.TestCase):
    def test_saved_upper_layers_leave_only_frozen_layers_checkpointed(self):
        """`--saved-activation-layers` = the trainable layers: no trainable layer is
        checkpointed, so no compiled layer is ever recomputed in backward."""
        import model4 as M
        from export_onnx4 import make_dummy_inputs
        torch.manual_seed(1)
        net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=4, d_ff=64, activation_checkpointing=True))
        for index, layer in enumerate(net.trunk_layers):
            layer.requires_grad_(index >= 2)
        inputs = make_dummy_inputs(32, E=5, M_edges=3, C=7, P=4, K=1)
        for saved, expected in ((0, [0, 1, 2, 3]), (2, [0, 1])):
            net.saved_activation_layers = saved
            wrapped = []

            def record(function, *args, **kw):
                wrapped.append(list(net.trunk_layers).index(function))
                return checkpoint_activation(function, *args, **kw)

            with self.subTest(saved=saved), patch("model4.player.checkpoint_activation", side_effect=record):
                net.train()
                board, _ = net.encode_board(*inputs[:8], inputs[15])
                board.float().sum().backward()
                self.assertEqual(wrapped, expected)
                self.assertTrue(all(not any(p.requires_grad for p in net.trunk_layers[i].parameters())
                                    for i in wrapped) or saved == 0)


class RecomputeState(unittest.TestCase):
    def test_recompute_sees_the_forward_guard_inputs(self):
        """What a Dynamo guard reads is the same in a layer's forward and its recompute."""
        trunk = stage_b_trunk()
        seen = []

        def record(layer):
            def run(x, pad):
                seen.append((x.requires_grad, torch.is_grad_enabled(), torch.is_autocast_enabled("cpu"),
                             all(p.requires_grad for p in layer.parameters()), tuple(x.shape)))
                return layer(x, src_key_padding_mask=pad)
            return run

        x = torch.randn(8, 12, 32)
        pad = torch.zeros(8, 12, dtype=torch.bool)
        with torch.autocast("cpu", dtype=torch.bfloat16):
            for layer in trunk:
                x = checkpoint_activation(record(layer), x, pad)
        forward = list(seen)
        x.float().sum().backward()
        recompute = seen[len(forward):]
        self.assertEqual(sorted(recompute), sorted(forward[2:]))   # the trainable layers, frozen input first


@unittest.skipUnless(COMPILE, "Inductor on the CPU takes minutes; set BAYLEE_COMPILE_TESTS=1")
class CompiledCheckpoint(unittest.TestCase):
    def setUp(self):
        import torch._dynamo
        torch._dynamo.reset()
        torch.set_num_threads(2)

    def compiled(self):
        trunk = stage_b_trunk()
        for layer in trunk:
            layer.compile(dynamic=True)
        return trunk

    def test_recomputed_compiled_layers_on_the_cpu(self):
        """Stage B's variants (frozen and trainable parameters, frozen and
        differentiable inputs, round-0 board shapes) compile five graphs here and
        every recompute runs its forward's graph; the mismatch needs CUDA's
        lowering and cache. Kept to show a CPU build where it does reproduce."""
        try:
            train_steps(self.compiled(), saved_layers=0)
        except torch.utils.checkpoint.CheckpointError as error:   # pragma: no cover - CUDA-only so far
            self.fail(f"the CUDA failure class reproduces on the CPU: {str(error).splitlines()[0]}")

    def test_trainable_layers_without_recompute_match_eager(self):
        eager = train_steps(stage_b_trunk(), saved_layers=2)
        compiled = train_steps(self.compiled(), saved_layers=2)
        for left, right in zip(compiled, eager):   # bf16 autocast: fused kernels round differently
            torch.testing.assert_close(left, right, rtol=2e-2, atol=5e-4)


if __name__ == "__main__":
    unittest.main()

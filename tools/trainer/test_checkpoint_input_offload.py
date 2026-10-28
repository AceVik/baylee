"""Checkpoint-input offload controls; CUDA numerical checks are explicit opt-in.

Eager/math checks do not establish exact compiled BF16 training parity.
"""
import contextlib
import copy
import io
import os
import subprocess
import sys
import unittest
from unittest.mock import patch

import torch
import model4 as M
import model5 as M5
from model4.activation_checkpoint import checkpoint_activation
from export_onnx4 import make_dummy_inputs
from training4.cli import parse_args


class CheckpointInputOffload(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def test_cli_defaults_and_requires_activation_checkpointing(self):
        base = ['train5.py', '--data', 'unused', '--out', 'unused']
        with patch('sys.argv', base):
            self.assertFalse(parse_args('v5')[0].offload_checkpoint_inputs)
        with (
            patch('sys.argv', base + ['--offload-checkpoint-inputs']),
            contextlib.redirect_stderr(io.StringIO()),
            self.assertRaises(SystemExit),
        ):
            parse_args('v5')
        with patch(
            'sys.argv',
            base + ['--activation-checkpointing', '--offload-checkpoint-inputs'],
        ):
            self.assertTrue(parse_args('v5')[0].offload_checkpoint_inputs)

    def test_cpu_flag_does_not_enter_save_on_cpu_and_preserves_rng(self):
        x = torch.randn(5, requires_grad=True)
        with patch('torch.autograd.graph.save_on_cpu') as offload:
            torch.manual_seed(81)
            out = checkpoint_activation(
                lambda a: torch.nn.functional.dropout(a, p=.25, training=True).square(),
                x,
                offload_inputs=True,
            )
            out.sum().backward()
            observed = (out.detach().clone(), x.grad.clone(), torch.get_rng_state())
            offload.assert_not_called()
        y = x.detach().clone().requires_grad_()
        torch.manual_seed(81)
        expected = checkpoint_activation(
            lambda a: torch.nn.functional.dropout(a, p=.25, training=True).square(), y,
        )
        expected.sum().backward()
        torch.testing.assert_close(observed[0], expected, rtol=0, atol=0)
        torch.testing.assert_close(observed[1], y.grad, rtol=0, atol=0)
        self.assertTrue(torch.equal(observed[2], torch.get_rng_state()))

    def assert_model_parity(self, device, bf16=False):
        torch.manual_seed(41)
        cfg = M5.Config5(
            d_model=32,
            structure_width=32,
            n_heads=4,
            n_layers=2,
            d_ff=64,
            relational_width=8,
            relational_revision=2,
            dropout=.1,
            activation_checkpointing=True,
        )
        left = M5.PlayerNet5(cfg).to(device).train()
        right = copy.deepcopy(left)
        right.offload_checkpoint_inputs = True
        self.assertEqual(left.state_dict().keys(), right.state_dict().keys())
        self.assertEqual(left.cfg.to_dict(), right.cfg.to_dict())
        self.assertNotIn('offload_checkpoint_inputs', right.cfg.to_dict())
        frames = [
            [
                value.to(device)
                for value in make_dummy_inputs(32, B=2, E=5, M_edges=3, C=4, P=3, K=2)
            ]
            for _ in range(3)
        ]
        initial = torch.randn(2, M.RECURRENT_SLOTS, 32, device=device)
        optimizers = [torch.optim.AdamW(net.parameters(), lr=1e-4) for net in (left, right)]
        observations = []
        for net, opt in zip((left, right), optimizers):
            torch.manual_seed(99)
            if device.type == 'cuda':
                torch.cuda.manual_seed_all(99)
            h = initial.clone()
            loss = torch.zeros((), device=device)
            for args in frames:
                with torch.autocast(
                    device_type=device.type, dtype=torch.bfloat16, enabled=bf16,
                ):
                    output = net(*args[:-2], h, args[-1])
                    h, candidates, pairs, scalar, value = output
                    loss = (
                        loss
                        + h.square().mean()
                        + candidates.masked_fill(~args[11], 0).float().square().mean()
                        + pairs.masked_fill(~args[13], 0).float().square().mean()
                        + scalar.float().square().mean()
                        + value.float().square().mean()
                    )
            loss.backward()
            observations.append((
                loss.detach(),
                h.detach(),
                {
                    name: p.grad.detach().clone()
                    for name, p in net.named_parameters()
                    if p.grad is not None
                },
                torch.get_rng_state(),
                torch.cuda.get_rng_state() if device.type == 'cuda' else None,
            ))
            opt.step()
        atol, rtol = (2e-3, 2e-2) if bf16 else (3e-6, 3e-5)
        for a, b in zip(observations[0][:2], observations[1][:2]):
            torch.testing.assert_close(a, b, atol=atol, rtol=rtol)
        self.assertEqual(observations[0][2].keys(), observations[1][2].keys())
        for name in observations[0][2]:
            torch.testing.assert_close(
                observations[0][2][name], observations[1][2][name],
                atol=atol, rtol=rtol, msg=name,
            )
        self.assertTrue(torch.equal(observations[0][3], observations[1][3]))
        if device.type == 'cuda':
            self.assertTrue(torch.equal(observations[0][4], observations[1][4]))
        for (name, a), (_, b) in zip(left.named_parameters(), right.named_parameters()):
            try:
                torch.testing.assert_close(a, b, atol=atol, rtol=rtol)
            except AssertionError as error:
                grad_left = observations[0][2].get(name)
                grad_right = observations[1][2].get(name)
                detail = (
                    f"{name}: max parameter difference={(a-b).abs().max().item():.9g}; "
                    f"gradient max left={grad_left.abs().max().item():.9g}, "
                    f"right={grad_right.abs().max().item():.9g}, "
                    f"difference={(grad_left-grad_right).abs().max().item():.9g}"
                    if grad_left is not None and grad_right is not None else name
                )
                raise AssertionError(f"{detail}\n{error}") from error

    def test_cpu_v5_full_recurrent_graph_gradients_update_and_state_contract(self):
        # CPU AdamW/checkpoint internals can query CUDA in a GPU-visible process.
        # Test CPU-only initialization in an isolated, genuinely GPU-hidden child.
        if os.environ.get('CUDA_VISIBLE_DEVICES') != '':
            env = os.environ.copy()
            env.update(
                CUDA_VISIBLE_DEVICES='',
                BAYLEE_TEST_CUDA_OFFLOAD='0',
                PYTHONDONTWRITEBYTECODE='1',
            )
            result = subprocess.run(
                [
                    sys.executable, '-m', 'unittest', '-v',
                    'test_checkpoint_input_offload.CheckpointInputOffload.test_cpu_v5_full_recurrent_graph_gradients_update_and_state_contract',
                ],
                cwd=os.path.dirname(__file__),
                env=env,
                capture_output=True,
                text=True,
                timeout=120,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return
        self.assertFalse(torch.cuda.is_initialized())
        self.assert_model_parity(torch.device('cpu'))
        self.assertFalse(torch.cuda.is_initialized())

    @unittest.skipUnless(
        os.environ.get('BAYLEE_TEST_CUDA_OFFLOAD') == '1',
        'explicit GPU opt-in required',
    )
    def test_cuda_gradients_optimizer_update_and_rng(self):
        # This is a deterministic eager/math numerical control, not a claim of
        # exact compiled BF16 production parity. Existing tolerances stay fixed.
        old_workspace = os.environ.get('CUBLAS_WORKSPACE_CONFIG')
        os.environ['CUBLAS_WORKSPACE_CONFIG'] = ':4096:8'
        deterministic = torch.are_deterministic_algorithms_enabled()
        warn_only = torch.is_deterministic_algorithms_warn_only_enabled()
        tf32 = torch.backends.cuda.matmul.allow_tf32
        try:
            if not torch.cuda.is_available():
                self.skipTest('CUDA unavailable')
            from torch.nn.attention import SDPBackend, sdpa_kernel

            torch.use_deterministic_algorithms(True)
            torch.backends.cuda.matmul.allow_tf32 = False
            with (
                sdpa_kernel(SDPBackend.MATH),
                torch.backends.cudnn.flags(
                    enabled=True, benchmark=False, deterministic=True, allow_tf32=False,
                ),
            ):
                for bf16 in (False, True):
                    with self.subTest(bf16=bf16):
                        self.assert_model_parity(torch.device('cuda'), bf16)
        finally:
            torch.use_deterministic_algorithms(deterministic, warn_only=warn_only)
            torch.backends.cuda.matmul.allow_tf32 = tf32
            if old_workspace is None:
                os.environ.pop('CUBLAS_WORKSPACE_CONFIG', None)
            else:
                os.environ['CUBLAS_WORKSPACE_CONFIG'] = old_workspace


if __name__ == '__main__':
    unittest.main()

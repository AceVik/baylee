"""Depth contract and next-update recurrent checkpoint regression."""
import io
import random
import unittest
import numpy as np
import torch
import model5 as M
from checkpoint4 import rng_state, restore_rng
from training4.depth_schedule import DepthSchedule, evaluate_depths
from training4.execution import plan_batch
from test_training_execution4 import batch_fixture

class DepthContracts(unittest.TestCase):
    def setUp(self):
        previous = torch.are_deterministic_algorithms_enabled()
        torch.use_deterministic_algorithms(True)
        self.addCleanup(torch.use_deterministic_algorithms, previous)
        torch.set_num_threads(1)
        torch.manual_seed(71)
        random.seed(71)
        np.random.seed(71)

    def test_contract(self):
        fixed = DepthSchedule()
        fixed.check_resume({"depth_schedule": fixed.to_dict()})
        for saved in ({}, {"depth_schedule": DepthSchedule.from_training(curriculum=True).to_dict()},
                      {"depth_schedule": {**fixed.to_dict(), "selection_k": 8}}):
            with self.assertRaises(ValueError): fixed.check_resume(saved)
        with self.assertRaises(ValueError): DepthSchedule.from_training(0)

    def test_phase_and_counts(self):
        schedule = DepthSchedule.from_training(curriculum=True)
        batch = batch_fixture(b=2, u=8)
        batch["step_valid"].fill_(True)
        batch["is_decision"].fill_(True)
        batch["is_decision"][:, 0] = False
        batch["step_valid"][0, 1] = False
        batch["loss_weight"].fill_(0.25)
        totals = {}
        schedule.count_batch(batch, 3, totals)
        self.assertEqual([p.thinking for p in plan_batch(batch, 3, schedule=schedule)], [0, 1, 2, 4, 8, 1, 2, 4])
        self.assertEqual(sum(x["decisions"] for x in totals.values()), 13)
        self.assertEqual(sum(x["policy_loss_weight"] for x in totals.values()), 3.25)

    def test_validation(self):
        calls = []
        def evaluator(net, loader, device, k):
            calls.append((k, list(loader)))
            return {"val_choice_nll": k}
        result = evaluate_depths(None, [0, 1, 2], None, DepthSchedule(), evaluator)
        self.assertEqual(calls, [(k, [0, 1, 2]) for k in (1, 2, 4, 8)])
        self.assertEqual(result["val_choice_nll"], 2)

    def test_next_update(self):
        for mixed in (False, True):
            with self.subTest(mixed=mixed):
                schedule = DepthSchedule.from_training(curriculum=mixed)
                device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
                net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64)).to(device)
                opt = torch.optim.AdamW(net.parameters(), lr=1e-3)
                batch = {name: value.to(device) for name, value in batch_fixture(b=2, u=4).items()}
                batch["is_decision"][:, 0] = False
                h = torch.zeros(2, M.RECURRENT_SLOTS, 32, device=device)
                def update(net, opt, h, step):
                    opt.zero_grad(set_to_none=True)
                    loss = 0
                    for t in range(4):
                        k = schedule.depth(step, t) * batch["is_decision"][:, t].long()
                        out = net(*[batch[n][:, t] for n in M.INPUT_NAMES[:-2]], h, k)
                        h = out[0]
                        loss = loss + out[1].square().mean() + out[4].square().mean()
                    loss = loss * (torch.rand((), device=h.device) + random.random() + np.random.random())
                    loss.backward()
                    opt.step()
                    return h.detach()
                h = update(net, opt, h, 0)
                buf = io.BytesIO()
                torch.save({"state": net.state_dict(), "optimizer": opt.state_dict(), "h": h,
                            "rng": rng_state(), "depth_schedule": schedule.to_dict()}, buf)
                expected_h = update(net, opt, h, 1)
                expected = {k: v.clone() for k, v in net.state_dict().items()}
                expected_opt = opt.state_dict()
                restored = M.PlayerNet5(net.cfg).to(device)
                restored_opt = torch.optim.AdamW(restored.parameters(), lr=1e-3)
                buf.seek(0)
                saved = torch.load(buf, weights_only=False)
                schedule.check_resume(saved)
                restored.load_state_dict(saved["state"])
                restored_opt.load_state_dict(saved["optimizer"])
                restore_rng(saved["rng"])
                actual_h = update(restored, restored_opt, saved["h"], 1)
                torch.testing.assert_close(actual_h, expected_h, atol=0, rtol=0)
                for name, tensor in restored.state_dict().items():
                    torch.testing.assert_close(tensor, expected[name], atol=0, rtol=0, msg=name)
                for key, state in restored_opt.state_dict()["state"].items():
                    for name, tensor in state.items():
                        torch.testing.assert_close(tensor, expected_opt["state"][key][name], atol=0, rtol=0)

    def test_local_step_diagnostics_preserve_h_and_restore_mode(self):
        from export_onnx4 import make_dummy_inputs
        from training4.depth_diagnostics import diagnose_depth_steps
        net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64)).train()
        inputs = make_dummy_inputs(32, E=3, M_edges=2, C=4, P=2, K=2)
        report = diagnose_depth_steps(net, inputs)
        self.assertTrue(net.training)
        for k, metrics in report["per_k"].items():
            self.assertEqual(len(metrics["latent_steps"]), int(k))
            self.assertEqual(metrics["persistent_h_max_abs_from_baseline"], 0)
            self.assertGreater(metrics["latent_steps"][0]["rms"], 0)

    def test_v5_future_frames_cannot_change_previous_outputs(self):
        net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
        batch = batch_fixture(b=2, u=4)
        schedule = DepthSchedule.from_training(curriculum=True)
        def run():
            h = torch.zeros(2, M.RECURRENT_SLOTS, 32)
            outputs = []
            for t in range(4):
                k = schedule.depth(0, t) * batch["is_decision"][:, t].long()
                values = net(*[batch[n][:, t] for n in M.INPUT_NAMES[:-2]], h, k)
                outputs.append(values)
                h = values[0]
            return outputs
        with torch.no_grad():
            before = run()
            batch["entity_feat"][:, -1] += 200
            after = run()
        for left, right in zip(before[:-1], after[:-1]):
            for a, b in zip(left, right):
                torch.testing.assert_close(a, b, atol=0, rtol=0)


    def test_explicit_cli_modes_and_transfer_guards(self):
        from unittest.mock import patch
        from training4.cli import parse_args
        from contextlib import redirect_stderr
        base = ["train5", "--data", "fixture", "--out", "fixture", "--architecture", "v5"]
        with patch("sys.argv", base + ["--depth-schedule", "legacy_frame_cycle"]):
            args, _, _ = parse_args()
            self.assertTrue(args.thinking_curriculum)
        with patch("sys.argv", base + ["--candidate-context-revision", "2", "--candidate-context-warmstart", "--init", "old.pt"]):
            args, _, _ = parse_args()
            self.assertTrue(args.candidate_context_warmstart)
        for flags in (["--depth-schedule", "fixed", "--thinking-curriculum"],
                      ["--thinking-steps", "0"],
                      ["--candidate-context-warmstart"],
                      ["--candidate-context-revision", "2", "--relational-revision", "1"],
                      ["--candidate-context-revision", "2", "--candidate-context-warmstart", "--init", "old.pt", "--encoder-warmstart-from7", "--encoder-version", "8"]):
            with patch("sys.argv", base + flags), redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): parse_args()

"""Execution optimizations must preserve losses, gradients and causal memory."""
import copy
import unittest
from types import SimpleNamespace
import torch
import model4 as M
from export_onnx4 import make_dummy_inputs
from training4.loss import compute_step_loss
from training4.device_loss import compute_device_loss
from training4.execution import plan_batch, encode_boards
from checkpoint4 import move_optimizer
from training4.validation import evaluate
from training4.validation_device import evaluate_device


def batch_fixture(b=8, u=4):
    inputs = make_dummy_inputs(32, E=5, M_edges=3, C=7, P=4, K=1)
    batch = {name: tensor.unsqueeze(1).repeat(b, u, *([1] * (tensor.ndim - 1)))
             for name, tensor in zip(M.INPUT_NAMES[:-2], inputs[:-2])}
    batch.update(step_valid=torch.ones(b, u, dtype=torch.bool),
                 is_decision=torch.ones(b, u, dtype=torch.bool),
                 head=torch.arange(b)[:, None].expand(b, u) % 4,
                 loss_weight=torch.rand(b, u), outcome=torch.rand(b, u),
                 aux_target=torch.rand(b, u, 4), single_idx=torch.zeros(b, u, dtype=torch.long),
                 scalar_target=torch.rand(b, u))
    for prefix in ("cand", "pair"):
        batch[prefix + "_free_mask"] = batch[prefix + "_mask"].float().clone()
        batch[prefix + "_target_mask"] = torch.zeros_like(batch[prefix + "_free_mask"])
    batch["step_valid"][-1, -2:] = False
    batch["is_decision"][:, 0] = False
    batch["cand_mask"][:, 0] = False
    batch["pair_mask"][:, 0] = False
    batch["single_idx"][batch["head"] != 0] = 9999
    return batch


class TrainingExecution(unittest.TestCase):
    def setUp(self):
        torch.manual_seed(91)
        torch.set_num_threads(1)

    def test_loss_and_output_gradients_match_all_heads_and_empty_masks(self):
        batch = batch_fixture()
        for t, plan in enumerate(plan_batch(batch, 0)):
            outputs = [torch.randn(8, 7), torch.randn(8, 4), torch.rand(8), torch.rand(8), torch.rand(8, 4)]
            old = [x.clone().requires_grad_() for x in outputs]
            new = [x.clone().requires_grad_() for x in outputs]
            reference, stats = compute_step_loss(*old, batch, t)
            actual, device_stats = compute_device_loss(*new, batch, t, plan)
            torch.testing.assert_close(actual, reference, atol=1e-6, rtol=1e-6)
            expected_stats = torch.tensor([stats[k] for k in ("pol_loss", "val_loss", "aux_loss", "correct_single", "total_single")])
            torch.testing.assert_close(device_stats, expected_stats, atol=1e-6, rtol=1e-6)
            reference.backward(); actual.backward()
            for a, b in zip(old, new):
                self.assertEqual(a.grad is None, b.grad is None)
                if a.grad is not None:
                    torch.testing.assert_close(a.grad, b.grad, atol=1e-7, rtol=1e-6)
        batch["step_valid"].zero_()
        plan = plan_batch(batch, 0)[0]
        loss, _ = compute_device_loss(*new, batch, 0, plan)
        self.assertFalse(loss.requires_grad)
        self.assertEqual(float(loss), 0)

    def test_plan_keeps_noncontiguous_last_candidate_and_passive_clock(self):
        batch = batch_fixture()
        batch["cand_mask"].zero_()
        batch["cand_mask"][:, 1, 6] = True
        plans = plan_batch(batch, 1, curriculum=True)
        self.assertEqual(plans[0].thinking, 0)
        self.assertEqual(plans[1].hint, (7, 4, 4))
        self.assertEqual(plans[2].candidates, 1)

    def test_grouped_boards_preserve_recurrence_outputs_and_parameter_gradients(self):
        batch = batch_fixture(b=4)
        config = M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64, activation_checkpointing=True)
        reference = M.PlayerNet4(config)
        actual = copy.deepcopy(reference)
        plans = plan_batch(batch, 0)

        def unroll(net, grouped):
            boards = encode_boards(net, batch, 3 if grouped else 1, torch.device("cpu"))
            h = torch.zeros(4, 9, 32)
            results = []
            loss = 0
            for t in range(4):
                k = batch["is_decision"][:, t].long()
                out = net(*[batch[n][:, t] for n in M.INPUT_NAMES[:-2]], h, k,
                          execution_hint=plans[t].hint if grouped else None,
                          encoded_board=boards[t] if grouped else None)
                h = torch.where(batch["step_valid"][:, t, None, None], out[0], h)
                results.append(tuple(x.detach() for x in out))
                loss = loss + out[0].square().mean() + out[3].square().mean() + out[4].square().mean()
            loss.backward()
            return results
        left, right = unroll(reference, False), unroll(actual, True)
        for a, b in zip(left, right):
            for x, y in zip(a, b):
                torch.testing.assert_close(x, y, atol=2e-6, rtol=2e-5)
        for a, b in zip(reference.parameters(), actual.parameters()):
            self.assertEqual(a.grad is None, b.grad is None)
            if a.grad is not None:
                torch.testing.assert_close(a.grad, b.grad, atol=2e-6, rtol=2e-5)

    def test_future_observation_cannot_change_earlier_board(self):
        batch = batch_fixture(b=2)
        net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
        with torch.no_grad():
            before = encode_boards(net, batch, 4, torch.device("cpu"))
            batch["entity_feat"][:, 3] += 500
            after = encode_boards(net, batch, 4, torch.device("cpu"))
        for t in range(3):
            torch.testing.assert_close(before[t][0], after[t][0], rtol=0, atol=0)
        self.assertFalse(torch.equal(before[3][0], after[3][0]))

    def test_token_budget_shrinks_large_board_chunks_without_changing_boards(self):
        batch = batch_fixture(b=2, u=8)
        net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
        tokens_per_frame = 2 * (M.RECURRENT_SLOTS + batch["entity_feat"].shape[2])
        calls = []
        original = net.encode_board
        net.encode_board = lambda *inputs: calls.append(inputs[0].shape[0]) or original(*inputs)
        with torch.no_grad():
            unlimited = encode_boards(net, batch, 4, torch.device("cpu"))
            for budget, rows in ((tokens_per_frame * 2, 4), (tokens_per_frame - 1, 2)):
                calls.clear()
                net.board_token_budget = budget
                limited = encode_boards(net, batch, 4, torch.device("cpu"))
                self.assertEqual(set(calls), {rows})
                for (a, mask_a), (b, mask_b) in zip(unlimited, limited):
                    torch.testing.assert_close(a, b, rtol=1e-6, atol=1e-6)
                    self.assertTrue(torch.equal(mask_a, mask_b))

    def test_entity_padding_preserves_every_visible_token_and_reference(self):
        batch = batch_fixture(b=2)
        net = M.PlayerNet4(M.Config4(d_model=32,n_layers=1,n_heads=4,d_ff=64)).eval()
        plans = plan_batch(batch,0)
        with torch.no_grad():
            ordinary = encode_boards(net,batch,2,torch.device("cpu"))
            padded = encode_boards(net,batch,2,torch.device("cpu"),entity_bucket=16)
            self.assertEqual(padded[0][0].shape[1],9+16)
            self.assertTrue(padded[0][1][:,14:].all())
            for t in range(4):
                torch.testing.assert_close(ordinary[t][0],padded[t][0][:,:14],atol=1e-6,rtol=1e-5)
                args=[batch[n][:,t] for n in M.INPUT_NAMES[:-2]]+[torch.zeros(2,9,32),batch["is_decision"][:,t].long()]
                left=net(*args,execution_hint=plans[t].hint,encoded_board=ordinary[t])
                right=net(*args,execution_hint=plans[t].hint,encoded_board=padded[t])
                for a,b in zip(left,right):torch.testing.assert_close(a,b,atol=1e-6,rtol=1e-5)

    def test_device_validation_preserves_all_metrics_and_resets(self):
        first, second = batch_fixture(), batch_fixture()
        first["is_new_traj"] = torch.ones(8, dtype=torch.bool)
        second["is_new_traj"] = torch.tensor([True,False]*4)
        for batch in (first,second):
            # The old validator gathered irrelevant non-single labels too.
            batch["single_idx"][batch["head"] != 0] = 0
        class Loader:
            dataset = SimpleNamespace(unroll_len=4)
            def __iter__(self): return iter((first,second))
            def __len__(self): return 2
        net = M.PlayerNet4(M.Config4(d_model=32,n_layers=1,n_heads=4,d_ff=64)).eval()
        expected = evaluate(net,Loader(),torch.device("cpu"),2)
        actual = evaluate_device(net,Loader(),torch.device("cpu"),2,board_chunk=3)
        self.assertEqual(expected.keys(),actual.keys())
        for key in expected:
            self.assertAlmostEqual(actual[key],expected[key],delta=2e-6,msg=key)
        net.prune_unused_policy_heads = True
        pruned = evaluate_device(net,Loader(),torch.device("cpu"),2,board_chunk=3)
        for key in expected:
            self.assertAlmostEqual(pruned[key],expected[key],delta=2e-6,msg=key)
        net.decision_chunk = 3
        deferred = evaluate_device(net,Loader(),torch.device("cpu"),2,board_chunk=3)
        for key in expected:
            self.assertAlmostEqual(deferred[key],expected[key],delta=2e-6,msg=key)

    @unittest.skipUnless(torch.cuda.is_available(), "CUDA optimizer placement regression")
    def test_resume_keeps_unfused_adamw_steps_on_cpu(self):
        for fused in (False, True):
            parameter = torch.nn.Parameter(torch.ones(3, device="cuda"))
            optimizer = torch.optim.AdamW([parameter], fused=fused)
            parameter.sum().backward(); optimizer.step(); optimizer.zero_grad()
            before = parameter.detach().clone()
            saved = copy.deepcopy(optimizer.state_dict())
            parameter.sum().backward(); optimizer.step(); optimizer.zero_grad()
            expected = parameter.detach().clone()
            parameter.data.copy_(before)
            optimizer.load_state_dict(saved)
            # Simulate the old resume helper's incorrect all-CUDA placement.
            optimizer.state[parameter]["step"] = optimizer.state[parameter]["step"].cuda()
            move_optimizer(optimizer, "cuda")
            self.assertEqual(optimizer.state[parameter]["step"].device.type, "cuda" if fused else "cpu")
            self.assertEqual(optimizer.state[parameter]["exp_avg"].device.type, "cuda")
            parameter.sum().backward(); optimizer.step()
            torch.testing.assert_close(parameter, expected, rtol=0, atol=0)


if __name__ == "__main__":
    unittest.main()

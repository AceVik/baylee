"""Deferred readouts must preserve causality and the complete recurrent gradient."""
import copy
import unittest

import torch
import model4 as M
from test_training_execution4 import batch_fixture
from training4.deferred import deferred_readouts, readout_groups
from training4.execution import plan_batch, encode_boards, StepPlan
from training4.device_loss import compute_device_loss, compute_unroll_device_loss


class DeferredReadouts(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(47)

    def fixture(self):
        batch = batch_fixture(b=4, u=8)
        batch["is_decision"][:, ::2] = False
        batch["is_decision"][0, 1::2] = False
        batch["step_valid"][1, 2] = False
        batch["cand_mask"][:, 3, 5:] = False
        batch["cand_mask"][:, 5, 4:] = False
        batch["pair_mask"][:, 5, 2:] = False
        return batch

    def outputs(self, net, batch, chunk, thinking_steps=4):
        plans = plan_batch(batch, 0, thinking_steps=thinking_steps)
        boards = encode_boards(net, batch, 3, torch.device("cpu"))
        h = torch.zeros(4, 9, 32)
        if chunk:
            out, h = deferred_readouts(net, batch, boards, h, plans, chunk, torch.device("cpu"))
        else:
            out = []
            for t, plan in enumerate(plans):
                values = net(*[batch[n][:, t] for n in M.INPUT_NAMES[:-2]], h,
                             batch["is_decision"][:, t].long() * plan.thinking,
                             execution_hint=plan.hint, encoded_board=boards[t],
                             policy_heads=plan.heads if getattr(net, "prune_unused_policy_heads", False) else None)
                out.append(values)
                h = torch.where(batch["step_valid"][:, t, None, None], values[0].float(), h)
        return out, h, plans

    def test_outputs_memory_loss_and_every_parameter_gradient(self):
        for selected, thinking_steps in ((s, k) for s in (False, True) for k in (1, 2, 4, 8)):
            with self.subTest(selected=selected, thinking_steps=thinking_steps):
                batch = self.fixture()
                net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64,
                                            activation_checkpointing=True))
                net.cache_thinking_kv = True
                net.prune_unused_policy_heads = selected
                actual = copy.deepcopy(net)
                left, h_left, plans = self.outputs(net, batch, 0, thinking_steps)
                right, h_right, _ = self.outputs(actual, batch, 3, thinking_steps)
                torch.testing.assert_close(h_left, h_right, atol=0, rtol=0)
                for t, (old, new) in enumerate(zip(left, right)):
                    for head, (a, b) in enumerate(zip(old, new)):
                        # Unrequested policy outputs may be placeholders.
                        if selected and head in (1, 2, 3):
                            needed = (plans[t].heads[0] or plans[t].heads[1],
                                      plans[t].heads[2], plans[t].heads[3])[head-1]
                            if not needed: continue
                        torch.testing.assert_close(a, b, atol=3e-6, rtol=3e-5)
                def backward(model, outputs, vectorized=False):
                    if vectorized:
                        aux_all = model.aux_head(torch.stack([out[0][:, 0] for out in outputs], dim=1))
                        total, stats = compute_unroll_device_loss(outputs, aux_all.float(), batch, plans)
                        total.backward()
                        return total, stats
                    total = 0
                    stats_sum = torch.zeros(5)
                    for t, out in enumerate(outputs):
                        auxiliary = model.aux_head(out[0][:, 0])
                        loss, st = compute_device_loss(*out[1:], auxiliary, batch, t, plans[t])
                        total = total + loss/8
                        stats_sum = stats_sum + st
                    total.backward()
                    return total, stats_sum
                loss_l, stats_l = backward(net, left, vectorized=False)
                loss_r, stats_r = backward(actual, right, vectorized=True)
                torch.testing.assert_close(loss_l, loss_r, atol=1e-6, rtol=1e-6)
                torch.testing.assert_close(stats_l, stats_r, atol=1e-5, rtol=1e-5)
                for (name, a), (_, b) in zip(net.named_parameters(), actual.named_parameters()):
                    self.assertEqual(a.grad is None, b.grad is None, name)
                    if a.grad is not None:
                        torch.testing.assert_close(a.grad, b.grad, atol=3e-6, rtol=3e-5, msg=name)

    def test_future_frame_cannot_change_earlier_memory_or_decisions(self):
        batch = self.fixture()
        net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
        with torch.no_grad():
            before, _, _ = self.outputs(net, batch, 8)
            batch["entity_feat"][:, -1] += 500
            after, _, _ = self.outputs(net, batch, 8)
        for t in range(7):
            for a, b in zip(before[t], after[t]):
                torch.testing.assert_close(a, b, atol=0, rtol=0)
        self.assertFalse(torch.equal(before[7][4], after[7][4]))

    def test_large_candidate_sets_stay_individual_without_dropping_frames(self):
        plans = [StepPlan(10000, 1, 4, True, (True, False, False, False))]*4
        groups = list(readout_groups(plans, 8, token_limit=6553))
        self.assertEqual(groups, [(0,), (1,), (2,), (3,)])


if __name__ == "__main__":
    unittest.main()

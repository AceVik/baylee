"""Actual policy evidence, partition invariance, transactional KL and frozen H/value."""
import copy
import unittest
import numpy as np
import torch
import torch.nn.functional as F
import model4 as M
from export_onnx4 import make_dummy_inputs
from policy_training4.optimization import (accept_final_state, behavior_kl, plan_groups,
                                           select_policy_parameters, surrogate_sum,
                                           transactional_step)
from policy_training4.forward import actor_outputs
from train_rl4 import action_logprob_and_kl


def assert_state_equal(test, left, right):
    if isinstance(left, torch.Tensor):
        torch.testing.assert_close(left, right, atol=0, rtol=0)
    elif isinstance(left, dict):
        test.assertEqual(left.keys(), right.keys())
        for key in left:
            assert_state_equal(test, left[key], right[key])
    elif isinstance(left, (list, tuple)):
        test.assertEqual(len(left), len(right))
        for a, b in zip(left, right):
            assert_state_equal(test, a, b)
    else:
        test.assertEqual(left, right)


class PolicyTraining(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(827)

    def test_group_counts_skip_zero_and_report_undersized_tail(self):
        actor = np.zeros((8, 100), dtype=bool)
        actor[1, :90] = True
        actor[3, :60] = True
        actor[5, :10] = True
        groups, report = plan_groups(actor, seed=10, minimum=128, microbatch_size=1)
        self.assertEqual(report["zero_actor_segments"], 5)
        selected = []
        accepted = 0
        for microbatches, decisions in groups:
            self.assertGreaterEqual(decisions, 128)
            indices = [idx for microbatch in microbatches for idx in microbatch]
            self.assertTrue(all(actor[idx].any() for idx in indices))
            self.assertEqual(decisions, int(actor[indices].sum()))
            selected.extend(indices)
            accepted += decisions
        self.assertEqual(len(set(selected)), len(selected))
        self.assertEqual(accepted + report["discarded_tail_decisions"], 160)
        self.assertLess(report["discarded_tail_decisions"], 128)

    def test_zero_actor_does_not_mutate_adam_or_parameters(self):
        p = torch.nn.Parameter(torch.tensor([.1]))
        opt = torch.optim.AdamW([p], lr=.1)
        before = copy.deepcopy(opt.state_dict())
        result = transactional_step([p], opt, [], 0, None, None, .02)
        self.assertFalse(result["accepted"])
        self.assertEqual(float(p.detach()), float(torch.tensor(.1)))
        assert_state_equal(self, opt.state_dict(), before)

    def test_partitioned_microbatches_match_one_group_gradient_update(self):
        x = torch.tensor([1., -2., .5, 3., -.3])
        advantage = torch.tensor([1., -.5, .3, .9, -.4])
        results = []
        for batches in ([torch.arange(5)], [torch.tensor([0]), torch.tensor([1, 2, 3, 4])]):
            p = torch.nn.Parameter(torch.tensor(.02))
            opt = torch.optim.SGD([p], lr=.05)
            old = F.logsigmoid(p.detach() * x)
            loss = lambda idx: surrogate_sum(F.logsigmoid(p*x[idx]), old[idx], advantage[idx], .2)
            kl = lambda idx: behavior_kl(F.logsigmoid(p*x[idx]), old[idx])
            row = transactional_step([p], opt, batches, 5, loss, kl, 1)
            self.assertTrue(row["accepted"])
            results.append(p.detach().clone())
        torch.testing.assert_close(results[0], results[1], atol=1e-7, rtol=1e-7)

    def test_zero_policy_gradient_cannot_produce_momentum_only_step(self):
        p = torch.nn.Parameter(torch.tensor(.1))
        opt = torch.optim.AdamW([p], lr=.1)
        p.square().backward(); opt.step(); opt.zero_grad()
        previous, adam = p.detach().clone(), copy.deepcopy(opt.state_dict())
        result = transactional_step([p], opt, [None], 128, lambda _: p*0,
                                    lambda _: torch.tensor(0.), .02)
        self.assertFalse(result["accepted"])
        self.assertIn("zero_policy_gradient", result["reason"])
        torch.testing.assert_close(p, previous, atol=0, rtol=0)
        assert_state_equal(self, opt.state_dict(), adam)

    def test_post_step_kl_overshoot_restores_adam_counters_moments_and_weights(self):
        p = torch.nn.Parameter(torch.tensor(.1))
        opt = torch.optim.AdamW([p], lr=.1)
        # Populate moments so rollback must restore more than an empty optimizer.
        p.square().backward(); opt.step(); opt.zero_grad()
        previous = p.detach().clone()
        adam = copy.deepcopy(opt.state_dict())
        row = transactional_step([p], opt, [None], 128, lambda _: -p*128,
                                 lambda _: torch.tensor(128.), .02)
        self.assertFalse(row["accepted"])
        torch.testing.assert_close(p, previous, atol=0, rtol=0)
        assert_state_equal(self, opt.state_dict(), adam)

    def test_real_policy_improves_while_value_and_recurrent_memory_remain_identical(self):
        net = M.PlayerNet4(M.Config4(d_model=16, n_heads=4, n_layers=1, d_ff=32)).eval()
        params = select_policy_parameters(net)
        source = copy.deepcopy(net.state_dict())
        args = list(make_dummy_inputs(16, B=2, E=3, M_edges=1, C=4, P=1, K=2))
        args[-2] = torch.randn(2, 9, 16)
        with torch.no_grad():
            before = net(*args)
            chosen = before[1].argmin(-1)
            old = F.log_softmax(before[1], -1).gather(1, chosen[:, None]).squeeze(1)
        def lp():
            return F.log_softmax(net(*args)[1], -1).gather(1, chosen[:, None]).squeeze(1)
        optimizer = torch.optim.AdamW(params, lr=.001, weight_decay=0)
        row = transactional_step(params, optimizer, [None], 2,
                                 lambda _: surrogate_sum(lp(), old, torch.ones(2), .2),
                                 lambda _: behavior_kl(lp(), old), .5)
        self.assertTrue(row["accepted"])
        with torch.no_grad():
            after = net(*args)
            self.assertGreater(float(lp().mean()), float(old.mean()))
            torch.testing.assert_close(before[0], after[0], atol=0, rtol=0)
            torch.testing.assert_close(before[4], after[4], atol=0, rtol=0)
        self.assertTrue(accept_final_state(net, source, .001, .02))
        self.assertFalse(accept_final_state(net, source, .05, .02))
        assert_state_equal(self, net.state_dict(), source)

    def test_actor_forward_matches_full_readouts_across_provenance_and_trimmed_frames(self):
        net = M.PlayerNet4(M.Config4(d_model=16, n_heads=4, n_layers=1, d_ff=32)).eval()
        select_policy_parameters(net)
        inputs = make_dummy_inputs(16, B=2, E=3, M_edges=1, C=4, P=3, K=2)
        batch = {name: torch.stack([value.clone() for _ in range(3)], 1)
                 for name, value in zip(M.INPUT_NAMES[:-2], inputs[:-2])}
        for t, size in enumerate((1, 3, 4)):
            batch["cand_mask"][:, t] = torch.arange(4) < size
            batch["pair_mask"][:, t] = torch.arange(3) < t+1
        batch.update(behavior_h=torch.randn(2, 9, 16), step_valid=torch.ones(2, 3, dtype=torch.bool),
                     is_decision=torch.tensor([[False, True, False], [True, False, True]]),
                     source_kind=torch.tensor([[1, 0, 1], [1, 1, 0]]),
                     thinking_steps=torch.tensor([[0, 2, 0], [4, 0, 1]]),
                     head=torch.tensor([[0, 0, 0], [0, 0, 1]]),
                     actor_mask=torch.tensor([[False, True, False], [False, False, True]]),
                     single_idx=torch.zeros(2, 3, dtype=torch.long),
                     temperature=torch.full((2, 3), .6),
                     cand_target_mask=torch.zeros(2, 3, 4), cand_free_mask=torch.zeros(2, 3, 4),
                     pair_target_mask=torch.zeros(2, 3, 3), pair_free_mask=torch.zeros(2, 3, 3),
                     behavior_logprob=torch.arange(6).reshape(2, 3).float(),
                     advantages=torch.arange(6).reshape(2, 3).float() / 10)
        batch["cand_target_mask"][1, 2, 1] = 1
        batch["cand_free_mask"][1, 2, :2] = 1
        h = batch["behavior_h"].clone()
        expected = []
        with torch.no_grad():
            for t in range(3):
                stored = batch["thinking_steps"][:, t]
                source = batch["source_kind"][:, t] == 0
                k = torch.where(batch["is_decision"][:, t], torch.where(source & (stored > 0), stored, 1), 0)
                out = net(*[batch[name][:, t] for name in M.INPUT_NAMES[:-2]], h, k)
                h = out[0]
                lp, _ = action_logprob_and_kl(*out[1:4], *out[1:4], batch, t)
                expected.append(lp[batch["actor_mask"][:, t]])
        actual, recorded, advantage = actor_outputs(net, batch)
        torch.testing.assert_close(actual, torch.cat(expected), rtol=2e-6, atol=2e-6)
        torch.testing.assert_close(recorded, torch.tensor([1., 5.]), rtol=0, atol=0)
        torch.testing.assert_close(advantage, torch.tensor([.1, .5]), rtol=0, atol=0)
        grouped = actor_outputs(net, batch, board_chunk=2)
        for reference, actual_grouped in zip((actual, recorded, advantage), grouped):
            torch.testing.assert_close(actual_grouped, reference, rtol=2e-6, atol=2e-6)
        actual.sum().backward()
        self.assertTrue(torch.isfinite(net.cand_scorer[0].weight.grad).all())


if __name__ == "__main__":
    unittest.main()

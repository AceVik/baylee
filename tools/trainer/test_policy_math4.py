"""CPU regressions for executed conditional Bernoulli probabilities and KL."""
import math
import unittest

import torch
import torch.nn.functional as F

from policy_math4 import action_logprob_and_kl


def log_sigmoid(x):
    # Independent scalar equivalent of the stable Rust collector calculation.
    return -max(-x, 0.0) - math.log1p(math.exp(-abs(x)))


def evaluate(head, values, anchor=None, target=None, free=None, valid=None, temp=1.0):
    n = len(values)
    logits = torch.tensor([values], dtype=torch.float32, requires_grad=True)
    anchor = torch.tensor([values if anchor is None else anchor], dtype=torch.float32)
    target = [1.0] * n if target is None else target
    free = [1.0] * n if free is None else free
    valid = [True] * n if valid is None else valid
    batch = {
        'head': torch.tensor([[head]]), 'single_idx': torch.tensor([[0]]),
        'temperature': torch.tensor([[temp]]),
    }
    for prefix in ('cand', 'pair'):
        batch[prefix + '_mask'] = torch.tensor([[valid]], dtype=torch.bool)
        batch[prefix + '_target_mask'] = torch.tensor([[target]])
        batch[prefix + '_free_mask'] = torch.tensor([[free]])
    unused = torch.zeros_like(logits, requires_grad=True)
    cand, pair = (unused, logits) if head == 2 else (logits, unused)
    acand, apair = (unused.detach(), anchor) if head == 2 else (anchor, unused.detach())
    scalar = torch.tensor([123.0], requires_grad=True)
    outputs = action_logprob_and_kl(cand, pair, scalar, acand, apair, scalar, batch, 0)
    return logits, outputs


class ConditionalBernoulliMath(unittest.TestCase):
    def test_logprob_matches_stable_scalar_collector_reference(self):
        for head in (1, 2):
            for temp in (0.0, 1e-4, 0.25, 1.0, 2.5):
                for target in ([1., 0., 1., 0., 1., 0.], [0., 1., 0., 1., 0., 1.]):
                    with self.subTest(head=head, temp=temp, target=target):
                        values = [-1000., -40., -0.7, 0.7, 40., 1000.]
                        _, (lp, kl) = evaluate(head, values, target=target, temp=temp)
                        scale = 1.0 if temp <= 1e-4 else 1.0 / temp
                        expected = sum(log_sigmoid(v * scale * (1 if t else -1))
                                       for v, t in zip(values, target))
                        self.assertAlmostEqual(lp.item(), expected, delta=5e-4)
                        self.assertEqual(kl.item(), 0.0)

    def test_rare_actions_keep_unit_gradients_at_saturated_logits(self):
        for head in (1, 2):
            for magnitude in (40., 1000.):
                for temp in (0.5, 1., 2.):
                    with self.subTest(head=head, magnitude=magnitude, temp=temp):
                        z, (lp, _) = evaluate(head, [magnitude, -magnitude], target=[0., 1.], temp=temp)
                        lp.sum().backward()
                        torch.testing.assert_close(z.grad, torch.tensor([[-1 / temp, 1 / temp]]))
                        self.assertAlmostEqual(lp.item(), -2 * magnitude / temp, delta=1e-4)

    def test_masks_exclude_forced_invalid_and_nonfinite_unused_logits(self):
        for head in (1, 2):
            with self.subTest(head=head):
                z, (lp, kl) = evaluate(
                    head, [-40., float('nan'), float('inf'), -float('inf')],
                    anchor=[2., float('nan'), -float('inf'), float('inf')],
                    free=[1., 0., 1., 0.], valid=[True, True, False, False],
                )
                self.assertAlmostEqual(lp.item(), -40., places=5)
                self.assertTrue(torch.isfinite(kl).all())
                (lp + kl).sum().backward()
                self.assertTrue(torch.isfinite(z.grad).all())
                torch.testing.assert_close(z.grad[:, 1:], torch.zeros(1, 3))
                _, (forced_lp, forced_kl) = evaluate(head, [1000., -1000.], free=[0., 0.])
                self.assertEqual(forced_lp.item(), 0.)
                self.assertEqual(forced_kl.item(), 0.)

    def test_kl_matches_independent_reference_and_gradients(self):
        for head in (1, 2):
            for values, anchor in (([-.8, 1.2], [.3, -2.]), ([-40., 40.], [40., -40.]),
                                   ([-1000., 1000.], [1000., -1000.])):
                with self.subTest(head=head, values=values):
                    z, (_, kl) = evaluate(head, values, anchor=anchor, temp=.5)
                    expected = 0.
                    expected_grad = []
                    for value, old in zip(values, anchor):
                        a, b = log_sigmoid(old * 2), log_sigmoid(-old * 2)
                        expected += math.exp(a) * (a - log_sigmoid(value * 2))
                        expected += math.exp(b) * (b - log_sigmoid(-value * 2))
                        expected_grad.append(2 * (math.exp(log_sigmoid(value * 2)) - math.exp(a)))
                    self.assertAlmostEqual(kl.item(), expected, delta=5e-4)
                    kl.sum().backward()
                    torch.testing.assert_close(z.grad, torch.tensor([expected_grad]), atol=1e-6, rtol=1e-6)

    def test_categorical_regression_preserves_mask_temperature_and_kl(self):
        z, (lp, kl) = evaluate(0, [1., 2., 1000.], anchor=[-.5, .2, -1000.],
                               valid=[True, True, False], temp=.5)
        policy = F.log_softmax(torch.tensor([[2., 4., -1e4]]), dim=-1)
        anchor = F.log_softmax(torch.tensor([[-1., .4, -1e4]]), dim=-1)
        expected_kl = (anchor.exp() * (anchor - policy)).sum(-1)
        torch.testing.assert_close(lp, policy[:, 0])
        torch.testing.assert_close(kl, expected_kl)
        lp.sum().backward()
        torch.testing.assert_close(z.grad[:, 2], torch.zeros(1))

    def test_scalar_remains_excluded(self):
        _, (lp, kl) = evaluate(3, [1000., -1000.])
        self.assertEqual(lp.item(), 0.)
        self.assertEqual(kl.item(), 0.)

    def test_train_rl4_reexports_same_callable(self):
        from train_rl4 import action_logprob_and_kl as compatibility_import
        self.assertIs(compatibility_import, action_logprob_and_kl)


if __name__ == '__main__':
    unittest.main()

"""A within-decision attention cache must preserve memory and all gradients."""
import copy
import unittest

import torch
import model4 as M
from export_onnx4 import make_dummy_inputs


class ThinkingCache(unittest.TestCase):
    def test_mixed_thinking_budgets_outputs_and_gradients(self):
        torch.set_num_threads(1)
        for maximum in (0, 1, 2, 4, 8):
            with self.subTest(maximum=maximum):
                torch.manual_seed(19)
                reference = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64))
                actual = copy.deepcopy(reference)
                actual.cache_thinking_kv = True
                self.assertEqual(reference.state_dict().keys(), actual.state_dict().keys())
                args = [x.repeat(3, *([1] * (x.ndim - 1))) for x in
                        make_dummy_inputs(32, E=5, M_edges=3, C=7, P=4, K=maximum)]
                args[-1] = torch.tensor([0, min(maximum, 1), maximum])
                args[-2] = torch.randn(3, 9, 32)
                left = reference(*args)
                right = actual(*args)
                for a, b in zip(left, right):
                    torch.testing.assert_close(a, b, atol=2e-6, rtol=2e-5)
                # Exercise every used policy/value branch without squaring the
                # -1e9 mask sentinels. Shared K/V must retain their gradients.
                def objective(outputs):
                    h, candidates, pairs, scalar, value = outputs
                    return (h.square().mean() + candidates.masked_fill(~args[11], 0).square().mean()
                            + pairs.masked_fill(~args[13], 0).square().mean()
                            + scalar.square().mean() + value.square().mean())
                objective(left).backward()
                objective(right).backward()
                for (name, a), (_, b) in zip(reference.named_parameters(), actual.named_parameters()):
                    self.assertEqual(a.grad is None, b.grad is None, name)
                    if a.grad is not None:
                        torch.testing.assert_close(a.grad, b.grad, atol=3e-6, rtol=3e-5, msg=name)
                # A later observation gets new projections, not a stale cache.
                args[0] = args[0] + .5
                with torch.no_grad():
                    for a, b in zip(reference(*args), actual(*args)):
                        torch.testing.assert_close(a, b, atol=2e-6, rtol=2e-5)


if __name__ == "__main__":
    unittest.main()

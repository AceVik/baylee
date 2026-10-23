"""Keeping activations must change memory use, not the recurrent gradient."""
import copy
import unittest

import torch
import model4 as M
from test_training_execution4 import batch_fixture
from training4.execution import encode_boards, plan_batch
from training4.deferred import deferred_readouts
from training4.device_loss import compute_device_loss


class ActivationRetentionTests(unittest.TestCase):
    def test_every_retention_level_preserves_loss_memory_gradients_and_rng(self):
        torch.set_num_threads(1)
        torch.manual_seed(28)
        for dropout in (0.0, 0.2):
            batch = batch_fixture(b=4, u=4)
            plans = plan_batch(batch, 0, curriculum=True)
            reference = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=3, d_ff=64,
                                              activation_checkpointing=True, dropout=dropout))
            def run(net):
                torch.manual_seed(124)
                boards = encode_boards(net, batch, 2, torch.device("cpu"))
                # Dropout runs test the trunk directly; grouped readouts require zero dropout.
                if dropout:
                    loss = sum(board.square().mean() for board, _ in boards)
                    hidden = boards[-1][0]
                else:
                    outputs, hidden = deferred_readouts(net, batch, boards, torch.zeros(4, 9, 32),
                                                        plans, 2, torch.device("cpu"))
                    loss = sum(compute_device_loss(*out[1:], net.aux_head(out[0][:, 0]), batch, t, plans[t])[0]
                               for t, out in enumerate(outputs)) / 4
                loss.backward()
                return loss.detach(), hidden.detach(), torch.get_rng_state()

            baseline = run(reference)
            for saved_layers in (1, 2, 3):
                with self.subTest(dropout=dropout, saved_layers=saved_layers):
                    actual = copy.deepcopy(reference)
                    actual.zero_grad(set_to_none=True)
                    actual.saved_activation_layers = saved_layers
                    observed = run(actual)
                    for old, new in zip(baseline, observed):
                        torch.testing.assert_close(old, new, rtol=0, atol=0)
                    self.assertEqual(reference.state_dict().keys(), actual.state_dict().keys())
                    for (name, old), (_, new) in zip(reference.named_parameters(), actual.named_parameters()):
                        self.assertEqual(old.grad is None, new.grad is None, name)
                        if old.grad is not None:
                            torch.testing.assert_close(old.grad, new.grad, rtol=0, atol=0, msg=name)


if __name__ == "__main__":
    unittest.main()

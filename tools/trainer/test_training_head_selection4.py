"""Omitting unused forward branches must preserve the complete training update."""
import copy
import unittest
import torch
import model4 as M
from test_training_execution4 import batch_fixture
from training4.execution import plan_batch, encode_boards
from training4.device_loss import compute_device_loss


class TrainingHeadSelection(unittest.TestCase):
    def test_every_head_loss_memory_and_gradients_match(self):
        torch.set_num_threads(1)
        for head in range(4):
            with self.subTest(head=head):
                torch.manual_seed(71)
                batch = batch_fixture(b=4)
                batch["head"].fill_(head)
                if head == 0:
                    batch["single_idx"].zero_()
                plans = plan_batch(batch, 1, curriculum=True)
                reference = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1,
                                                  d_ff=64, activation_checkpointing=True))
                reference.cache_thinking_kv = True
                actual = copy.deepcopy(reference)

                def unroll(net, selected):
                    boards = encode_boards(net, batch, 2, torch.device("cpu"))
                    h = torch.zeros(4, 9, 32)
                    loss, stats = 0, torch.zeros(5)
                    for t, plan in enumerate(plans):
                        thinking = batch["is_decision"][:, t].long() * plan.thinking
                        out = net(*[batch[n][:, t] for n in M.INPUT_NAMES[:-2]], h, thinking,
                                  execution_hint=plan.hint, encoded_board=boards[t],
                                  policy_heads=plan.heads if selected else None)
                        auxiliary = net.aux_head(out[0][:, 0])
                        step_loss, step_stats = compute_device_loss(*out[1:], auxiliary, batch, t, plan)
                        h = torch.where(batch["step_valid"][:, t, None, None], out[0], h)
                        loss = loss + step_loss / 4
                        stats += step_stats
                    loss.backward()
                    return loss, stats, h

                for a, b in zip(unroll(reference, False), unroll(actual, True)):
                    torch.testing.assert_close(a, b, atol=1e-6, rtol=1e-6)
                for (name, a), (_, b) in zip(reference.named_parameters(), actual.named_parameters()):
                    self.assertEqual(a.grad is None, b.grad is None, name)
                    if a.grad is not None:
                        torch.testing.assert_close(a.grad, b.grad, atol=1e-6, rtol=1e-6, msg=name)


if __name__ == "__main__":
    unittest.main()

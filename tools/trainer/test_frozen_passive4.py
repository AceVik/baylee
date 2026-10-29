"""Passive observations must retain memory without touching Adam or policy weights."""
import copy
from types import SimpleNamespace
import unittest

import torch
import model4 as M
from test_training_execution4 import batch_fixture
from training4.execution import plan_batch
from training4.device_loss import compute_device_loss
from training4.optimizer_step import update_parameters


class FrozenPassiveTest(unittest.TestCase):
    def test_passive_memory_then_real_update_matches_without_spurious_adam_step(self):
        torch.set_num_threads(1)
        torch.manual_seed(832)
        net = M.PlayerNet4(M.Config4(d_model=32, n_layers=1, n_heads=4, d_ff=64))
        for name, parameter in net.named_parameters():
            parameter.requires_grad_(name.startswith(('cand_proj.', 'cand_ctx_proj.', 'cand_scorer.')))
        optimizer = torch.optim.AdamW([p for p in net.parameters() if p.requires_grad])
        batch = batch_fixture(b=2, u=1)
        batch['is_decision'].zero_()
        batch['head'].zero_()
        batch['step_valid'].fill_(True)
        initial = {n: p.detach().clone() for n, p in net.named_parameters()}
        plan = plan_batch(batch, 0)[0]
        memory = torch.zeros(2, M.RECURRENT_SLOTS, 32)
        output = net(*[batch[n][:, 0] for n in M.INPUT_NAMES[:-2]], memory,
                     torch.zeros(2, dtype=torch.long), execution_hint=plan.hint, policy_heads=plan.heads)
        loss, _ = compute_device_loss(*output[1:], net.aux_head(output[0][:, 0]), batch, 0, plan)
        self.assertFalse(loss.requires_grad)
        self.assertFalse(torch.equal(memory, output[0]))
        def forbidden_mutation():
            self.fail('passive batch reached mutation boundary')
        self.assertFalse(update_parameters(loss, net, optimizer,
                                          SimpleNamespace(before_mutation=forbidden_mutation), None, 1))
        self.assertEqual(len(optimizer.state), 0)
        for n, p in net.named_parameters():
            torch.testing.assert_close(p, initial[n], rtol=0, atol=0)
        reference = copy.deepcopy(net)
        expected_opt = torch.optim.AdamW([p for p in reference.parameters() if p.requires_grad])
        # A following supervised update must have Adam step one and no inherited decay.
        for actual_net, actual_opt in ((net, optimizer), (reference, expected_opt)):
            actual_opt.zero_grad(set_to_none=True)
            supervised = sum(p.square().sum() for p in actual_net.parameters() if p.requires_grad)
            self.assertTrue(update_parameters(supervised, actual_net, actual_opt,
                                             SimpleNamespace(before_mutation=lambda: None), None, 1))
        for left, right in zip(net.parameters(), reference.parameters()):
            torch.testing.assert_close(left, right, rtol=0, atol=0)
        self.assertTrue(all(int(state['step']) == 1 for state in optimizer.state.values()))


if __name__ == '__main__':
    unittest.main()

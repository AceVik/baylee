"""Relational schedules preserve legacy optimizer behavior and exact resume."""
import copy
import unittest
import torch
from torch import nn
from training4.optimizer_groups import build_optimizer_groups, apply_relational_warmup, check_resume_schedule


class Net(nn.Module):
    def __init__(self):
        super().__init__()
        self.shared = nn.Linear(2, 2)
        self.seat_value_head = nn.Linear(2, 1)
        self.relational = nn.Module()
        self.relational.inner = nn.Linear(2, 2)
        self.relational.output = nn.Linear(2, 2, bias=False)


class Groups(unittest.TestCase):
    def test_default_groups_exactly_preserve_legacy_layout(self):
        net = Net()
        groups, contract = build_optimizer_groups(net, 1e-5)
        named = list(net.named_parameters())
        expected = [dict(params=[p for n, p in named if 'seat_value_head' not in n], lr=1e-5, weight_decay=.01),
                    dict(params=[p for n, p in named if 'seat_value_head' in n], lr=1e-5 * .2, weight_decay=.05)]
        self.assertIsNone(contract)
        for actual, want in zip(groups, expected):
            self.assertEqual(set(actual), set(want))
            self.assertEqual([id(p) for p in actual['params']], [id(p) for p in want['params']])
            self.assertEqual(actual['lr'], want['lr'])
            self.assertEqual(actual['weight_decay'], want['weight_decay'])
        check_resume_schedule({}, None)

    def configured(self, net):
        return build_optimizer_groups(net, 1e-5, architecture='v5', relational_lr=1e-4,
                                      output_lr=3e-5, warmup_steps=100)

    def test_parameter_coverage_and_warmup_only_new_groups(self):
        net = Net()
        groups, contract = self.configured(net)
        ids = [id(p) for g in groups for p in g['params']]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(set(ids), {id(p) for p in net.parameters()})
        self.assertEqual([g['name'] for g in groups], ['shared', 'critic', 'relational_inner', 'relational_output'])
        self.assertEqual(groups[-1]['weight_decay'], 0.)
        opt = torch.optim.AdamW(groups)
        for update in (1, 50, 100, 101):
            apply_relational_warmup(opt, contract, update)
            actual = [g['lr'] for g in opt.param_groups]
            expected = [1e-5, 1e-5 * .2, 1e-4 * min(update / 100, 1), 3e-5 * min(update / 100, 1)]
            self.assertEqual(actual, expected)

    def test_actual_v5_layout_and_frozen_parameter_coverage(self):
        import model5 as V5
        net = V5.PlayerNet5(V5.Config5(relational_revision=2, d_model=16, structure_width=16,
                                      n_heads=2, n_layers=1, d_ff=32))
        next(net.global_proj.parameters()).requires_grad_(False)
        groups, contract = self.configured(net)
        ids = [id(p) for g in groups for p in g['params']]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(set(ids), {id(p) for p in net.parameters() if p.requires_grad})
        self.assertIs(groups[-1]['params'][0], net.relational.output.weight)
        self.assertEqual(contract['groups'][-1]['parameter_names'], ['relational.output.weight'])

    def test_candidate_decoder_learns_at_new_module_rates(self):
        import model5 as V5
        net = V5.PlayerNet5(V5.Config5(relational_revision=2, candidate_context_revision=3, d_model=16,
                                      structure_width=16, n_heads=2, n_layers=1, d_ff=32))
        groups, contract = self.configured(net)
        ids = [id(p) for g in groups for p in g['params']]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(set(ids), {id(p) for p in net.parameters()})
        by_name = {g['name']: g for g in contract['groups']}
        self.assertEqual(sorted(by_name['decoder_output']['parameter_names']),
                         sorted(n for n, _ in net.named_parameters()
                                if n.startswith('cand_decoder.') and ('.attn.out_proj.' in n or '.ff.2.' in n)))
        self.assertTrue(all(n.startswith('cand_decoder.') for n in by_name['decoder_inner']['parameter_names']))
        self.assertFalse(any(n.startswith('cand_decoder.') for n in by_name['shared']['parameter_names']))
        self.assertEqual(by_name['decoder_output']['weight_decay'], 0.)
        self.assertTrue(by_name['decoder_inner']['warmup'] and by_name['decoder_output']['warmup'])

    def test_resume_preserves_adam_state_and_next_update(self):
        net = Net()
        groups, contract = self.configured(net)
        opt = torch.optim.AdamW(groups)
        for update in range(1, 4):
            for p in net.parameters():
                p.grad = torch.ones_like(p)
            apply_relational_warmup(opt, contract, update)
            opt.step()
        saved = dict(optimizer_schedule=contract, optimizer=copy.deepcopy(opt.state_dict()), step_num=3)
        check_resume_schedule(saved, contract)
        clone = copy.deepcopy(net)
        new_groups, same = self.configured(clone)
        resumed = torch.optim.AdamW(new_groups)
        resumed.load_state_dict(saved['optimizer'])
        for model, optimizer in ((net, opt), (clone, resumed)):
            for p in model.parameters():
                p.grad = torch.ones_like(p)
            apply_relational_warmup(optimizer, same, 4)
            optimizer.step()
        for a, b in zip(net.parameters(), clone.parameters()):
            torch.testing.assert_close(a, b, atol=0, rtol=0)
        for mutation in ('contract', 'order', 'rate'):
            broken = copy.deepcopy(saved)
            if mutation == 'contract':
                broken['optimizer_schedule']['warmup_steps'] = 99
            elif mutation == 'order':
                broken['optimizer']['param_groups'].reverse()
            else:
                broken['optimizer']['param_groups'][-1]['lr'] = 1.
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, 'Resume'):
                check_resume_schedule(broken, contract)
        with self.assertRaises(ValueError):
            check_resume_schedule({}, contract)
        with self.assertRaises(ValueError):
            check_resume_schedule(saved, None)

    def test_invalid_options_and_inheritance(self):
        net = Net()
        for kwargs in (dict(relational_lr=1e-4), dict(architecture='v5', output_lr=1e-4),
                       dict(architecture='v5', warmup_steps=10),
                       dict(architecture='v5', relational_lr=1e-4, freeze_trunk=True),
                       dict(relational_lr=float('nan')), dict(warmup_steps=-1)):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                build_optimizer_groups(net, 1e-5, **kwargs)
        groups, _ = build_optimizer_groups(net, 1e-5, architecture='v5', relational_lr=1e-4)
        self.assertEqual(groups[-1]['lr'], 1e-4)


if __name__ == '__main__':
    unittest.main()

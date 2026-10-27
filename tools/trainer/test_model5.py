"""CPU regressions for v5 exact transfer and learned graph processing."""
import unittest
import torch
import model4 as V4
import model5 as V5


class Model5Tests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        self.options = dict(d_model=16, n_layers=1, n_heads=2, d_ff=32, structure_width=16)

    def test_warmstart_exact_rng_and_strictness(self):
        torch.manual_seed(81)
        old = V4.PlayerNet4(V4.Config4(**self.options)).eval()
        rng = torch.get_rng_state()
        torch.manual_seed(81)
        new = V5.PlayerNet5(V5.Config5(**self.options)).eval()
        self.assertTrue(torch.equal(rng, torch.get_rng_state()))
        new.load_state_dict(V5.warmstart_from_v4(old.state_dict(), new), strict=True)
        from export_onnx4 import make_dummy_inputs
        inputs = list(make_dummy_inputs(16, E=3, M_edges=3, C=3, P=3))
        inputs[-2] = torch.zeros((1, 9, 16))
        with torch.no_grad():
            for k in (0, 1, 2, 4):
                inputs[-1].fill_(k)
                for a, b in zip(old(*inputs), new(*inputs)):
                    self.assertTrue(torch.equal(a, b))
            new.cfg.relational_steps = 0
            for a, b in zip(old(*inputs), new(*inputs)):
                self.assertTrue(torch.equal(a, b))
        # Complete chronological history: observe once, then decide without another update.
        new.cfg.relational_steps = 2
        history = torch.zeros(1, 9, 16)
        with torch.no_grad():
            for _ in range(3):
                inputs[-2] = history
                inputs[-1].fill_(0)
                observed = new(*inputs)[0]
                torch.testing.assert_close(observed, old(*inputs)[0], rtol=0, atol=0)
                inputs[-2] = observed
                inputs[-1].fill_(2)
                decided = new(*inputs)[0]
                self.assertTrue(torch.equal(observed, decided))
                history = observed
        bad = dict(old.state_dict()); bad.pop(next(iter(bad)))
        with self.assertRaises(ValueError):
            V5.warmstart_from_v4(bad, new)

    def test_two_hop_dependency_and_inner_learning_after_output_update(self):
        torch.manual_seed(17)
        p = V5.RelationalProcessor(V5.Config5(**self.options, relational_width=8))
        x = torch.randn(1, 3, 16, requires_grad=True)
        edges = torch.tensor([[[0, 1], [1, 2]]])
        kinds = torch.zeros(1, 2, dtype=torch.long)
        mask = torch.ones(1, 2, dtype=torch.bool)
        present = torch.ones(1, 3, dtype=torch.bool)
        counts = torch.ones(1, 3)
        optimizer = torch.optim.SGD(p.parameters(), lr=.1)
        p(x, edges, kinds, mask, present, counts).square().sum().backward()
        self.assertGreater(p.output.weight.grad.abs().sum().item(), 0)
        self.assertEqual(p.message.weight.grad.abs().sum().item(), 0)
        optimizer.step(); optimizer.zero_grad()
        p.steps = 1
        one = p(x, edges, kinds, mask, present, counts)[0, 2].sum()
        one_grad = torch.autograd.grad(one, x)[0][0, 0]
        self.assertEqual(one_grad.abs().sum().item(), 0)
        p.steps = 2
        two = p(x, edges, kinds, mask, present, counts)[0, 2].sum()
        two_grad = torch.autograd.grad(two, x, retain_graph=True)[0][0, 0]
        self.assertGreater(two_grad.abs().sum().item(), 0)
        two.backward()
        self.assertGreater(p.message.weight.grad.abs().sum().item(), 0)

    def test_checkpointed_relation_matches_forward_and_backward(self):
        import copy
        ordinary = V5.PlayerNet5(V5.Config5(**self.options)).train()
        torch.nn.init.normal_(ordinary.relational.output.weight, std=.1)
        checked = copy.deepcopy(ordinary)
        checked.cfg.activation_checkpointing = True
        x = torch.randn(1, 12, 16)
        edges = torch.tensor([[[9, 10], [10, 11]]])
        kinds = torch.zeros(1, 2, dtype=torch.long)
        mask = torch.ones(1, 2, dtype=torch.bool)
        present = torch.ones(1, 12, dtype=torch.bool)
        features = torch.ones(1, 3, 128)
        a = ordinary.relational_tokens(x, edges, kinds, mask, present, features)
        b = checked.relational_tokens(x, edges, kinds, mask, present, features)
        torch.testing.assert_close(a, b, rtol=0, atol=0)
        a.square().sum().backward(); b.square().sum().backward()
        for first, second in zip(ordinary.relational.parameters(), checked.relational.parameters()):
            torch.testing.assert_close(first.grad, second.grad, rtol=0, atol=0)

    def test_count_direction_mask_permutation_and_gradient(self):
        torch.manual_seed(7)
        p = V5.RelationalProcessor(V5.Config5(**self.options, relational_width=8))
        x = torch.randn(1, 4, 16)
        e = torch.tensor([[[0, 1], [1, 2], [2, 3]]]); t = torch.tensor([[0, 2, 4]])
        m = torch.ones(1, 3, dtype=torch.bool); present = torch.ones(1, 4, dtype=torch.bool)
        counts = torch.ones(1, 4)
        y = p(x, e, t, m, present, counts)
        self.assertTrue(torch.equal(x, y))
        y.sum().backward()
        self.assertGreater(p.output.weight.grad.abs().sum().item(), 0)
        torch.nn.init.normal_(p.output.weight, std=.1)
        y = p(x, e, t, m, present, counts)
        order = torch.tensor([2, 0, 3, 1]); inverse = torch.argsort(order)
        torch.testing.assert_close(y[:, order], p(x[:, order], inverse[e], t, m, present[:, order], counts[:, order]))
        torch.testing.assert_close(y, p(x, e.flip(1), t.flip(1), m, present, counts))
        ext = torch.cat((e, torch.tensor([[[99, -1]]])), 1)
        torch.testing.assert_close(y, p(x, ext, torch.cat((t, t[:, :1]), 1),
            torch.cat((m, torch.zeros(1, 1, dtype=torch.bool)), 1), present, counts))
        self.assertFalse(torch.allclose(y, p(x, e.repeat(1, 2, 1), t.repeat(1, 2), m.repeat(1, 2), present, counts)))
        self.assertFalse(torch.allclose(y, p(x, e, t, m, present, counts * 2)))
        self.assertFalse(torch.allclose(y, p(x, e.flip(-1), t, m, present, counts)))
        y.sum().backward()
        self.assertGreater(p.message.weight.grad.abs().sum().item(), 0)


if __name__ == '__main__':
    unittest.main()

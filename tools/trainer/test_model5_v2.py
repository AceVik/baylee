"""Revision2 numerical/card-count regressions; no CUDA context required."""
import unittest
import torch
import model4 as V4
import model5 as V5
from export_onnx4 import make_dummy_inputs


class Revision2Tests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(2)
        torch.manual_seed(31)
        self.cfg = V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64,
                             structure_width=16, relational_width=16, relational_revision=2)

    def graph(self, copies, grouped):
        n = 2 if grouped else copies + 1
        edges = (torch.tensor([[[1, 0]]]) if grouped else
                 torch.stack((torch.arange(1, n), torch.zeros(copies, dtype=torch.long)), -1)[None])
        types = torch.zeros(edges.shape[:2], dtype=torch.long)
        mask = torch.ones(edges.shape[:2], dtype=torch.bool)
        present = torch.ones(1, n, dtype=torch.bool)
        counts = torch.ones(1, n)
        if grouped:
            counts[0, 1] = copies
        return edges, types, mask, present, counts

    def test_weighted_card_counts_and_stable_large_fanout(self):
        p = V5.RelationalProcessorV2(self.cfg)
        for copies in (20, 100, 500):
            grouped, ungrouped = self.graph(copies, True), self.graph(copies, False)
            _, _, _, _, gg, gc = p.edge_statistics(*grouped)
            _, _, _, _, ug, uc = p.edge_statistics(*ungrouped)
            self.assertEqual(gg[0, 0, 0].item(), 1)
            self.assertEqual(ug[0, 0, 0].item(), copies)
            self.assertEqual(gc[0, 0, 0].item(), copies)
            torch.testing.assert_close(gc[0, 0], uc[0, 0], rtol=0, atol=0)
            x = torch.randn(1, 2, 32)
            expanded = torch.cat((x[:, :1], x[:, 1:].repeat(1, copies, 1)), 1)
            seen = []
            hook = p.update.register_forward_pre_hook(lambda m, args: seen.append(args[0].detach()))
            y = p(expanded, *ungrouped)
            hook.remove()
            self.assertTrue(torch.isfinite(y).all())
            # Weighted means of bounded gated messages stay bounded independent of fanout.
            for update in seen:
                self.assertLessEqual(update[..., 16:32].abs().max().item(), 1.000001)
            self.assertEqual(gc.dtype, torch.float32)

    def test_warmstart_rng_history_and_learning(self):
        torch.manual_seed(18)
        old = V4.PlayerNet4(V4.Config4(**{k:v for k,v in self.cfg.to_dict().items() if k in V4.Config4.__dataclass_fields__})).eval()
        rng = torch.get_rng_state()
        torch.manual_seed(18)
        new = V5.PlayerNet5(self.cfg).eval()
        self.assertTrue(torch.equal(rng, torch.get_rng_state()))
        new.load_state_dict(V5.warmstart_from_v4(old.state_dict(), new), strict=True)
        inputs = list(make_dummy_inputs(32, E=3, M_edges=2, C=3, P=2))
        with torch.no_grad():
            for k in (0, 1, 2, 4):
                inputs[-1].fill_(k)
                for first, second in zip(old(*inputs), new(*inputs)):
                    self.assertTrue(torch.equal(first, second))
        p = new.relational
        x = torch.randn(1, 2, 32)
        graph = self.graph(20, True)
        opt = torch.optim.SGD(p.parameters(), lr=.1)
        p(x, *graph).square().sum().backward()
        self.assertGreater(p.output.weight.grad.abs().sum().item(), 0)
        opt.step(); opt.zero_grad()
        p(x, *graph).square().sum().backward()
        self.assertGreater(p.message.weight.grad.abs().sum().item(), 0)

    def test_padding_masks_and_permutation(self):
        p = V5.RelationalProcessorV2(self.cfg)
        torch.nn.init.normal_(p.output.weight, std=.1)
        x = torch.randn(1, 4, 32)
        e = torch.tensor([[[0, 1], [1, 2], [2, 3]]]); t = torch.tensor([[0, 2, 5]])
        m = torch.ones(1, 3, dtype=torch.bool); present = torch.ones(1, 4, dtype=torch.bool)
        counts = torch.tensor([[1., 20., 1., 2.]])
        y = p(x, e, t, m, present, counts)
        order = torch.tensor([2, 0, 3, 1]); inverse = torch.argsort(order)
        torch.testing.assert_close(y[:, order], p(x[:, order], inverse[e], t, m, present[:, order], counts[:, order]))
        xp = torch.cat((x, torch.randn(1, 2, 32)), 1)
        ep = torch.cat((e, torch.tensor([[[4, 1], [5, 2], [99, -1]]])), 1)
        yp = p(xp, ep, t.repeat(1, 2), m.repeat(1, 2),
               torch.cat((present, torch.zeros(1, 2, dtype=torch.bool)), 1), torch.ones(1, 6).index_copy(1, torch.arange(4), counts))
        torch.testing.assert_close(y, yp[:, :4])

    def test_checkpointed_v2_gradients(self):
        import copy
        ordinary = V5.PlayerNet5(self.cfg).train()
        torch.nn.init.normal_(ordinary.relational.output.weight, std=.1)
        checked = copy.deepcopy(ordinary)
        checked.cfg.activation_checkpointing = True
        x = torch.randn(1, 12, 32)
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

    def test_identity_and_old_config_inference(self):
        old = V5.Config5()
        self.assertEqual(old.relational_revision, 1)
        self.assertEqual(V5.architecture_for_config(old), 'baylee_ai_v5_relational_v1')
        self.assertEqual(V5.architecture_for_config(self.cfg), 'baylee_ai_v5_relational_v2')
        with self.assertRaises(ValueError):
            V5.Config5(relational_revision=3)


if __name__ == '__main__':
    unittest.main()

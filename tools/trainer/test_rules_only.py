"""`--train-rules-only` moves the rule encoder and nothing else; `--anchor-l2`
holds trainable weights near where they started."""
import unittest

import torch
import model5 as V5
from export_onnx4 import make_dummy_inputs
from test_structure4 import small_forest
from training4.optimizer_groups import build_optimizer_groups
from training4.rules_only import ParameterAnchor, freeze_all_but_rules


class RulesOnlyTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(2207)
        self.net = V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64,
                                            structure_width=16, relational_width=16,
                                            relational_revision=2)).eval()
        self.net.attach_structure(small_forest())
        self.inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=6, P=2))
        entity_rules = self.inputs[-4]
        entity_rules[..., 0, 0] = 3
        entity_rules[..., 0, 1] = 1

    def train(self, anchor_weight=0.0, steps=5):
        freeze_all_but_rules(self.net)
        anchor = ParameterAnchor(self.net, anchor_weight) if anchor_weight else None
        groups, _ = build_optimizer_groups(self.net, 5e-2, architecture="v5", decay=False)
        optimizer = torch.optim.AdamW(groups)
        for _ in range(steps):
            optimizer.zero_grad(set_to_none=True)
            loss = self.net(*self.inputs)[1][..., 0].sum()
            if anchor is not None:
                loss = loss + anchor.penalty()
            loss.backward()
            optimizer.step()
        return anchor

    def test_only_the_rule_encoder_moves(self):
        before = {name: value.clone() for name, value in self.net.state_dict().items()}
        self.train()
        after = self.net.state_dict()
        moved = {name for name, value in before.items() if not torch.equal(value, after[name])}
        self.assertTrue(moved, "the encoder learnt")
        self.assertTrue(all(name.startswith("rules.") for name in moved), moved)

    def test_the_anchor_shortens_the_way_from_the_start(self):
        start = {n: p.clone() for n, p in self.net.named_parameters()}
        self.train()
        free = float(sum((p - start[n]).square().sum() for n, p in self.net.named_parameters()).detach())
        with torch.no_grad():
            for n, p in self.net.named_parameters():
                p.copy_(start[n])
        anchor = self.train(anchor_weight=10.0)
        held = anchor.distance() ** 2
        self.assertLess(held, free)


if __name__ == "__main__":
    unittest.main()

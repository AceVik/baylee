"""`--train-candidate-column`: one column learns, nothing else moves, and a
candidate with the slot at zero scores as the initial network does."""
import io
import unittest

import torch
import model5 as V5
from export_onnx4 import make_dummy_inputs
from training4.candidate_column import carries_column, freeze_all_but_column, weight_of
from training4.optimizer_groups import build_optimizer_groups

SLOT = 12


class CandidateColumnTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(1912)
        self.net = V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64,
                                            structure_width=16, relational_width=16,
                                            relational_revision=2)).eval()
        self.inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=6, P=2))
        self.index = next(i for i, t in enumerate(self.inputs)
                          if t.dim() == 3 and t.shape[-1] == V5.CAND_FEAT_WIDTH)

    def train(self, steps=3):
        freeze_all_but_column(self.net, SLOT, start_from_zero=True)
        groups, _ = build_optimizer_groups(self.net, 1e-2, architecture="v5", decay=False)
        optimizer = torch.optim.AdamW(groups)
        for _ in range(steps):
            optimizer.zero_grad(set_to_none=True)
            logits = self.net(*self.inputs)[1]
            logits[..., 0].sum().backward()
            optimizer.step()

    def with_slot(self, value):
        self.inputs[self.index][..., SLOT] = value

    def test_only_the_column_moves(self):
        self.with_slot(1.0)
        before = {name: value.clone() for name, value in self.net.state_dict().items()}
        before[f"cand_proj.0.weight"][:, SLOT] = 0
        self.train()
        after = self.net.state_dict()
        for name, value in before.items():
            if name == "cand_proj.0.weight":
                moved = value[:, SLOT] != after[name][:, SLOT]
                self.assertTrue(moved.any(), "the column learnt")
                other = list(range(value.shape[1]))
                other.remove(SLOT)
                self.assertTrue(torch.equal(value[:, other], after[name][:, other]))
            else:
                self.assertTrue(torch.equal(value, after[name]), name)

    def test_a_zero_slot_gets_no_gradient(self):
        self.with_slot(0.0)
        self.train(1)
        self.assertTrue(torch.equal(weight_of(self.net)[:, SLOT],
                                    torch.zeros_like(weight_of(self.net)[:, SLOT])))

    def test_a_zero_slot_scores_as_the_initial_network(self):
        self.with_slot(0.0)
        with torch.no_grad():
            reference = self.net(*self.inputs)[1].clone()
        self.with_slot(1.0)
        self.train()
        self.with_slot(0.0)
        with torch.no_grad():
            self.assertTrue(torch.equal(reference, self.net(*self.inputs)[1]))

    def test_a_trained_column_round_trips_through_a_checkpoint(self):
        self.with_slot(1.0)
        self.train()
        buffer = io.BytesIO()
        torch.save(self.net.state_dict(), buffer)
        buffer.seek(0)
        restored = torch.load(buffer, weights_only=True)
        for name, value in self.net.state_dict().items():
            self.assertTrue(torch.equal(value, restored[name]), name)

    def test_a_batch_without_the_slot_has_nothing_to_give(self):
        feat = torch.zeros(2, 3, 4, V5.CAND_FEAT_WIDTH)
        self.assertFalse(carries_column({"cand_feat": feat}, SLOT))
        feat[1, 2, 3, SLOT] = -1.0
        self.assertTrue(carries_column({"cand_feat": feat}, SLOT))


if __name__ == "__main__":
    unittest.main()

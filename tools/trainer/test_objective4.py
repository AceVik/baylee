"""A forced acknowledgement cannot dilute learning a meaningful decision."""
import unittest
import torch
from test_training_execution4 import batch_fixture
from training4.loss import compute_step_loss
from training4.device_loss import compute_device_loss
from training4.execution import plan_batch
from training4.objective import LOSS_VERSION, check_resume_objective, selection_score, choice_statistics


class ObjectiveTests(unittest.TestCase):
    def test_forced_rows_do_not_dilute_choice_loss_or_gradient(self):
        for optimized in (False, True):
            observed = []
            for lanes in (1, 8):
                batch = batch_fixture(b=lanes, u=1)
                batch["step_valid"].fill_(True)
                batch["is_decision"].fill_(True)
                batch["head"].zero_(); batch["single_idx"].zero_()
                batch["loss_weight"].fill_(1)
                batch["cand_feat"].zero_(); batch["cand_mask"].zero_()
                batch["cand_mask"][:, :, 0] = True
                batch["cand_mask"][0, :, 1] = True
                raw = torch.tensor([[0., 1.]]).repeat(lanes, 1).requires_grad_()
                logits = raw.masked_fill(~batch["cand_mask"][:, 0, :2], -1e9)
                args = [logits, torch.zeros(lanes, 4), torch.zeros(lanes),
                        batch["outcome"][:, 0], batch["aux_target"][:, 0], batch, 0]
                loss, stats = (compute_device_loss(*args, plan_batch(batch, 0)[0]) if optimized
                               else compute_step_loss(*args))
                loss.backward()
                observed.append((loss.detach(), raw.grad[0].clone()))
                if lanes > 1:
                    torch.testing.assert_close(raw.grad[1:], torch.zeros_like(raw.grad[1:]))
            for a, b in zip(*observed):
                torch.testing.assert_close(a, b)

    def test_policy_loss_is_symmetric_under_candidate_relabeling_and_binary_complement(self):
        # Two distinct rows are required: a normalized row weight cancels for B=1.
        for optimized in (False, True):
            for head in (0, 1, 2):
                results = []
                for flip in (False, True):
                    batch = batch_fixture(b=2, u=1)
                    batch["step_valid"].fill_(True)
                    batch["is_decision"].fill_(True)
                    batch["head"].fill_(head)
                    batch["loss_weight"].fill_(1)
                    batch["outcome"].fill_(.5)
                    prefix = "pair" if head == 2 else "cand"
                    batch[prefix + "_mask"].fill_(True)
                    batch[prefix + "_free_mask"].fill_(1)
                    batch[prefix + "_target_mask"].zero_()
                    batch[prefix + "_target_mask"][0].fill_(1)
                    batch["single_idx"][:, 0] = torch.tensor([0, 1])
                    width = batch[prefix + "_mask"].shape[-1]
                    raw = torch.arange(2 * width, dtype=torch.float32).reshape(2, width) / 3
                    if flip:
                        if head == 0:
                            raw = raw.flip(-1)
                            batch["single_idx"] = width - 1 - batch["single_idx"]
                        else:
                            raw = -raw
                            batch[prefix + "_target_mask"] = 1 - batch[prefix + "_target_mask"]
                    cand = raw if head != 2 else torch.zeros(2, batch["cand_mask"].shape[-1])
                    pair = raw if head == 2 else torch.zeros(2, batch["pair_mask"].shape[-1])
                    args = [cand, pair, torch.zeros(2), batch["outcome"][:, 0],
                            batch["aux_target"][:, 0], batch, 0]
                    loss, _ = (compute_device_loss(*args, plan_batch(batch, 0)[0]) if optimized
                               else compute_step_loss(*args))
                    results.append(loss)
                torch.testing.assert_close(*results)

    def test_choice_metrics_include_optional_pass_and_exclude_forced_nonpass(self):
        logits = torch.tensor([[2., 0.], [2., -1e9], [0., 3.]])
        mask = torch.tensor([[True, True], [True, False], [True, True]])
        stats = choice_statistics(logits, mask, torch.ones(3, dtype=torch.bool),
                                  torch.tensor([0, 0, 0]), torch.ones(3))
        self.assertEqual(stats[1:].tolist(), [1., 2.])

    def test_better_value_cannot_outvote_worse_policy_by_default(self):
        before = dict(val_pol_loss=.3, val_mse=.2)
        after = dict(val_pol_loss=.4, val_mse=.01)
        self.assertLess(selection_score(before, "policy"), selection_score(after, "policy"))
        self.assertGreater(selection_score(before, "combined"), selection_score(after, "combined"))
        with self.assertRaisesRegex(ValueError, "loss semantics"):
            check_resume_objective({}, "policy")
        saved = dict(training_loss_version=LOSS_VERSION, checkpoint_selection="policy")
        check_resume_objective(saved, "policy")
        with self.assertRaisesRegex(ValueError, "selection differs"):
            check_resume_objective(saved, "combined")


if __name__ == "__main__":
    unittest.main()

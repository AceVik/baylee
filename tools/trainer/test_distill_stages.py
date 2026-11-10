"""Bundle distillation and partial training: contract, loss terms, metric, selection, freezing."""
import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import torch

import model4 as M
from test_training_execution4 import batch_fixture
from training4 import distill as D
from training4.deferred import deferred_readouts
from training4.execution import encode_boards, plan_batch
from training4.objective import selection_score
from training4.partial import (LowRankAdapter, apply_partial_training, state_dict_for_save, upstream_frozen)

CPU = torch.device("cpu")


def flags(**overrides):
    values = dict(train_policy_head_only=False, adapter_on_candidate_head=0, trainable_trunk_from_layer=-1,
                  freeze_value_head=False)
    values.update(overrides)
    return SimpleNamespace(**values)


def tiny_net(layers=2, seed=3):
    torch.manual_seed(seed)
    net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=layers, d_ff=64))
    net.decision_chunk = 2
    net.prune_unused_policy_heads = True
    return net


def distill_batch():
    """Lanes 0-1 single choices (mostly priority), 2 subsets, 3 pairs; three frames."""
    b, u = 4, 3
    batch = batch_fixture(b=b, u=u)
    batch["step_valid"][:] = True
    batch["is_decision"][:] = True
    batch["loss_weight"] = torch.ones(b, u)
    batch["head"] = torch.tensor([[0, 0, 0], [0, 0, 0], [1, 1, 1], [2, 2, 2]])
    batch["pending_kind"] = torch.tensor([[0, 0, 8], [0, 0, 0], [7, 7, 7], [6, 6, 6]])
    batch["n_ne_r"] = torch.tensor([[True, False, True], [False, True, False], [True, False, False], [False, True, False]])
    batch["n_single_idx"] = torch.full((b, u), -1)
    batch["cand_mask"][:] = False
    batch["cand_mask"][..., :4] = True
    batch["single_idx"] = torch.tensor([[1, 2, 3], [0, 3, 1], [0, 0, 0], [0, 0, 0]])
    batch["cand_free_mask"] = batch["cand_mask"].float()
    batch["cand_target_mask"] = torch.zeros_like(batch["cand_free_mask"])
    batch["cand_target_mask"][2, :, 1] = 1.
    batch["pair_mask"][:] = True
    batch["pair_free_mask"] = batch["pair_mask"].float()
    batch["pair_target_mask"] = torch.zeros_like(batch["pair_free_mask"])
    batch["pair_target_mask"][3, :, 2] = 1.
    return batch


def outputs_from(cand, pair, scalar):
    """Deferred-readout-shaped outputs from [B, U, C], [B, U, P] and [B, U] logits."""
    b, u = scalar.shape
    return [(torch.zeros(b, 9, 32), cand[:, t], pair[:, t], scalar[:, t], torch.full((b,), .5)) for t in range(u)]


def known_student():
    """Single choices: agree at (0,0) (0,2) (1,0) (1,2); subsets agree at t0, t2; pairs at t0, t1."""
    cand = torch.full((4, 3, 7), -3.)
    for lane, t, index in ((0, 0, 1), (0, 1, 0), (0, 2, 3), (1, 0, 0), (1, 1, 1), (1, 2, 1)):
        cand[lane, t, index] = 3.
    cand[2, 0, 1] = cand[2, 2, 1] = 3.   # subset decoded {1} = target at t0, t2
    cand[2, 1, 1:3] = 3.                  # {1, 2} != target at t1
    pair = torch.full((4, 3, 4), -3.)
    pair[3, 0, 2] = pair[3, 1, 2] = 3.   # {2} = target at t0, t1
    pair[3, 2, :] = 3.                    # everything at t2
    return outputs_from(cand, pair, torch.full((4, 3), .5))


class DistillMetric(unittest.TestCase):
    def test_agreement_counts_by_group_and_at_n_ne_r(self):
        batch = distill_batch()
        table = D.agreement(known_student(), batch, plan_batch(batch, 0), (0,), .05).tolist()
        groups = dict(zip(D.GROUPS, table))
        # distilled priority: (0,0)n (0,1) (1,0) (1,1)n (1,2); agree (0,0) (1,0) (1,2).
        self.assertEqual(groups["distilled"], [5, 2, 3, 1])
        self.assertEqual(groups["single"], [1, 1, 1, 1])  # (0,2) is ChooseTargets
        self.assertEqual(groups["subset"], [3, 1, 2, 1])
        self.assertEqual(groups["pair"], [3, 1, 2, 1])
        self.assertEqual(groups["scalar"], [0, 0, 0, 0])

    def test_selection_uses_agreement_at_n_ne_r_not_the_loss(self):
        better = dict(val_distill_selection=.62, val_distill_ce_r=.9, val_pol_loss=.9)
        worse = dict(val_distill_selection=.55, val_distill_ce_r=.1, val_pol_loss=.1)
        self.assertLess(selection_score(better, "distill_n_ne_r"), selection_score(worse, "distill_n_ne_r"))
        self.assertEqual(selection_score(dict(val_distill_selection=None), "distill_n_ne_r"), float("inf"))
        self.assertLess(selection_score(worse, "policy"), selection_score(better, "policy"))


class DistillLoss(unittest.TestCase):
    def test_terms_route_by_kind_and_n_ne_r(self):
        batch = distill_batch()
        plans = plan_batch(batch, 0)
        student = known_student()
        loss, stats = D.distill_loss(student, copy.deepcopy(student), batch, plans, (0,), 1.)
        means = D.term_means(stats)
        for name in D.TERMS[1:]:
            if means[name] is not None:
                self.assertAlmostEqual(means[name], 0., places=6, msg=name)
        # CE only at distilled N != R: (0,0) R=1 and (1,1) R=3, over the four valid candidates.
        logp = lambda lane, t, i: torch.log_softmax(student[t][1][lane, :4], -1)[i]
        expected = -(logp(0, 0, 1) + logp(1, 1, 3)) / 2
        self.assertAlmostEqual(means["ce_r"], float(expected), places=5)
        self.assertAlmostEqual(float(loss), float(expected), places=5)
        self.assertEqual(stats[len(D.TERMS):].tolist()[:3], [2., 3., 1.])

    def test_reference_moves_only_its_own_terms(self):
        batch = distill_batch()
        plans = plan_batch(batch, 0)
        student = known_student()
        base = D.term_means(D.distill_loss(student, copy.deepcopy(student), batch, plans, (0,), 1.)[1])
        for lane, t, changed in ((0, 0, None), (0, 1, "kl_distilled_n_eq_r"), (0, 2, "kl_single")):
            reference = copy.deepcopy(student)
            reference[t][1][lane, :4] = torch.tensor([2., -1., .5, 0.])
            means = D.term_means(D.distill_loss(student, reference, batch, plans, (0,), 1.)[1])
            for name in D.TERMS:
                if name == changed:
                    self.assertGreater(means[name], 1e-3, name)
                elif means[name] is not None:
                    self.assertAlmostEqual(means[name], base[name], places=6, msg=name)

    def test_gradient_is_zero_where_student_matches_r1_and_n_equals_r(self):
        batch = distill_batch()
        plans = plan_batch(batch, 0)
        student = known_student()
        reference = copy.deepcopy(student)
        logits = [o[1].clone().requires_grad_() for o in student]
        student = [(o[0], logits[t], o[2], o[3], o[4]) for t, o in enumerate(student)]
        loss, _ = D.distill_loss(student, reference, batch, plans, (0,), 1.)
        loss.backward()
        # KL's gradient at equality is p_student - p_r1: zero up to rounding.
        torch.testing.assert_close(logits[1].grad[0], torch.zeros(7), rtol=0, atol=1e-7)  # priority, N = R
        self.assertGreater(float(logits[0].grad[0].abs().max()), 1e-3)                    # priority, N != R
        self.assertTrue(torch.equal(logits[0].grad[0, 4:], torch.zeros(3)))  # masked candidates


class DistillContract(unittest.TestCase):
    def test_labels_train_only_under_the_flag(self):
        plain, labeled = SimpleNamespace(meta={}), SimpleNamespace(meta={"bundle_distillation": {"contract": D.CONTRACT}})
        D.check_datasets((plain,), False)
        self.assertFalse(plain.distillation)
        D.check_datasets((labeled,), True)
        self.assertTrue(labeled.distillation)
        with self.assertRaisesRegex(ValueError, "only under --distill-bundle"):
            D.check_datasets((SimpleNamespace(meta={"bundle_distillation": {"contract": D.CONTRACT}}),), False)
        with self.assertRaisesRegex(ValueError, "needs bundle_distillation_v1"):
            D.check_datasets((plain,), True)

    def test_frames_carry_kind_and_n_ne_r_only_when_enabled(self):
        import dataset4
        dataset = object.__new__(dataset4.TrajectoryDataset4)
        self.assertEqual(set(dataset._with_distillation({"x": 1}, 128 | 1, 0, None)), {"x"})
        dataset.distillation = True
        frame = dataset._with_distillation({}, 128 | 4 | 1, 11, 3)
        self.assertEqual((int(frame["pending_kind"]), bool(frame["n_ne_r"]), int(frame["n_single_idx"])), (11, True, 3))
        frame = dataset._with_distillation({}, 64 | 8 | 1, 0, None)  # RL mixture bits are not N != R
        self.assertEqual((bool(frame["n_ne_r"]), int(frame["n_single_idx"])), (False, -1))


class PartialTraining(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def update(self, net, batch):
        plans = plan_batch(batch, 0)
        boards = encode_boards(net, batch, 2, CPU)
        out, _ = deferred_readouts(net, batch, boards, torch.zeros(4, 9, 32), plans, 2, CPU)
        loss = sum(o[1].float().square().mean() + o[2].float().square().mean() + o[3].square().mean()
                   + o[4].square().mean() for o in out) + net.aux_head(out[0][0][:, 0]).square().mean()
        optimizer = torch.optim.AdamW([p for p in net.parameters() if p.requires_grad], lr=1e-2)
        loss.backward()
        optimizer.step()
        return out

    def test_head_only_keeps_every_frozen_parameter_bit_identical(self):
        net = tiny_net()
        before = copy.deepcopy(net.state_dict())
        description = apply_partial_training(net, flags(train_policy_head_only=True))
        self.assertEqual(description["mode"], "policy_head")
        self.assertTrue(upstream_frozen(net))
        self.update(net, distill_batch())
        changed = {k for k, v in net.state_dict().items() if not torch.equal(v, before[k])}
        self.assertTrue(changed)
        self.assertTrue(all(k.startswith(("cand_proj.", "cand_scorer.", "cand_ctx_proj.")) for k in changed), changed)

    def test_upper_trunk_stops_gradient_below_its_layer(self):
        net = tiny_net(layers=2)
        before = copy.deepcopy(net.state_dict())
        apply_partial_training(net, flags(trainable_trunk_from_layer=1, freeze_value_head=True))
        self.assertFalse(upstream_frozen(net))
        self.update(net, distill_batch())
        frozen = ("trunk_layers.0.", "entity_proj.", "rules", "relational", "recurrent_cell.", "h_norm.",
                  "seat_value_head.", "aux_head.", "global_proj.", "dsl_proj.")
        for name, parameter in net.named_parameters():
            if name.startswith(frozen):
                self.assertIsNone(parameter.grad, name)
                self.assertTrue(torch.equal(parameter, before[name]), name)
        self.assertFalse(torch.equal(net.state_dict()["trunk_layers.1.ff.0.weight"], before["trunk_layers.1.ff.0.weight"]))

    def test_adapter_starts_exact_and_saves_as_the_plain_architecture(self):
        net = tiny_net()
        plain = copy.deepcopy(net)
        batch = distill_batch()
        apply_partial_training(net, flags(adapter_on_candidate_head=4))
        self.assertIsInstance(net.cand_scorer[0], LowRankAdapter)
        with torch.no_grad():
            plans = plan_batch(batch, 0)
            a = deferred_readouts(net, batch, encode_boards(net, batch, 2, CPU), torch.zeros(4, 9, 32), plans, 2, CPU)[0]
            b = deferred_readouts(plain, batch, encode_boards(plain, batch, 2, CPU), torch.zeros(4, 9, 32), plans, 2, CPU)[0]
        for x, y in zip(a, b):
            torch.testing.assert_close(x[1], y[1], rtol=0, atol=0)
        before = copy.deepcopy(plain.state_dict())
        self.update(net, batch)
        saved = state_dict_for_save(net)
        self.assertEqual(set(saved), set(before))
        changed = {k for k, v in saved.items() if not torch.equal(v, before[k])}
        self.assertEqual(changed, {"cand_scorer.0.weight"})
        merged = M.PlayerNet4(net.cfg)
        merged.load_state_dict(saved, strict=True)
        merged.decision_chunk = 2
        merged.prune_unused_policy_heads = True
        with torch.no_grad():
            a = deferred_readouts(net, batch, encode_boards(net, batch, 2, CPU), torch.zeros(4, 9, 32), plans, 2, CPU)[0]
            c = deferred_readouts(merged, batch, encode_boards(merged, batch, 2, CPU), torch.zeros(4, 9, 32), plans, 2, CPU)[0]
        for x, y in zip(a, c):
            torch.testing.assert_close(x[1], y[1], rtol=1e-5, atol=1e-5)

    def test_flags_off_change_nothing(self):
        net = tiny_net()
        states = {n: p.requires_grad for n, p in net.named_parameters()}
        self.assertIsNone(apply_partial_training(net, flags()))
        self.assertEqual(states, {n: p.requires_grad for n, p in net.named_parameters()})
        self.assertIs(state_dict_for_save(net).__class__, net.state_dict().__class__)


class ReferencePolicy(unittest.TestCase):
    def test_shared_trunk_reference_equals_its_own_forward(self):
        torch.set_num_threads(1)
        net = tiny_net()
        net.eval()
        copied = D.Reference.snapshot(net)
        apply_partial_training(net, flags(train_policy_head_only=True))
        shared = D.Reference(copied, net)
        self.assertTrue(shared.shared_trunk)
        separate = D.Reference(D.Reference.snapshot(net), net)
        separate.shared_trunk = False
        batch = distill_batch()
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        plans = plan_batch(batch, 0)
        with torch.no_grad():
            boards = encode_boards(net, batch, 2, CPU)
            hidden = torch.zeros(4, 9, 32)
            a = shared.outputs(batch, batch, boards, hidden, plans, CPU, False, 2)
            b = separate.outputs(batch, batch, boards, hidden, plans, CPU, False, 2)
        for x, y in zip(a, b):
            for p, q in zip(x, y):
                torch.testing.assert_close(p, q, rtol=1e-6, atol=1e-6)
        self.assertFalse(any(p.requires_grad for p in shared.net.parameters()))


class DistillOptions(unittest.TestCase):
    STAGE_A = ["train5", "--data", "d", "--out", "o", "--init", "i", "--train-policy-head-only", "--distill-bundle",
               "--distill-kinds", "0", "--distill-kl-weight", "1.0", "--checkpoint-selection", "distill_n_ne_r",
               "--anchor-l2", "0.15", "--lr", "6e-5", "--batch-size", "8", "--board-chunk", "32",
               "--board-token-budget", "32768", "--decision-chunk", "4", "--window-burn-in", "0",
               "--window-entity-buckets", "16", "--loss-version", "5", "--loss-normalization", "unroll_per_head"]

    def parse(self, argv):
        from training4.cli import parse_args
        with patch("sys.argv", argv), contextlib.redirect_stderr(io.StringIO()):
            return parse_args("v5")[0]

    def test_stage_a_recipe_parses_and_guards_reject_mixtures(self):
        args = self.parse(self.STAGE_A)
        self.assertEqual((args.distill_kinds, args.checkpoint_selection), ((0,), "distill_n_ne_r"))
        args = self.parse([*self.STAGE_A, "--distill-kinds", "0,11"])
        self.assertEqual(args.distill_kinds, (0, 11))
        for change in (["--adapter-on-candidate-head", "8"],                 # two stage-A modes
                       ["--freeze-trunk"],                                    # replaced by partial training
                       ["--checkpoint-selection", "policy"],                  # distillation selects on N != R
                       ["--decision-chunk", "1", "--window-burn-in", "-1"],   # needs the deferred path
                       ["--resume", "last.pt"]):                              # fresh --init only
            argv = [a for a in self.STAGE_A]
            if change[0] == "--checkpoint-selection":
                argv[argv.index("--checkpoint-selection") + 1] = change[1]
            elif change[0] == "--decision-chunk":
                argv[argv.index("--decision-chunk") + 1] = "1"
                argv[argv.index("--window-burn-in") + 1] = "-1"
            else:
                argv += change
            with self.subTest(change=change), self.assertRaises(SystemExit):
                self.parse(argv)
        with self.assertRaises(SystemExit):
            self.parse(["train5", "--data", "d", "--out", "o", "--checkpoint-selection", "distill_n_ne_r"])


class DistillValidation(unittest.TestCase):
    def test_untouched_student_has_zero_guards_and_n_rate_selection_inputs(self):
        torch.set_num_threads(1)
        net = tiny_net()
        copied = D.Reference.snapshot(net)
        apply_partial_training(net, flags(train_policy_head_only=True))
        reference = D.Reference(copied, net)
        batch = distill_batch()
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        metrics = D.evaluate_distill(net, reference, [batch, copy.deepcopy(batch)], CPU, (0,), 1., 2, .05, 2, bf16=False)
        for name in D.TERMS[1:]:
            if metrics[f"val_distill_{name}"] is not None:
                self.assertAlmostEqual(metrics[f"val_distill_{name}"], 0., places=6, msg=name)
        self.assertEqual(metrics["val_distill_distilled_decisions"], 10)
        self.assertEqual(metrics["val_distill_distilled_n_ne_r"], 4)
        self.assertAlmostEqual(metrics["val_distill_distilled_n_top1_r"], .6)
        self.assertEqual(metrics["val_distill_selection"], metrics["val_distill_distilled_top1_r_at_n_ne_r"])
        self.assertAlmostEqual(metrics["val_value_mse_to_r1"], 0., places=6)


class DriverStages(unittest.TestCase):
    def test_stage_a_driver_updates_only_the_head_and_saves_plain_checkpoints(self):
        from training4 import driver
        from model4.structure import structure_digest
        batch = distill_batch()
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        forest = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data
        meta = {}

        class Dataset:
            def __init__(self, *args, split, **kw):
                self.structure = SimpleNamespace(data=forest, digest=structure_digest(forest))
                self.fingerprint = split
                self.meta = meta

            def __len__(self):
                return 1

        class Sampler:
            VERSION = driver.D4.StatefulBatchSampler4.VERSION

            def __init__(self, *args, **kw):
                self.epoch = 0
                self.start_batch = 0

        common = ["--data", "fake", "--d-model", "32", "--layers", "2", "--heads", "4", "--d-ff", "64",
                  "--batch-size", "4", "--unroll", "3", "--board-chunk", "2", "--decision-chunk", "2", "--max-steps", "1",
                  "--epochs", "1", "--loss-version", "5", "--loss-normalization", "unroll_per_head", "--lr", "1e-2"]

        def run(output, extra, data_meta):
            meta.clear()
            meta.update(data_meta)
            argv = ["train5", "--out", str(output), *common, *extra]
            with patch("sys.argv", argv), patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                    patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                    patch.object(driver, "DataLoader", return_value=[]), \
                    patch.object(driver, "prefetch_batches", return_value=[(copy.deepcopy(batch), None)]), \
                    patch.object(driver, "evaluate_device", return_value=dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)), \
                    patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(io.StringIO()):
                driver.main("v5")
            return torch.load(output / "last.pt", map_location="cpu", weights_only=False)

        labeled = {"bundle_distillation": {"contract": D.CONTRACT}}
        distill = ["--distill-bundle", "--checkpoint-selection", "distill_n_ne_r", "--skip-final-validation"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initial = run(root / "init", [], {})["state_dict"]
            init = ["--init", str(root / "init")]
            with self.assertRaises(ValueError):
                run(root / "unlabeled", [*init, *distill, "--train-policy-head-only"], {})
            with self.assertRaises(ValueError):
                run(root / "unflagged", [*init], labeled)
            for name, extra, allowed in (
                    ("head", ["--train-policy-head-only"], ("cand_proj.", "cand_scorer.", "cand_ctx_proj.")),
                    ("adapter", ["--adapter-on-candidate-head", "4"], ("cand_scorer.0.weight",))):
                saved = run(root / name, [*init, *distill, *extra], labeled)
                self.assertEqual(set(saved["state_dict"]), set(initial), name)
                changed = {k for k, v in saved["state_dict"].items() if not torch.equal(v, initial[k])}
                self.assertTrue(changed, name)
                self.assertTrue(all(k.startswith(allowed) for k in changed), (name, changed))
                self.assertEqual(saved["checkpoint_selection"], "distill_n_ne_r")
                self.assertEqual(saved["bundle_distillation"]["kinds"], [0])


if __name__ == "__main__":
    unittest.main()

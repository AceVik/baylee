"""The house teacher (`--distill-teacher house`): contract, loss at every kind, held-out gate."""
import contextlib
import copy
import io
import json
import math
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import torch

import model4 as M
from test_training_execution4 import batch_fixture
import test_distill_stages as stages
from training4 import distill as D
from training4 import house as H
from training4.execution import plan_batch

TEACHER = {"level": "expert", "generation": 7, "foresight": "fair"}


def house_batch():
    """Lane 0 priority: t0 H taps / N passes, t1 H passes / N taps, t2 H taps 1 / N taps 2 (candidates:
    0 pass, 1 tap, 2 tap, 3 cast). Lane 1 single choices: t0 mulligan N != H, t1 target N = H, t2 target
    N != H. Lane 2 subsets (attackers): N != H, N = H, N != H. Lane 3: blocks N != H, N = H, then a
    number N != H."""
    b, u = 4, 3
    batch = batch_fixture(b=b, u=u)
    batch["step_valid"][:] = True
    batch["is_decision"][:] = True
    batch["is_new_traj"] = torch.ones(b, dtype=torch.bool)
    batch["loss_weight"] = torch.ones(b, u)
    batch["head"] = torch.tensor([[0, 0, 0], [0, 0, 0], [1, 1, 1], [2, 2, 3]])
    batch["pending_kind"] = torch.tensor([[0, 0, 0], [11, 8, 8], [5, 5, 5], [6, 6, 16]])
    batch["n_ne_r"] = torch.tensor([[True, True, True], [True, False, True], [True, False, True],
                                    [True, False, True]])
    batch["cand_mask"][:] = False
    batch["cand_mask"][..., :4] = True
    batch["single_idx"] = torch.tensor([[1, 0, 1], [2, 3, 1], [9999] * 3, [9999] * 3])
    batch["coarse_class"] = torch.full((b, u, 7), -1)
    batch["coarse_class"][0, :, :4] = torch.tensor([0, 1, 1, 2])
    batch["coarse_n_idx"] = torch.full((b, u), -1)
    batch["coarse_n_idx"][0] = torch.tensor([0, 1, 2])
    batch["cand_free_mask"] = batch["cand_mask"].float()
    batch["cand_target_mask"] = torch.zeros_like(batch["cand_free_mask"])
    batch["cand_target_mask"][2, 0, 1] = 1.
    batch["cand_target_mask"][2, 2, 1:3] = 1.
    batch["pair_mask"][:] = True
    batch["pair_free_mask"] = batch["pair_mask"].float()
    batch["pair_target_mask"] = torch.zeros_like(batch["pair_free_mask"])
    batch["pair_target_mask"][3, 0, 2] = 1.
    batch["scalar_target"] = torch.zeros(b, u)
    batch["scalar_target"][3, 2] = .75
    batch["distill_subtype"] = torch.full((b, u), -1)
    batch["fired_mask"] = torch.zeros(b, u, dtype=torch.long)
    return batch


def outputs(cand, pair, scalar, value=.5):
    b, u = scalar.shape
    return [(torch.zeros(b, 9, 32), cand[:, t], pair[:, t], scalar[:, t], torch.full((b,), value)) for t in range(u)]


def student_logits():
    cand = torch.zeros(4, 3, 7)
    cand[0, 0, :4] = torch.tensor([0., .5, .2, -1.])   # H's tap leads the acts, pass close
    cand[0, 1, :4] = torch.tensor([0., 1., 0., 0.])    # N's tap leads, H passes
    cand[2, 0, :4] = torch.tensor([-1., 1., -1., -1.])  # decodes {1} = H
    pair = torch.zeros(4, 3, 4)
    pair[3, 0] = torch.tensor([-1., -1., 1., -1.])      # decodes {2} = H
    return cand, pair, torch.full((4, 3), .5)


SCOPE = D.Scope(kinds=tuple(range(D.KIND_SLOTS)), teacher="house")


class HouseLoss(unittest.TestCase):
    def loss(self, scope=SCOPE, value=.5):
        cand, pair, scalar = student_logits()
        cand.requires_grad_()
        reference_cand = torch.zeros(4, 3, 7)
        reference_cand[1, 1, :4] = torch.tensor([0., 0., 0., 2.])   # r1 at the N = H target
        student = outputs(cand, pair, scalar)
        reference = outputs(reference_cand, torch.zeros(4, 3, 4), torch.full((4, 3), .25), value)
        batch = house_batch()
        loss, stats = D.distill_loss(student, reference, batch, plan_batch(batch, 0), scope)
        return loss, stats, cand

    def test_every_kind_has_its_term_and_rows(self):
        _, stats, _ = self.loss()
        n = len(D.TERMS)
        weight = lambda name: stats[n + D.TERMS.index(name)].item()
        mean = lambda name: D.term_means(stats)[name]
        self.assertEqual(stats.shape, (D.STATS_WIDTH,))
        self.assertEqual({name: weight(name) for name in H.H_TERMS + H.R1_TERMS + ("sq_value_r1",)},
                         dict(ce_h_group=2., ce_h_within=2., ce_h_single=2., bce_h_subset=8., bce_h_pair=4.,
                              sq_h_scalar=1., kl_distilled_n_eq_r=1., kl_subset=4., kl_pair=4., sq_scalar=0.,
                              sq_value_r1=12.))
        # Priority: H's tap against the pass group (act-best), the pass group where H passes; inside H's
        # group its exact candidate (t0 among three acts, t2 among the same three).
        group_t0 = math.log(1 + math.exp(-.5))
        group_t1 = math.log(3 + math.e)
        self.assertAlmostEqual(mean("ce_h_group"), (group_t0 + group_t1) / 2, places=5)
        within_t0 = -math.log(math.exp(.5) / (math.exp(.5) + math.exp(.2) + math.exp(-1.)))
        self.assertAlmostEqual(mean("ce_h_within"), (within_t0 + math.log(3.)) / 2, places=5)
        self.assertAlmostEqual(mean("ce_h_single"), math.log(4.), places=5)   # mulligan and target, uniform
        self.assertAlmostEqual(mean("sq_h_scalar"), .0625, places=6)
        # No bundle term moves.
        for name in ("ce_r", "ce_coarse_r", "ce_fine_n", "ce_n_out_of_scope", "kl_kind_out_of_scope", "kl_single"):
            self.assertEqual(weight(name), 0., name)
        kinds = D.kind_means(stats)
        self.assertEqual({name: row[1] for name, row in kinds.items()},
                         dict(priority=3, mulligan=1, targets=1, attackers=2, blockers=1, number=1))
        self.assertAlmostEqual(kinds["priority"][0], (group_t0 + within_t0 + group_t1 + math.log(3.)) / 3, places=5)
        line = D.train_line(stats)
        self.assertEqual(line["kinds"]["mulligan"][1], 1)

    def test_total_is_the_h_terms_plus_the_weighted_guards(self):
        for kl_weight, value_weight in ((1., 0.), (.5, 2.)):
            scope = D.Scope(**{**SCOPE.__dict__, "kl_weight": kl_weight, "value_weight": value_weight})
            loss, stats, _ = self.loss(scope, value=.75)
            means = D.term_means(stats)
            expected = (sum(means[name] or 0. for name in H.H_TERMS)
                        + kl_weight * sum(means[name] or 0. for name in H.R1_TERMS)
                        + value_weight * means["sq_value_r1"])
            self.assertAlmostEqual(loss.item(), expected, places=5)
            self.assertAlmostEqual(means["sq_value_r1"], .0625, places=6)

    def test_where_n_equals_h_only_the_kl_moves(self):
        loss, _, cand = self.loss()
        loss.backward()
        # r1 prefers the target H also answers; the KL pulls toward r1 there, not toward H's index.
        grad = cand.grad[1, 1, :4]
        self.assertLess(grad[3].item(), 0.)
        # The tap H takes rises; the pass N took falls (act-best and the exact candidate inside).
        self.assertLess(cand.grad[0, 0, 1].item(), 0.)
        self.assertGreater(cand.grad[0, 0, 0].item(), 0.)
        # H passes: the pass rises.
        self.assertLess(cand.grad[0, 1, 0].item(), 0.)

    def test_contract_and_options(self):
        bundle = SimpleNamespace(meta={"bundle_distillation": {"contract": D.CONTRACT, "subtypes": [], "groups": []}})
        house = SimpleNamespace(meta={"bundle_distillation": {"contract": D.HOUSE_CONTRACT, "subtypes": [],
                                                              "groups": [], "teacher": TEACHER}})
        D.check_datasets([house], True, "house")
        D.check_datasets([bundle], True, "bundle")
        for dataset, teacher in ((house, "bundle"), (bundle, "house")):
            with self.subTest(teacher=teacher), self.assertRaises(ValueError):
                D.check_datasets([dataset], True, teacher)
        with self.assertRaises(ValueError):
            D.check_datasets([house], False)
        for bad in (dict(subtypes=("cast",)), dict(coarse="act"), dict(out_of_scope="ce-n")):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                D.Scope.from_options((0,), 1., bad.get("subtypes", ()), {}, [house], teacher="house",
                                     **{k: v for k, v in bad.items() if k != "subtypes"})
        with self.assertRaises(ValueError):
            D.Scope.from_options((0,), 1., (), {}, [bundle], value_weight=1.)

        parse = stages.DistillOptions().parse
        stage_a = [a for a in stages.DistillOptions.STAGE_A if a not in ("--anchor-l2", "0.15")]
        args = parse([*stage_a, "--distill-teacher", "house", "--train-pair-scalar-heads"])
        self.assertEqual((args.distill_teacher, args.train_pair_scalar_heads), ("house", True))
        arm_b = [a for a in stage_a if a != "--train-policy-head-only"]
        args = parse([*arm_b, "--distill-teacher", "house", "--trainable-trunk-from-layer", "8", "--freeze-value-head",
                      "--distill-value-weight", "1"])
        self.assertEqual(args.distill_value_weight, 1.)
        for bad in (["--distill-teacher", "house", "--distill-subtypes", "cast"],
                    ["--distill-teacher", "house", "--distill-coarse", "act"],
                    ["--distill-teacher", "house", "--pregate-chain-start", "0.8"],
                    ["--distill-value-weight", "1"], ["--distill-teacher", "house", "--distill-value-weight", "-1"]):
            with self.subTest(bad=bad), self.assertRaises(SystemExit):
                parse([*stage_a, *bad])
        with self.assertRaises(SystemExit):
            parse([*arm_b, "--train-pair-scalar-heads"])
        with self.assertRaises(SystemExit):
            parse([a for a in stage_a if a != "--distill-bundle"] + ["--distill-teacher", "house",
                                                                    "--checkpoint-selection", "policy"])


class HouseGate(unittest.TestCase):
    def test_agreement_per_kind_for_the_student_and_r1(self):
        batch = house_batch()
        cand, pair, scalar = student_logits()
        student = outputs(cand, pair, scalar)
        reference = outputs(torch.zeros(4, 3, 7), torch.zeros(4, 3, 4), torch.full((4, 3), .75))
        counter = H.HouseCounter(.05)
        counter.observe(student, reference, batch, plan_batch(batch, 0))
        metrics = counter.metrics()
        self.assertEqual((metrics["val_house_n_ne_h"], metrics["val_house_n_eq_h"]), (9, 3))
        self.assertAlmostEqual(metrics["val_house_top1_n_ne_h"], 3 / 9)
        by_kind = metrics["val_house_top1_n_ne_h_by_kind"]
        self.assertEqual({name: (row["n_ne_h"], row["student"]) for name, row in by_kind.items()},
                         dict(priority=(3, 1 / 3), mulligan=(1, 0.), targets=(1, 0.), attackers=(2, .5),
                              blockers=(1, 1.), number=(1, 0.)))
        # r1 (zeros) passes at t1 where H passes, and r1's number is H's.
        self.assertEqual((by_kind["priority"]["r1"], by_kind["number"]["r1"]), (1 / 3, 1.))
        self.assertFalse(metrics["val_house_gate"]["passed"])
        # The drift watch where N = H, per kind, and in the epoch line.
        self.assertEqual(metrics["val_house_top1_n_eq_h_by_kind"], dict(targets=0., attackers=1., blockers=1.))
        epoch = json.loads(D.epoch_line(1, 5, None, metrics).split(" ", 2)[2])
        self.assertEqual(epoch["validation"]["val_house_top1_n_eq_h_by_kind"]["attackers"], 1.)

    def test_gate_reads_top1_and_the_chain_start_rate_against_h(self):
        good = dict(val_house_top1_n_ne_h=.72, val_house_chain_start_act=.55, val_house_chain_start_act_h=.6)
        self.assertTrue(H.house_gate(good)[0])
        for name, value in (("val_house_top1_n_ne_h", .69), ("val_house_chain_start_act", .53),
                            ("val_house_chain_start_act_h", None)):
            with self.subTest(name=name):
                self.assertFalse(H.house_gate({**good, name: value})[0])
        verdict = H.house_gate(good)[1]
        self.assertAlmostEqual(verdict["chain_start_act_ratio"], .55 / .6)
        self.assertEqual((verdict["chain_start_act"], verdict["chain_start_act_h"]), (.55, .6))
        self.assertFalse(verdict["more_active_than_teacher"])
        # One-sided: a student far more active than H passes, flagged for the play check.
        passed, verdict = H.house_gate({**good, "val_house_chain_start_act": .8})
        self.assertTrue(passed and verdict["more_active_than_teacher"])

    def test_chain_starts_where_h_could_start_one_and_pass_shares(self):
        """H answers pass, tap, cast, pass, pass, cast: chances to start a chain at t0, t1, t4, t5
        (t2 continues, t3 ends one); H starts two. The student acts at t0, t1, t5; N always passes."""
        u = 6
        batch = batch_fixture(b=1, u=u)
        batch["step_valid"][:] = True
        batch["is_decision"][:] = True
        batch["is_new_traj"] = torch.ones(1, dtype=torch.bool)
        batch["loss_weight"] = torch.ones(1, u)
        batch["head"] = torch.zeros(1, u, dtype=torch.long)
        batch["pending_kind"] = torch.zeros(1, u, dtype=torch.long)
        batch["cand_mask"][:] = False
        batch["cand_mask"][..., :4] = True
        batch["single_idx"] = torch.tensor([[0, 1, 3, 0, 0, 3]])
        batch["coarse_class"] = torch.full((1, u, 7), -1)
        batch["coarse_class"][..., :4] = torch.tensor([0, 1, 1, 2])
        batch["coarse_n_idx"] = torch.zeros(1, u, dtype=torch.long)
        batch["n_ne_r"] = batch["single_idx"] != 0
        batch["distill_subtype"] = torch.full((1, u), -1)
        cand = torch.zeros(1, u, 7)
        cand[0, :, 0] = 1.                                    # the student passes ...
        cand[0, [0, 1, 5], 3] = 2.                            # ... except at t0, t1 and t5
        student = outputs(cand, torch.zeros(1, u, 4), torch.zeros(1, u))
        reference = outputs(torch.zeros(1, u, 7), torch.zeros(1, u, 4), torch.zeros(1, u))
        chains = D.ChainCounter(D.Scope(kinds=(0,)))
        chains.observe(student, reference, batch, plan_batch(batch, 0))
        metrics = H.HouseCounter(.05).metrics(chains.metrics())
        self.assertEqual(metrics["val_house_chain_start_chances"], 4)
        self.assertEqual((metrics["val_house_chain_start_act"], metrics["val_house_chain_start_act_h"],
                          metrics["val_house_chain_start_act_r1"]), (.75, .5, 0.))
        self.assertEqual((metrics["val_house_priority_pass_share"], metrics["val_house_priority_pass_share_h"],
                          metrics["val_house_priority_pass_share_r1"]), (.5, .5, 1.))
        self.assertEqual(metrics["val_house_chain_completion"], .5)   # passes mid-way through the first chain


class HouseDriver(unittest.TestCase):
    def test_both_arms_train_every_head_they_hold_and_print_every_kind(self):
        from training4 import driver
        from model4.structure import structure_digest
        batch = stages.distill_batch()
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        # Blocks where N != H at t1, so the pair head has a target.
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
        logs = {}

        def run(output, extra, data_meta):
            meta.clear()
            meta.update(data_meta)
            argv = ["train5", "--out", str(output), *common, *extra]
            stdout = logs[output.name] = io.StringIO()
            with patch("sys.argv", argv), patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                    patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                    patch.object(driver, "DataLoader", return_value=[]), \
                    patch.object(driver, "prefetch_batches", return_value=[(copy.deepcopy(batch), None)]), \
                    patch.object(driver, "evaluate_device", return_value=dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)), \
                    patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(stdout):
                driver.main("v5")
            return torch.load(output / "last.pt", map_location="cpu", weights_only=False)

        labeled = {"bundle_distillation": {"contract": D.HOUSE_CONTRACT, "teacher": TEACHER}}
        distill = ["--distill-bundle", "--distill-teacher", "house", "--checkpoint-selection", "distill_n_ne_r",
                   "--skip-final-validation"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initial = run(root / "init", [], {})["state_dict"]
            init = ["--init", str(root / "init")]
            with self.assertRaises(ValueError):   # house labels under the bundle teacher
                run(root / "mismatch", [*init, "--distill-bundle", "--checkpoint-selection", "distill_n_ne_r",
                                        "--train-policy-head-only"], labeled)
            for name, extra, allowed, value_weight in (
                    ("a", ["--train-policy-head-only", "--train-pair-scalar-heads"],
                     ("cand_proj.", "cand_scorer.", "cand_ctx_proj.", "pair_", "scalar_head."), 0.),
                    ("b", ["--trainable-trunk-from-layer", "1", "--freeze-value-head", "--distill-value-weight", "1"],
                     ("trunk_layers.1.", "trunk_norm.", "thinking_block.", "cand_", "pair_", "scalar_head."), 1.)):
                saved = run(root / name, [*init, *distill, *extra], labeled)
                changed = {k for k, v in saved["state_dict"].items() if not torch.equal(v, initial[k])}
                self.assertTrue(all(k.startswith(allowed) for k in changed), (name, changed))
                self.assertTrue(any(k.startswith("pair_") for k in changed), (name, changed))
                house = saved["bundle_distillation"]
                self.assertEqual((house["contract"], house["house"]["teacher"], house["house"]["value_weight"]),
                                 (D.HOUSE_CONTRACT, TEACHER, value_weight))
                self.assertEqual(house["kinds"], list(range(D.KIND_SLOTS)))
                text = logs[name].getvalue()
                line = json.loads(text.split("[distill] train ", 1)[1].splitlines()[0])
                self.assertTrue(line.get("kinds"))
                epoch = json.loads(text.split("[distill] epoch ", 1)[1].splitlines()[0])
                self.assertIn("kinds", epoch["train"])


if __name__ == "__main__":
    unittest.main()

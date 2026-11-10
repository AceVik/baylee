"""The deliberation block in bundle distillation: arms A + block and B + block."""
import contextlib
import copy
import io
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import torch

import model5 as V5
from test_distill_stages import SUBTYPES, distill_batch, flags, scoped, with_subtypes
from training4 import distill as D
from training4.deferred import deferred_readouts
from training4.execution import encode_boards, plan_batch
from training4.optimizer_groups import build_optimizer_groups
from training4.partial import apply_partial_training, below_layer_frozen, upstream_frozen
from training4.rules_only import ParameterAnchor
from training4.think_loss import ThinkLossConfig

CPU = torch.device("cpu")
HEAD = ("cand_proj.", "cand_scorer.", "cand_ctx_proj.", "cand_context_gate.")
BLOCK = "deliberation."


def think_net(layers=2, seed=3, stirred=True):
    torch.manual_seed(seed)
    net = V5.PlayerNet5(V5.Config5(d_model=32, n_heads=4, n_layers=layers, d_ff=64, structure_width=16,
                                   relational_width=16, relational_revision=2, thinking_revision=2,
                                   deliberation_width=32, deliberation_heads=4, deliberation_candidates=4,
                                   deliberation_steps=3))
    net.decision_chunk = 2
    net.prune_unused_policy_heads = True
    net.deliberation.steps_override = 3
    net.deliberation.collect = True
    if stirred:
        with torch.no_grad():
            net.deliberation.correction.weight.normal_(std=.3)
            net.relational.output.weight.normal_(std=.05)
    return net


def run(net, reference, batch, scope, config=ThinkLossConfig()):
    """One distillation loss with the thinking terms, as the driver computes it."""
    batch.setdefault("is_new_traj", torch.ones(4, dtype=torch.bool))
    plans = plan_batch(batch, 0, 2)
    with reference.capturing(net):
        boards = encode_boards(net, batch, 2, CPU)
    hidden = torch.zeros(4, 9, 32)
    prepared, _ = deferred_readouts(net, batch, boards, hidden, plans, 2, CPU)
    ref = reference.outputs(batch, batch, boards, hidden, plans, CPU, False, 2)
    by_step = []

    def thinking(_prepared, b, p):
        value, stats = D.thinking_terms(net.thinking_frames, ref, b, p, scope, config)
        by_step.append(stats)
        return value
    loss, _ = D.distill_loss(prepared, ref, batch, plans, scope, {"thinking": thinking})
    return loss, by_step[0], prepared, ref


def touched(net):
    return {name for name, p in net.named_parameters() if p.grad is not None and float(p.grad.abs().max()) > 0}


class Arms(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def arm(self, **stage):
        net = think_net()
        snapshot = D.Reference.snapshot(net)
        description = apply_partial_training(net, flags(**stage))
        reference = D.Reference(snapshot, net, stage.get("trainable_trunk_from_layer", -1))
        return net, reference, description

    def test_a_plus_block_trains_exactly_the_head_and_the_block(self):
        net, reference, description = self.arm(train_policy_head_only=True)
        self.assertEqual((description["mode"], description["deliberation"]), ("policy_head", "trains"))
        trainable = {name for name, p in net.named_parameters() if p.requires_grad}
        self.assertEqual(trainable, {name for name, _ in net.named_parameters()
                                     if name.startswith(HEAD + (BLOCK,))})
        # The block sits after the heads: boards and memory stay r1's.
        self.assertTrue(upstream_frozen(net))
        self.assertTrue(reference.shared_trunk)
        loss, _, _, _ = run(net, reference, with_subtypes(distill_batch()), scoped())
        loss.backward()
        moved = touched(net)
        self.assertTrue(moved <= trainable)
        for name in ("deliberation.correction.weight", "deliberation.confidence.weight",
                     "deliberation.self_attn.q_proj.weight", "deliberation.cross_attn.q_proj.weight",
                     "cand_scorer.0.weight", "cand_proj.0.weight"):
            self.assertIn(name, moved)
        # Nothing reads the consequence of an answer under these targets.
        self.assertNotIn("deliberation.consequence.weight", moved)

    def test_b_plus_block_trains_the_upper_trunk_the_heads_and_the_block(self):
        net, reference, description = self.arm(trainable_trunk_from_layer=1, freeze_value_head=True)
        self.assertEqual((description["mode"], description["deliberation"]), ("upper_trunk", "trains"))
        self.assertFalse(upstream_frozen(net))
        self.assertTrue(below_layer_frozen(net, 1))
        self.assertEqual(reference.shared_lower, 1)
        loss, _, _, _ = run(net, reference, with_subtypes(distill_batch()), scoped())
        loss.backward()
        moved = touched(net)
        allowed = ("trunk_layers.1.", "trunk_norm.", "thinking_block.", "pair_", "scalar_head.", BLOCK) + HEAD
        self.assertTrue(all(name.startswith(allowed) for name in moved), moved)
        for name in ("trunk_layers.1.attn.q_proj.weight", "deliberation.correction.weight", "cand_scorer.0.weight"):
            self.assertIn(name, moved)
        for name, parameter in net.named_parameters():
            if name.startswith(("trunk_layers.0.", "entity_proj.", "rules", "relational.", "recurrent_cell.",
                                "h_norm.", "seat_value_head.", "aux_head.")):
                self.assertIsNone(parameter.grad, name)

    def test_the_adapter_leaves_the_block_frozen_and_a_net_without_one_is_described_as_before(self):
        net, _, description = self.arm(adapter_on_candidate_head=2)
        self.assertNotIn("deliberation", description)
        self.assertFalse(any(p.requires_grad for n, p in net.named_parameters() if n.startswith(BLOCK)))
        from test_distill_stages import tiny_net
        plain = apply_partial_training(tiny_net(), flags(train_policy_head_only=True))
        self.assertEqual(set(plain), {"version", "mode", "adapter_rank", "trainable_trunk_from_layer",
                                      "freeze_value_head", "trainable_parameters"})

    def test_the_reference_stays_r1_whatever_the_block_has_learnt(self):
        net = think_net(stirred=False)
        snapshot = D.Reference.snapshot(net)
        apply_partial_training(net, flags(train_policy_head_only=True))
        reference = D.Reference(snapshot, net)
        batch = with_subtypes(distill_batch())
        _, _, prepared, ref = run(net, reference, batch, scoped())
        # A fresh block corrects nothing: the student is r1 at every step.
        for t in range(3):
            torch.testing.assert_close(prepared[t][1], ref[t][1], atol=1e-6, rtol=0)
        with torch.no_grad():
            net.deliberation.correction.weight.normal_(std=.5)
            reference.net.deliberation.correction.weight.normal_(std=.5)
        _, _, moved, same = run(net, reference, batch, scoped())
        self.assertGreater(float((moved[1][1] - ref[1][1]).abs().max()), 1e-3)
        # The reference's block is switched off: its answer is the heads' first proposal.
        for t in range(3):
            torch.testing.assert_close(same[t][1], ref[t][1], atol=0, rtol=0)
        self.assertIsNone(reference.net.thinking_frames)


class Validation(unittest.TestCase):
    def test_held_out_agreement_with_r_is_reported_per_step_and_ends_at_the_selection_metric(self):
        torch.set_num_threads(1)
        net = think_net()
        snapshot = D.Reference.snapshot(net)
        apply_partial_training(net, flags(train_policy_head_only=True))
        reference = D.Reference(snapshot, net)
        batch = with_subtypes(distill_batch())
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        metrics = D.evaluate_distill(net, reference, [batch, copy.deepcopy(batch)], CPU, scoped(), 2, .05, 2,
                                     bf16=False, think=ThinkLossConfig())
        by_step = metrics["val_distill_think_top1_r_at_n_ne_r_by_step"]
        self.assertEqual(len(by_step), 4)
        self.assertEqual(metrics["val_distill_think_n_ne_r"], metrics["val_distill_distilled_n_ne_r"])
        # The last step is the net's answer: the same number the checkpoint is selected on.
        self.assertAlmostEqual(by_step[-1], metrics["val_distill_selection"])
        plain = D.evaluate_distill(net, reference, [batch], CPU, scoped(), 2, .05, 2, bf16=False)
        self.assertNotIn("val_distill_think_top1_r_at_n_ne_r_by_step", plain)


class Terms(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def frames(self, steps_logits, confidence=None):
        """Traces of lanes 0-1 (single choices) from `[S + 1, B, U, C]` logits."""
        s, b, u, c = steps_logits.shape
        confidence = torch.zeros(s - 1, b, u) if confidence is None else confidence
        return [{"logits": steps_logits[:, :, t], "confidence": confidence[:, :, t],
                 "consequence": torch.zeros(s - 1, b, 4), "index": torch.zeros(b, 4, dtype=torch.long),
                 "chosen": torch.ones(b, 4, dtype=torch.bool)} for t in range(u)]

    def setting(self):
        batch = with_subtypes(distill_batch())
        batch["head"][2:] = 3          # only the single-choice lanes are read here
        plans = plan_batch(batch, 0, 2)
        r1 = torch.zeros(4, 3, 7)
        r1[..., 0] = 2.                # r1 answers candidate 0 everywhere
        reference = [(None, r1[:, t], None, None, None) for t in range(3)]
        return batch, plans, r1, reference

    def test_a_step_is_pulled_to_r_only_in_scope_where_n_differs_and_to_r1_elsewhere(self):
        batch, plans, r1, reference = self.setting()
        # In scope with N != R: lane 0 frame 0 (cast, R = 1) and lane 1 frame 1 is land: out of scope.
        steps = torch.stack((r1, r1, r1)).clone().requires_grad_(True)
        only_deep = ThinkLossConfig(deep_weight=1., improve_weight=0., confidence_weight=0.)
        loss, stats = D.thinking_terms(self.frames(steps), reference, batch, plans, scoped(), only_deep)
        loss.backward()
        gradient = steps.grad[1]
        # At the in-scope N != R decision the middle step is pushed toward R = 1.
        self.assertLess(float(gradient[0, 0, 1]), 0.)
        self.assertGreater(float(gradient[0, 0, 0]), 0.)
        # Where the step already is r1 and R is not the target, nothing pulls: a
        # label R = 3 out of scope (lane 1, frame 1) moves nothing.
        self.assertLess(float(gradient[1, 1].abs().max()), 1e-6)
        self.assertLess(float(gradient[1, 0].abs().max()), 1e-6)
        # The last step belongs to distill_loss, step 0 to the heads.
        self.assertEqual(float(steps.grad[0].abs().max()), 0.)
        self.assertEqual(float(steps.grad[2].abs().max()), 0.)
        # One in-scope N != R decision; r1 does not answer R at any step.
        self.assertEqual(stats.tolist(), [0., 0., 0., 1.])

    def test_improvement_penalises_a_later_step_that_is_worse_and_counts_corrections(self):
        batch, plans, r1, reference = self.setting()
        toward = r1.clone(); toward[0, 0, 1] = 6.       # answers R = 1 at the in-scope N != R decision
        only_improve = ThinkLossConfig(deep_weight=0., improve_weight=1., confidence_weight=0.)
        better, stats = D.thinking_terms(self.frames(torch.stack((r1, toward, toward))), reference, batch, plans,
                                         scoped(), only_improve)
        worse, _ = D.thinking_terms(self.frames(torch.stack((r1, toward, r1))), reference, batch, plans,
                                    scoped(), only_improve)
        self.assertGreater(float(worse), float(better) + 1e-3)
        self.assertEqual(stats.tolist(), [0., 1., 1., 1.])
        # The rule groups' weights scale the cross-entropy of a step as they do the head's.
        weighted, _ = D.thinking_terms(self.frames(torch.stack((r1, toward, r1))), reference, batch, plans,
                                       scoped(group_weights=(3., 1.)), only_improve)
        self.assertGreater(float(weighted), float(worse) * 1.5)

    def test_confidence_learns_whether_the_step_answers_its_target(self):
        batch, plans, r1, reference = self.setting()
        toward = r1.clone(); toward[0, 0, 1] = 6.
        only_confidence = ThinkLossConfig(deep_weight=0., improve_weight=0., confidence_weight=1.)
        confidence = torch.zeros(2, 4, 3, requires_grad=True)
        loss, _ = D.thinking_terms(self.frames(torch.stack((r1, r1, toward)), confidence), reference, batch, plans,
                                   scoped(), only_confidence)
        loss.backward()
        # Step 1 still answers r1 where R is wanted: its confidence is pushed down; step 2 answers R: up.
        self.assertGreater(float(confidence.grad[0, 0, 0]), 0.)
        self.assertLess(float(confidence.grad[1, 0, 0]), 0.)
        # Out of scope the target is r1's own answer, which both steps give.
        self.assertLess(float(confidence.grad[0, 1, 1]), 0.)
        with self.assertRaises(ValueError):
            D.thinking_terms([None, None, None], reference, batch, plans, scoped(), only_confidence)


class Recipe(unittest.TestCase):
    def test_a_fresh_block_is_left_out_of_the_anchor_and_learns_at_its_own_rate(self):
        net = think_net()
        apply_partial_training(net, flags(train_policy_head_only=True))
        anchor = ParameterAnchor(net, .15, fresh=(BLOCK,))
        held = {id(p) for p, _ in anchor.pairs}
        for name, parameter in net.named_parameters():
            self.assertEqual(id(parameter) in held, parameter.requires_grad and not name.startswith(BLOCK), name)
        self.assertEqual(len(ParameterAnchor(net, .15).pairs),
                         sum(p.requires_grad for p in net.parameters()))
        groups, _ = build_optimizer_groups(net, 6e-5, architecture="v5", decay=False, thinking_lr=3e-4)
        block = {id(p) for n, p in net.named_parameters() if n.startswith(BLOCK)}
        self.assertEqual([(g["lr"], g["weight_decay"]) for g in groups], [(6e-5, 0.), (3e-4, 1e-2)])
        self.assertEqual({id(p) for p in groups[1]["params"]}, block)
        self.assertFalse({id(p) for p in groups[0]["params"]} & block)
        same, _ = build_optimizer_groups(net, 6e-5, architecture="v5", decay=False)
        self.assertEqual(len(same), 1)
        from test_distill_stages import tiny_net
        with self.assertRaises(ValueError):
            build_optimizer_groups(tiny_net(), 6e-5, thinking_lr=3e-4)

    BASE = ["train5", "--data", "d", "--out", "o", "--init", "i", "--distill-bundle",
            "--distill-subtypes", "cast,pass,instant_their_turn,spend", "--checkpoint-selection", "distill_n_ne_r",
            "--anchor-l2", "0.15", "--lr", "6e-5", "--decision-chunk", "4", "--board-chunk", "32",
            "--window-burn-in", "0", "--loss-version", "5", "--loss-normalization", "unroll_per_head",
            "--relational-revision", "2"]
    THINK = ["--thinking-revision", "2", "--thinking-warmstart", "--think-supervision", "--deliberation-steps", "4",
             "--thinking-lr", "3e-4"]

    def parse(self, argv):
        from training4.cli import parse_args
        with patch("sys.argv", argv), contextlib.redirect_stderr(io.StringIO()):
            return parse_args("v5")

    def test_round0_arms_with_the_block_parse_and_the_block_alone_combines_with_no_stage(self):
        for stage in (["--train-policy-head-only"], ["--trainable-trunk-from-layer", "8", "--freeze-value-head"]):
            args, _, identity = self.parse([*self.BASE, *stage, *self.THINK])
            self.assertEqual((args.thinking_revision, args.thinking_lr), (2, 3e-4))
            self.assertIsNotNone(args.think_loss)
            self.assertEqual(identity["thinking"]["contract"], "deliberation_v1")
        alone, _, identity = self.parse([*self.BASE, *self.THINK, "--train-thinking-only"])
        self.assertEqual(identity["train_parts"], "thinking")
        for bad in ([*self.BASE, "--train-policy-head-only", *self.THINK, "--train-thinking-only"],
                    [*self.BASE, "--trainable-trunk-from-layer", "8", *self.THINK, "--train-thinking-only"],
                    [*self.BASE, "--adapter-on-candidate-head", "4", *self.THINK],
                    [*self.BASE, "--train-policy-head-only", "--thinking-lr", "3e-4"]):
            with self.subTest(bad=bad[-3:]), self.assertRaises(SystemExit):
                self.parse(bad)


class Driver(unittest.TestCase):
    def test_both_arms_through_the_driver_update_what_they_name_and_report_steps(self):
        from model4.structure import structure_digest
        from training4 import driver
        batch = with_subtypes(distill_batch())
        batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        forest = V5.PlayerNet5(V5.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64, relational_revision=2)).rules.data
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
                  "--epochs", "1", "--loss-version", "5", "--loss-normalization", "unroll_per_head", "--lr", "1e-2",
                  "--relational-revision", "2"]
        printed = io.StringIO()

        def go(output, extra, data_meta):
            meta.clear()
            meta.update(data_meta)
            with patch("sys.argv", ["train5", "--out", str(output), *common, *extra]), \
                    patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                    patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                    patch.object(driver, "DataLoader", return_value=[copy.deepcopy(batch)]), \
                    patch.object(driver, "prefetch_batches", return_value=[(copy.deepcopy(batch), None)]), \
                    patch.object(driver, "evaluate_device", return_value=dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)), \
                    patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(printed):
                driver.main("v5")
            return torch.load(output / "last.pt", map_location="cpu", weights_only=False)

        labeled = {"bundle_distillation": {"contract": D.CONTRACT, "subtypes": list(SUBTYPES), "groups": ["mass", "react"]}}
        distill = ["--distill-bundle", "--checkpoint-selection", "distill_n_ne_r", "--skip-final-validation",
                   "--distill-subtypes", "cast,pass,instant_their_turn,spend", "--anchor-l2", "0.15"]
        think = ["--thinking-revision", "2", "--thinking-warmstart", "--think-supervision", "--deliberation-steps", "3",
                 "--deliberation-width", "32", "--deliberation-candidates", "4", "--thinking-lr", "3e-2"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initial = go(root / "init", [], {})["state_dict"]
            init = ["--init", str(root / "init")]
            upper = ("trunk_layers.1.", "trunk_norm.", "thinking_block.", "cand_", "pair_", "scalar_head.", BLOCK)
            for name, stage, allowed in (
                    ("a-block", ["--train-policy-head-only"], HEAD + (BLOCK,)),
                    ("b-block", ["--trainable-trunk-from-layer", "1", "--freeze-value-head"], upper),
                    ("block", ["--train-thinking-only"], (BLOCK,))):
                saved = go(root / name, [*init, *distill, *stage, *think], labeled)
                state = saved["state_dict"]
                self.assertEqual({k for k in state if not k.startswith(BLOCK)}, set(initial), name)
                changed = {k for k, v in state.items() if k not in initial or not torch.equal(v, initial[k])}
                self.assertTrue(all(k.startswith(allowed) for k in changed), (name, changed))
                self.assertGreater(float(state["deliberation.correction.weight"].abs().max()), 0., name)
                if name != "block":
                    self.assertFalse(torch.equal(state["cand_scorer.0.weight"], initial["cand_scorer.0.weight"]), name)
                    self.assertEqual(saved["partial_training"]["deliberation"], "trains")
                self.assertEqual(saved["thinking"]["targets"], "bundle_distillation_scope_v1")
                self.assertEqual(saved["config"]["thinking_revision"], 2)


if __name__ == "__main__":
    unittest.main()

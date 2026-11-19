"""Advantage distillation (`--distill-advantage pairwise`): the probe sidecar, the pairwise loss, its validation."""
import contextlib
import copy
import hashlib
import io
import json
import math
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import torch

import model4 as M
from test_training_execution4 import batch_fixture
import test_distill_stages as stages
from training4 import distill as D
from training4 import distill_probe as P
from training4.execution import plan_batch
from training4.objective import selection_score

ADVANTAGE = D.Scope(kinds=(0,), advantage="pairwise")   # beta .05, clip 3, two worlds at least
NAN = float("nan")


def advantage_batch():
    """One lane of priority decisions over candidates 0..3, N != R unless said:
    t0 probed R 1 / N 0, R ahead (+.1 +- .02); t1 probed R 2 / N 0, N ahead (-.1 +- .02);
    t2 probed R 3 / N 1, no advantage; t3 unprobed R 1; t4 R 2 / N 0 probed in one world only
    (+.5); t5 N = R = 0."""
    b, u = 1, 6
    batch = batch_fixture(b=b, u=u)
    batch["step_valid"][:] = True
    batch["is_decision"][:] = True
    batch["is_new_traj"] = torch.ones(b, dtype=torch.bool)
    batch["loss_weight"] = torch.ones(b, u)
    batch["head"] = torch.zeros(b, u, dtype=torch.long)
    batch["pending_kind"] = torch.zeros(b, u, dtype=torch.long)
    batch["distill_subtype"] = torch.full((b, u), -1)
    batch["fired_mask"] = torch.zeros(b, u, dtype=torch.long)
    batch["cand_mask"][:] = False
    batch["cand_mask"][..., :4] = True
    batch["single_idx"] = torch.tensor([[1, 2, 3, 1, 2, 0]])
    batch["n_ne_r"] = torch.tensor([[True, True, True, True, True, False]])
    batch["probe_n_idx"] = torch.tensor([[0, 0, 1, -1, 0, -1]])
    batch["probe_worlds"] = torch.tensor([[4, 4, 4, 0, 1, 0]])
    batch["probe_mean"] = torch.tensor([[.1, -.1, 0., 0., .5, 0.]])
    batch["probe_se"] = torch.tensor([[.02, .02, .05, NAN, NAN, NAN]])
    return batch


def outputs(cand):
    """Deferred-readout-shaped outputs from [B, U, C] candidate logits."""
    b, u, _ = cand.shape
    return stages.outputs_from(cand, torch.zeros(b, u, 4), torch.zeros(b, u))


def loss_of(student_cand, reference_cand, batch, scope=ADVANTAGE):
    student_cand = student_cand.clone().requires_grad_()
    loss, stats = D.distill_loss(outputs(student_cand), outputs(reference_cand), batch, plan_batch(batch, 0), scope)
    loss.backward()
    return loss, stats, student_cand.grad


def rows(stats, name):
    return stats[len(D.TERMS) + D.TERMS.index(name)].item()


class PairwiseLoss(unittest.TestCase):
    def test_probed_frames_learn_pairwise_and_nothing_is_ce_to_r(self):
        loss, stats, _ = loss_of(torch.zeros(1, 6, 7), torch.zeros(1, 6, 7), advantage_batch())
        # Probed with two worlds or more: t0-t2; unprobed N != R: t3 and the one-world t4; N = R: t5.
        self.assertEqual({name: rows(stats, name) for name in ("bce_adv_pairwise", "kl_adv_unprobed",
                                                               "kl_distilled_n_eq_r", "ce_r")},
                         dict(bce_adv_pairwise=3., kl_adv_unprobed=2., kl_distilled_n_eq_r=1., ce_r=0.))
        # z_s = z_r = 0: each row is softplus(0) - q * 0 = log 2; the KL rows are zero.
        means = D.term_means(stats)
        self.assertAlmostEqual(means["bce_adv_pairwise"], math.log(2.), places=6)
        self.assertAlmostEqual(loss.item(), math.log(2.), places=6)
        self.assertEqual(D.train_line(stats)["bce_adv_pairwise"], means["bce_adv_pairwise"])

    def test_the_advantage_moves_rs_log_odds_over_n_and_no_advantage_moves_nothing(self):
        _, _, grad = loss_of(torch.zeros(1, 6, 7), torch.zeros(1, 6, 7), advantage_batch())
        # d/dz_s = (sigmoid(z_s) - q) / 3 rows, with q = sigmoid(z_r + clamp(mean / .05, +-3)).
        step = (.5 - torch.sigmoid(torch.tensor(2.)).item()) / 3
        torch.testing.assert_close(grad[0, 0, :4], torch.tensor([-step, step, 0., 0.]))   # R ahead: R rises over N
        torch.testing.assert_close(grad[0, 1, :4], torch.tensor([step, 0., -step, 0.]))   # N ahead: N rises over R
        self.assertLess(grad[0, 0, 1].item(), 0.)
        self.assertGreater(grad[0, 1, 2].item(), 0.)
        # No advantage, student = r1; and every KL row at its minimum: no gradient at all.
        self.assertEqual(grad[0, 2:].abs().max().item(), 0.)

    def test_zero_advantage_keeps_r1s_own_odds_wherever_they_stand(self):
        torch.manual_seed(5)
        r1 = torch.randn(1, 6, 7) * 2
        _, _, grad = loss_of(r1, r1, advantage_batch())
        self.assertEqual(grad[0, 2].abs().max().item(), 0.)              # mean 0: q = sigmoid(z_r) = sigmoid(z_s)
        self.assertLess(grad[0, 3:].abs().max().item(), 1e-6)            # unprobed and N = R: KL at its minimum
        self.assertGreater(grad[0, 0, :4].abs().max().item(), 0.)

    def test_one_sgd_step_raises_or_lowers_z_s_by_the_sign_and_clips(self):
        logits = torch.zeros(1, 6, 7)
        _, _, grad = loss_of(logits, torch.zeros(1, 6, 7), advantage_batch())
        after = logits - 3. * grad
        z = lambda t, r, n: (after[0, t, r] - after[0, t, n]).item()
        self.assertGreater(z(0, 1, 0), 0.)
        self.assertLess(z(1, 2, 0), 0.)
        self.assertEqual(z(2, 3, 1), 0.)
        # With one world enough, t4 (+.5 = 10 betas) is probed and its shift capped at 3.
        scope = D.Scope(kinds=(0,), advantage="pairwise", advantage_min_worlds=1)
        _, stats, grad = loss_of(torch.zeros(1, 6, 7), torch.zeros(1, 6, 7), advantage_batch(), scope)
        self.assertEqual((rows(stats, "bce_adv_pairwise"), rows(stats, "kl_adv_unprobed")), (4., 1.))
        self.assertAlmostEqual(grad[0, 4, 2].item(), (.5 - torch.sigmoid(torch.tensor(3.)).item()) / 4, places=6)

    def test_unprobed_n_ne_r_rows_never_pull_toward_r(self):
        batch = advantage_batch()
        r1 = torch.zeros(1, 6, 7)
        r1[0, 3, 0] = 2.                             # r1 answers N (0) at the unprobed t3; R is 1
        _, stats, grad = loss_of(r1, r1, batch)
        self.assertLess(grad[0, 3].abs().max().item(), 1e-6)            # the KL at its minimum (float rounding)
        student = r1.clone()
        student[0, 3, 1] = 3.                        # a student leaning to R there is pulled back to r1
        _, _, grad = loss_of(student, r1, batch)
        self.assertGreater(grad[0, 3, 1].item(), 0.)
        self.assertLess(grad[0, 3, 0].item(), 0.)
        # Without the mode the same row is cross-entropy to R: R rises.
        plain = copy.deepcopy(batch)
        _, plain_stats, plain_grad = loss_of(student, r1, plain, D.Scope(kinds=(0,)))
        self.assertLess(plain_grad[0, 3, 1].item(), 0.)
        self.assertEqual(rows(plain_stats, "ce_r"), 5.)
        self.assertEqual(rows(stats, "ce_r"), 0.)

    def test_other_kinds_keep_their_terms_and_the_total_weights_each_part(self):
        batch = stages.distill_batch()
        b, u = batch["single_idx"].shape
        batch.update(probe_n_idx=torch.full((b, u), -1), probe_worlds=torch.zeros(b, u, dtype=torch.long),
                     probe_mean=torch.zeros(b, u), probe_se=torch.full((b, u), NAN))
        batch["probe_n_idx"][0, 0], batch["probe_worlds"][0, 0], batch["probe_mean"][0, 0] = 0, 3, .02
        plans = plan_batch(batch, 0)
        student, r1 = stages.drifted()
        plain = D.distill_loss(student, r1, batch, plans, stages.PRIORITY)[1]
        scope = D.Scope(kinds=(0,), kl_weight=.5, advantage="pairwise", advantage_weight=2.)
        loss, stats = D.distill_loss(student, r1, batch, plans, scope)
        n = len(D.TERMS)
        for name in ("kl_single", "kl_subset", "kl_pair", "sq_scalar", "kl_kind_out_of_scope"):
            i = D.TERMS.index(name)
            self.assertEqual((stats[i].item(), stats[n + i].item()), (plain[i].item(), plain[n + i].item()), name)
        # Priority: (0,0) probed; (1,1) N != R unprobed; (0,1), (1,0), (1,2) N = R.
        self.assertEqual((rows(stats, "bce_adv_pairwise"), rows(stats, "kl_adv_unprobed"),
                          rows(stats, "kl_distilled_n_eq_r"), rows(stats, "ce_r")), (1., 1., 3., 0.))
        means = D.term_means(stats)
        i, j = D.TERMS.index("kl_distilled_n_eq_r"), D.TERMS.index("kl_adv_unprobed")
        pooled = (stats[i] + stats[j]).item() / (stats[n + i] + stats[n + j]).item()
        others = sum(means[name] or 0. for name in ("kl_kind_out_of_scope", "kl_single", "kl_subset", "kl_pair",
                                                    "sq_scalar"))
        self.assertAlmostEqual(loss.item(), 2. * means["bce_adv_pairwise"] + .5 * (pooled + others), places=5)

    def test_thinking_terms_refuse_the_mode(self):
        with self.assertRaisesRegex(ValueError, "no R"):
            D.thinking_terms([None], [], advantage_batch(), [], ADVANTAGE, None)


def sidecar_rows():
    """Two probed frames: game 7 seat 0 cursor 3 (R 2 / N 0, two worlds) and game 7 seat 1
    cursor 3 (R 1 / N 2, subtype 4, fired 0b10)."""
    index = np.zeros(2, dtype=P.INDEX)
    index["game"], index["seat"], index["cursor"] = 7, (0, 1), 3
    index["r_idx"], index["n_idx"], index["worlds"] = (2, 1), (0, 2), (2, 5)
    index["subtype"], index["fired"] = (P.NO_SUBTYPE, 4), (0, 0b10)
    index["mean"], index["se"] = (.03, -.08), (.01, .02)
    return index


def write_sidecar(root, index, **header_overrides):
    raw = index.tobytes()
    (root / "probe.index.bin").write_bytes(raw)
    digest = hashlib.sha256(raw).hexdigest()
    header = dict(version=1, contract=P.CONTRACT, rows=len(index), key=["game", "seat", "cursor"],
                  index=dict(file="probe.index.bin", row_bytes=56, sorted_by=["game", "seat", "cursor"],
                             dtype=[[name, P.INDEX.fields[name][0].str.replace("|", "")] for name in P.INDEX.names],
                             sha256=digest))
    header.update(header_overrides)
    (root / "probe.json").write_text(json.dumps(header))
    return dict(dir=str(root), rows=len(index), contract=P.CONTRACT, header="probe.json", index_sha256=digest)


def frame(single, n_ne_r=True, labelled=True, subtype=-1, fired=0, width=3, masked=()):
    mask = np.ones(width, bool)
    mask[list(masked)] = False
    return dict(cand_mask=mask, is_decision=np.array(labelled), loss_weight=np.array(float(labelled), np.float32),
                pending_kind=np.array(0), head=np.array(0), single_idx=np.array(single), n_ne_r=np.array(n_ne_r),
                distill_subtype=np.array(subtype), fired_mask=np.array(fired))


class ProbeSidecar(unittest.TestCase):
    def test_joins_by_game_seat_cursor_and_fails_on_a_contradicting_frame(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry = write_sidecar(root, sidecar_rows())
            probes = P.Probes.load(root, entry)
            joined = probes.annotate(frame(2), 7, 0, 3)
            self.assertEqual((int(joined["probe_n_idx"]), int(joined["probe_worlds"])), (0, 2))
            self.assertAlmostEqual(float(joined["probe_mean"]), .03, places=6)
            self.assertAlmostEqual(float(joined["probe_se"]), .01, places=6)
            joined = probes.annotate(frame(1, subtype=4, fired=0b10), 7, 1, 3)
            self.assertEqual(int(joined["probe_n_idx"]), 2)
            unprobed = probes.annotate(frame(1), 7, 0, 4)                # no row: no probe
            self.assertEqual((int(unprobed["probe_n_idx"]), int(unprobed["probe_worlds"])), (-1, 0))
            self.assertTrue(math.isnan(float(unprobed["probe_se"])))
            passive = probes.annotate(frame(0, labelled=False), 7, 0, 4)   # nothing to join, nothing claimed
            self.assertEqual(int(passive["probe_worlds"]), 0)
            for name, bad in (("R is not the target", (frame(1), 7, 0, 3)),
                              ("N masked", (frame(2, masked=(0,)), 7, 0, 3)),
                              ("R off the menu", (frame(2, width=2), 7, 0, 3)),
                              ("the frame says N = R", (frame(2, n_ne_r=False), 7, 0, 3)),
                              ("another subtype", (frame(1, subtype=3, fired=0b10), 7, 1, 3)),
                              ("other fired groups", (frame(1, subtype=4), 7, 1, 3)),
                              ("no labelled decision", (frame(2, labelled=False), 7, 0, 3))):
                with self.subTest(name), self.assertRaises(ValueError):
                    probes.annotate(*bad)

    def test_load_fails_closed(self):
        index = sidecar_rows()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry = write_sidecar(root, index)
            for name, change in (("meta names another digest", lambda: P.Probes.load(root, {**entry,
                                                                                         "index_sha256": "0" * 64})),
                                 ("meta names other rows", lambda: P.Probes.load(root, {**entry, "rows": 3}))):
                with self.subTest(name), self.assertRaisesRegex(ValueError, "another probe sidecar"):
                    change()
            write_sidecar(root, index, contract="distill_probe_v0")
            with self.assertRaisesRegex(ValueError, "distill_probe_v0"):
                P.Probes.load(root)
            write_sidecar(root, index, rows=3)
            with self.assertRaisesRegex(ValueError, "row count"):
                P.Probes.load(root)
            write_sidecar(root, index)
            header = json.loads((root / "probe.json").read_text())
            header["index"]["sha256"] = "0" * 64
            (root / "probe.json").write_text(json.dumps(header))
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                P.Probes.load(root)
            header["index"]["dtype"] = header["index"]["dtype"][:-1]
            (root / "probe.json").write_text(json.dumps(header))
            with self.assertRaisesRegex(ValueError, "layout"):
                P.Probes.load(root)
        with self.assertRaisesRegex(ValueError, "sorted"):
            P.Probes(index[::-1].copy())
        same = index.copy()
        same["n_idx"][0] = same["r_idx"][0]
        with self.assertRaisesRegex(ValueError, "N is its R"):
            P.Probes(same)
        self.assertIsNone(P.named({"bundle_distillation": {"contract": D.CONTRACT}}))
        self.assertEqual(P.named({"bundle_distillation": {"probe": {"rows": 2}}}), {"rows": 2})

    def test_the_dataset_hands_every_stored_frame_to_the_sidecar(self):
        import dataset4
        dataset = object.__new__(dataset4.TrajectoryDataset4)
        dataset.store = SimpleNamespace(index=np.array([(7, 0, 3)], dtype=[("game", "<u4"), ("seat", "u1"),
                                                                          ("cursor", "<u4")]))
        dataset._encode_step = lambda row: frame(2)
        index = sidecar_rows()
        dataset.probes = P.Probes(index)
        self.assertEqual(int(dataset.encode_single_step(0)["probe_n_idx"]), 0)
        index["r_idx"][0] = 1
        dataset.probes = P.Probes(index)
        with self.assertRaisesRegex(ValueError, "does not match"):
            dataset.encode_single_step(0)


class AdvantageValidation(unittest.TestCase):
    def test_answers_where_r_or_n_leads_and_the_shift_correlation(self):
        batch = advantage_batch()
        cand = torch.zeros(1, 6, 7)
        cand[0, 0, 1] = 2.     # R ahead at t0: the student answers R (z_s 2, target shift 2)
        cand[0, 1, 2] = -2.    # N ahead at t1: it answers N (z_s -2, target shift -2); t2 stays r1
        counter = P.AdvantageCounter(ADVANTAGE)
        counter.observe(outputs(cand), outputs(torch.zeros(1, 6, 7)), batch, plan_batch(batch, 0))
        metrics = counter.metrics()
        self.assertEqual({key: metrics[key] for key in ("val_adv_probed_rows", "val_adv_r_ahead_rows",
                                                        "val_adv_n_ahead_rows")},
                         dict(val_adv_probed_rows=3, val_adv_r_ahead_rows=1, val_adv_n_ahead_rows=1))
        # r1 (zeros) answers candidate 0: N at both.
        self.assertEqual((metrics["val_adv_r_ahead_top1_r"], metrics["val_adv_r_ahead_top1_r_r1"]), (1., 0.))
        self.assertEqual((metrics["val_adv_n_ahead_top1_n"], metrics["val_adv_n_ahead_top1_n_r1"]), (1., 1.))
        self.assertAlmostEqual(metrics["val_adv_shift_corr"], 1., places=6)
        self.assertEqual((metrics["val_adv_shift_mean_r_ahead"], metrics["val_adv_shift_mean_n_ahead"]), (2., -2.))
        self.assertEqual(metrics["val_adv_selection"], 1.)
        # Untouched, the student is r1: no shift to correlate, r1's own answers select.
        untouched = P.AdvantageCounter(ADVANTAGE)
        untouched.observe(outputs(torch.zeros(1, 6, 7)), outputs(torch.zeros(1, 6, 7)), batch, plan_batch(batch, 0))
        metrics = untouched.metrics()
        self.assertIsNone(metrics["val_adv_shift_corr"])
        self.assertEqual(metrics["val_adv_selection"], .5)
        # A batch without probes leaves the counter unseen.
        bare = P.AdvantageCounter(ADVANTAGE)
        bare.observe(outputs(cand), outputs(cand), stages.distill_batch(), plan_batch(stages.distill_batch(), 0))
        self.assertFalse(bare.seen)

    def test_selection_reads_the_mean_of_both_sides(self):
        better, worse = dict(val_adv_selection=.7), dict(val_adv_selection=.6)
        self.assertLess(selection_score(better, "distill_advantage"), selection_score(worse, "distill_advantage"))
        self.assertEqual(selection_score(dict(val_adv_selection=None), "distill_advantage"), float("inf"))
        self.assertEqual(selection_score({}, "distill_advantage"), float("inf"))

    def test_held_out_pass_keeps_the_pregate_and_adds_the_probes(self):
        torch.set_num_threads(1)
        net = stages.tiny_net()
        copied = D.Reference.snapshot(net)
        from training4.partial import apply_partial_training
        apply_partial_training(net, stages.flags(train_policy_head_only=True))
        reference = D.Reference(copied, net)
        batch = advantage_batch()
        metrics = D.evaluate_distill(net, reference, [batch], stages.CPU, ADVANTAGE, 2, .05, 2, bf16=False)
        self.assertEqual(metrics["val_adv_probed_rows"], 3)
        self.assertEqual(metrics["val_distill_selection"], metrics["val_adv_selection"])
        self.assertIsNone(metrics["val_adv_shift_corr"])   # untouched: the student is r1
        self.assertIn("val_distill_pregate_start_act_ratio", metrics)
        self.assertIn("val_distill_chain_completion", metrics)
        self.assertAlmostEqual(metrics["val_distill_kl_adv_unprobed"], 0., places=6)
        epoch = json.loads(D.epoch_line(1, 1, None, metrics).split(" ", 2)[2])
        self.assertEqual(epoch["validation"]["val_adv_probed_rows"], 3)
        # Without the mode the same probes are read for comparison; R stays the selection.
        plain = D.evaluate_distill(net, reference, [advantage_batch()], stages.CPU, D.Scope(kinds=(0,)), 2, .05, 2,
                                   bf16=False)
        self.assertEqual(plain["val_adv_probed_rows"], 3)
        self.assertEqual(plain["val_distill_selection"], plain["val_distill_distilled_top1_r_at_n_ne_r"])

    def test_pregate_stage_reports_the_probes_of_an_advantage_run(self):
        from training4 import pregate as G
        good = dict(val_distill_pregate_start_act_ratio=.9, val_distill_pregate_pass_ratio=.97,
                    val_adv_selection=.6, val_adv_r_ahead_top1_r=.4)
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()) as out:
            meta, verdict = Path(temporary) / "meta.json", Path(temporary) / "verdict.json"
            run = dict(bundle_distillation={"contract": D.CONTRACT, "advantage": {"mode": "pairwise"}})
            meta.write_text(json.dumps(dict(run, val_metrics=good)))
            self.assertEqual(G.main(["--model", temporary, "--out", str(verdict)]), 0)
            report = json.loads(verdict.read_text())
            self.assertEqual(report["advantage"], dict(val_adv_selection=.6, val_adv_r_ahead_top1_r=.4))
            meta.write_text(json.dumps(dict(run, val_metrics={**good, "val_distill_pregate_pass_ratio": .5})))
            self.assertEqual(G.main(["--model", temporary]), 3)
        self.assertIn("[advantage] ", out.getvalue())


class AdvantageOptions(unittest.TestCase):
    ARM = [a if a != "distill_n_ne_r" else "distill_advantage" for a in stages.DistillOptions.STAGE_A] + [
        "--distill-advantage", "pairwise"]

    def parse(self, argv):
        return stages.DistillOptions().parse(argv)

    def test_the_arm_parses_with_its_defaults_and_tuning(self):
        args = self.parse(self.ARM)
        self.assertEqual((args.distill_advantage, args.distill_advantage_beta, args.distill_advantage_clip,
                          args.distill_advantage_weight, args.distill_advantage_min_worlds),
                         ("pairwise", .05, 3., 1., 2))
        args = self.parse([*self.ARM, "--distill-advantage-beta", ".1", "--distill-advantage-clip", "2",
                           "--distill-advantage-weight", "0.5", "--distill-advantage-min-worlds", "4",
                           "--pregate-chain-start", ".8"])
        self.assertEqual((args.distill_advantage_beta, args.distill_advantage_min_worlds), (.1, 4))
        self.assertIsNone(self.parse(stages.DistillOptions.STAGE_A).distill_advantage)

    def test_refused_combinations(self):
        for bad in (["--distill-coarse", "act"], ["--distill-subtypes", "cast"],
                    ["--distill-group-weights", "mass=2"], ["--distill-out-of-scope", "ce-n"],
                    ["--distill-teacher", "house"], ["--distill-kinds", "0,8"],
                    ["--distill-advantage-beta", "0"], ["--distill-advantage-clip", "-1"],
                    ["--distill-advantage-weight", "-1"], ["--distill-advantage-min-worlds", "0"],
                    ["--distill-advantage-beta", "inf"], ["--distill-advantage", "listwise"]):
            with self.subTest(bad=bad), self.assertRaises(SystemExit):
                self.parse([*self.ARM, *bad])
        selection = list(self.ARM)
        selection[selection.index("distill_advantage")] = "distill_n_ne_r"
        stage_a = stages.DistillOptions.STAGE_A
        for name, argv in (("advantage selects with distill_advantage", selection),
                           ("distill_advantage needs the mode",
                            [a if a != "distill_n_ne_r" else "distill_advantage" for a in stage_a]),
                           ("tuning needs the mode", [*stage_a, "--distill-advantage-beta", ".1"]),
                           ("the mode needs --distill-bundle", [a for a in self.ARM if a != "--distill-bundle"]),
                           ("the selection needs --distill-bundle",
                            ["train5", "--data", "d", "--out", "o", "--checkpoint-selection", "distill_advantage"])):
            with self.subTest(name), self.assertRaises(SystemExit):
                self.parse(argv)
        data = [SimpleNamespace(distill_subtypes=["cast"], distill_groups=["mass"], probes=object())]
        for bad in (dict(coarse="act"), dict(subtypes=("cast",)), dict(group_weights={"mass": 2.}),
                    dict(out_of_scope="ce-n"), dict(kinds=(0, 8)), dict(advantage_beta=0.), dict(advantage="listwise"),
                    dict(advantage_min_worlds=0)):
            options = dict(kinds=(0,), kl_weight=1., subtypes=(), group_weights={}, datasets=data,
                           advantage="pairwise")
            options.update(bad)
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                D.Scope.from_options(**options)
        with self.assertRaisesRegex(ValueError, "probe sidecar"):
            D.Scope.from_options((0,), 1., (), {}, [SimpleNamespace(distill_subtypes=[], distill_groups=[])],
                                 advantage="pairwise")
        scope = D.Scope.from_options((0,), 1., (), {}, data, advantage="pairwise", advantage_beta=.1,
                                     advantage_min_worlds=3)
        self.assertEqual((scope.advantage, scope.advantage_beta, scope.advantage_min_worlds), ("pairwise", .1, 3))


class AdvantageDriver(unittest.TestCase):
    def test_a_head_only_run_trains_the_pairwise_term_and_names_its_probes(self):
        from training4 import driver
        from model4.structure import structure_digest
        batch = stages.distill_batch()
        b, u = batch["single_idx"].shape
        batch["is_new_traj"] = torch.ones(b, dtype=torch.bool)
        batch.update(probe_n_idx=torch.full((b, u), -1), probe_worlds=torch.zeros(b, u, dtype=torch.long),
                     probe_mean=torch.zeros(b, u), probe_se=torch.full((b, u), NAN))
        batch["probe_n_idx"][0, 0], batch["probe_worlds"][0, 0], batch["probe_mean"][0, 0] = 0, 3, .08
        forest = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data
        meta, where = {}, {}

        class Dataset:
            def __init__(self, *args, split, **kw):
                self.structure = SimpleNamespace(data=forest, digest=structure_digest(forest))
                self.fingerprint = split
                self.meta = meta
                self.data_dir = where.get("dir")

            def __len__(self):
                return 1

        class Sampler:
            VERSION = driver.D4.StatefulBatchSampler4.VERSION

            def __init__(self, *args, **kw):
                self.epoch = 0
                self.start_batch = 0

        common = ["--data", "fake", "--d-model", "32", "--layers", "2", "--heads", "4", "--d-ff", "64",
                  "--batch-size", "4", "--unroll", "3", "--board-chunk", "2", "--decision-chunk", "2", "--max-steps",
                  "1", "--epochs", "1", "--loss-version", "5", "--loss-normalization", "unroll_per_head", "--lr", "1e-2"]
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
                    patch.object(driver, "evaluate_device", return_value=dict(val_pol_loss=1., val_mse=1.,
                                                                              val_aux_mse=1., val_top1_acc=0.)), \
                    patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(stdout):
                driver.main("v5")
            return torch.load(output / "last.pt", map_location="cpu", weights_only=False)

        arm = ["--distill-bundle", "--distill-advantage", "pairwise", "--checkpoint-selection", "distill_advantage",
               "--train-policy-head-only", "--distill-advantage-beta", ".04"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sidecar = root / "data" / "bundle-distill"
            sidecar.mkdir(parents=True)
            entry = write_sidecar(sidecar, sidecar_rows())
            where["dir"] = root / "data"
            initial = run(root / "init", [], {})["state_dict"]
            init = ["--init", str(root / "init")]
            with self.assertRaisesRegex(ValueError, "probe sidecar"):
                run(root / "unprobed", [*init, *arm], {"bundle_distillation": {"contract": D.CONTRACT}})
            labelled = {"bundle_distillation": {"contract": D.CONTRACT, "probe": entry}}
            saved = run(root / "advantage", [*init, *arm], labelled)
            changed = {k for k, v in saved["state_dict"].items() if not torch.equal(v, initial[k])}
            self.assertTrue(changed and all(k.startswith(("cand_proj.", "cand_scorer.", "cand_ctx_proj."))
                                            for k in changed), changed)
            named = saved["bundle_distillation"]["advantage"]
            self.assertEqual((named["mode"], named["beta"], named["min_worlds"], named["rows"], named["index_sha256"]),
                             ("pairwise", .04, 2, 2, entry["index_sha256"]))
            self.assertEqual(saved["checkpoint_selection"], "distill_advantage")
            text = logs["advantage"].getvalue()
            train = json.loads(text.split("[distill] train ", 1)[1].splitlines()[0])
            self.assertIsNotNone(train["bce_adv_pairwise"])
            self.assertIsNone(train["ce_r"])
            epoch = json.loads(text.split("[distill] epoch ", 1)[1].splitlines()[0])
            self.assertEqual(set(epoch["train"]), set(D.TERMS))
            self.assertEqual(epoch["validation"]["val_adv_probed_rows"], 0)   # no held-out batch here
            self.assertIn("val_distill_pregate_start_act_ratio", epoch["validation"])
            final = json.loads((root / "advantage" / "meta.json").read_text())
            self.assertEqual(final["bundle_distillation"]["advantage"]["mode"], "pairwise")
            # A plain bundle run on the same data loads the probes (validation reads them) and trains as ever.
            plain = run(root / "plain", [*init, "--distill-bundle", "--checkpoint-selection", "distill_n_ne_r",
                                         "--train-policy-head-only"], labelled)
            self.assertNotIn("advantage", plain["bundle_distillation"])
            train = json.loads(logs["plain"].getvalue().split("[distill] train ", 1)[1].splitlines()[0])
            self.assertIsNone(train["bce_adv_pairwise"])
            self.assertIsNotNone(train["ce_r"])


if __name__ == "__main__":
    unittest.main()

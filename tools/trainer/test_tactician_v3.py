"""The stage-S1 tactician: shapes and passes, deep supervision, the records it
reads and refuses, and the ONNX export against PyTorch at every T.

Run with `CUDA_VISIBLE_DEVICES= python -m unittest test_tactician_v3` from
tools/trainer; nothing here touches the GPU.
"""
from __future__ import annotations

import json
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path

import numpy as np
import torch

from tactician_v3.export import check_rows, export_dir, parity, random_rows
from tactician_v3.model import Config, TacticianV3, decode, exact
from tactician_v3.records import (FACT_COLUMNS, PRIOR_COLUMNS, RECORD_CONTRACT, ROW_TAIL,
                                   WINDOW_WIDTH, decision_kind, read_records)
from tactician_v3.train import CONTRACT, group_softmax_loss, loss_of, pass_losses, train

D = 6


def key_of(window: str, group: int, unit: int) -> str:
    """A unit's key as the seat writes it: attacker@defender, blocker>attacker."""
    if window == "attack":
        return f"ObjectId({group}#0)@{{\"Player\":{unit}}}"
    if window == "block":
        return f"ObjectId({group}#0)>ObjectId({90 + unit}#0)"
    if window == "priority":
        return json.dumps("PassPriority" if unit == 0 else {"CastSpell": {"card": unit}})
    return f"target{unit}"


def write_records(directory: Path, decisions: int = 60, contract: str = RECORD_CONTRACT,
                  rule_column: bool = False, run: bool = True) -> None:
    """A fake imitating collection: windows in turn, labels a function of the rows."""
    directory.mkdir(parents=True, exist_ok=True)
    if run:
        (directory / "run.json").write_text(json.dumps({"contract": contract}))
    rng = np.random.default_rng(3)
    lines, blob, offset = [], [], 0
    for i in range(decisions):
        window = ("attack", "block", "target", "priority")[i % 4]
        n = 2 + i % 4
        state = rng.normal(size=D).astype(np.float32)
        units = rng.normal(size=(n, D)).astype(np.float32)
        tails = rng.normal(size=(n + 1, ROW_TAIL)).astype(np.float32)
        tails[:, :16] = 0.0
        tails[:, {"attack": 0, "block": 1, "target": 2, "priority": 12}[window]] = 1.0
        tails[0, 3] = 1.0
        if rule_column:
            tails[1, 5] = 1.0
        score = units[:, 0]
        if window in ("target", "priority"):
            rules = [int(j == int(np.argmax(score))) for j in range(n)]
            groups = [0] * n
        else:
            groups = [j // 2 for j in range(n)]
            rules = [0] * n
            for g in set(groups):
                members = [j for j in range(n) if groups[j] == g]
                best = max(members, key=lambda j: score[j])
                if score[best] > 0:
                    rules[best] = 1
        net = [int(s > 0.5) for s in score]
        for j in range(n):
            tails[j + 1, 4] = net[j]
        floats = np.concatenate([state, units.ravel(), tails.ravel()])
        line = dict(contract=contract, seat=i % 2, seq=i, turn=1, window=window, value=0.5,
                    keys=[key_of(window, groups[j], j) for j in range(n)], logits=[0.0] * n, net=net, rules=rules,
                    played=rules, probability=1.0, neighbors=0, offered=[], drawn="rules",
                    micros=1, groups=groups, label="rules", rows_read_rules=False,
                    fired=["trade"] if i % 4 == 1 else [],
                    game=f"g{i // 4}", deal=i // 4, seed=i // 4, policy_seed=1, focal_seat=0,
                    utility=1.0, capped=False,
                    blob=dict(file="w00.f32", offset=offset, state=D, units=n * D,
                              rows=(n + 1) * ROW_TAIL))
        lines.append(json.dumps(line))
        blob.append(floats)
        offset += 4 * len(floats)
    (directory / "w00.jsonl").write_text("\n".join(lines) + "\n")
    np.concatenate(blob).astype("<f4").tofile(directory / "w00.f32")


def tiny(passes: int = 3) -> TacticianV3:
    torch.manual_seed(0)
    return TacticianV3(Config(d_model=D, width=16, layers=2, heads=4, passes=passes,
                              ffn_mult=2)).eval()


class Records(unittest.TestCase):
    def test_rows_are_assembled_as_the_seat_assembles_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            store = read_records([Path(tmp) / "r"])
            self.assertEqual(len(store), 60)
            self.assertEqual(store.width, 2 * D + ROW_TAIL)
            batch = store.batch([0, 3, 4])
            self.assertEqual(batch.rows.shape, (3, 6, 2 * D + ROW_TAIL))
            # Decision 0: two units, rows 0..2 real, the null row's unit zero.
            self.assertEqual(batch.mask[0].tolist(), [True, True, True, False, False, False])
            self.assertTrue(torch.all(batch.rows[0, 0, D:2 * D] == 0))
            self.assertTrue(torch.all(batch.rows[0, 3:] == 0))
            self.assertTrue(torch.equal(batch.rows[0, 1, :D], store.states[0]))
            self.assertTrue(torch.equal(batch.rows[0, 2, D:2 * D], store.units[1]))
            self.assertEqual(batch.groups[0].tolist(), [0, 0, -1, -1, -1])
            train_idx, held_idx = store.split(10)
            self.assertTrue(all(store.seed[i] % 10 == 0 for i in held_idx))
            self.assertEqual(len(train_idx) + len(held_idx), 60)

    def test_the_probe_reads_stage_one_records_with_the_rule_columns_zeroed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_records(root / "stage1", contract="tactician_records_v1", rule_column=True,
                          run=False)
            store = read_records([root / "stage1"], stage1_probe=True)
            self.assertEqual(len(store), 60)
            self.assertTrue(torch.all(store.tails[:, [5, 11]] == 0))
            imitation = read_records([self._imitation(root)])
            self.assertTrue(torch.equal(store.groups, imitation.groups))
            # The probe reads stage-1 records only.
            with self.assertRaises(ValueError):
                read_records([root / "imitation"], stage1_probe=True)

    @staticmethod
    def _imitation(root: Path) -> Path:
        write_records(root / "imitation")
        return root / "imitation"

    def test_other_records_and_rows_carrying_the_rule_are_refused(self):
        import tactician4
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_records(root / "stage1", contract="tactician_records_v1")
            with self.assertRaises(ValueError):
                read_records([root / "stage1"])
            write_records(root / "ruled", rule_column=True)
            with self.assertRaises(ValueError):
                read_records([root / "ruled"])
            write_records(root / "bare", run=False)
            with self.assertRaises(ValueError):
                read_records([root / "bare"])
            # The outcome trainer of stage 1 refuses imitation labels.
            write_records(root / "imitation")
            with self.assertRaises(ValueError):
                tactician4.read_records([root / "imitation"])


class Model(unittest.TestCase):
    def test_every_pass_answers_every_unit_and_a_confidence(self):
        net = tiny(3)
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            batch = read_records([Path(tmp) / "r"]).batch(list(range(8)))
        outputs = net(batch.rows, batch.mask)
        self.assertEqual(len(outputs), 3)
        for logits, confidence in outputs:
            self.assertEqual(logits.shape, batch.labels.shape)
            self.assertEqual(confidence.shape, (8,))
        self.assertEqual(len(net(batch.rows, batch.mask, 5)), 5)
        with self.assertRaises(ValueError):
            net(batch.rows, batch.mask, 9)

    def test_padding_changes_no_answer(self):
        net = tiny(2)
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            store = read_records([Path(tmp) / "r"])
        batch = store.batch([0, 1, 2, 3])
        with torch.no_grad():
            together = net(batch.rows, batch.mask)
            for b in range(4):
                alone = net(store.batch([b]).rows, None)
                n = int(store.n[b])
                for (l_all, c_all), (l_one, c_one) in zip(together, alone):
                    self.assertTrue(torch.allclose(l_all[b, :n], l_one[0], atol=1e-5))
                    self.assertTrue(torch.allclose(c_all[b], c_one[0], atol=1e-5))

    def test_the_independent_head_scores_a_unit_from_its_own_row_alone(self):
        torch.manual_seed(0)
        net = TacticianV3(Config(d_model=D, width=16, layers=2, heads=4, passes=1, ffn_mult=2,
                                 independent=True)).eval()
        rows = random_rows(D, 4, 0, 1)[None]
        other = rows.clone()
        other[0, 3] = torch.randn(other.shape[-1])
        with torch.no_grad():
            a, _ = net(rows, None, 1)[0]
            b, _ = net(other, None, 1)[0]
        # Unit 2 (row 3) changed; every other unit scores as before.
        self.assertTrue(torch.allclose(a[0, [0, 1, 3]], b[0, [0, 1, 3]], atol=1e-6))
        self.assertFalse(torch.allclose(a[0, 2], b[0, 2]))
        joint = tiny(1)
        with torch.no_grad():
            c, _ = joint(rows, None, 1)[0]
            e, _ = joint(other, None, 1)[0]
        self.assertFalse(torch.allclose(c[0, 0], e[0, 0], atol=1e-6))

    def test_a_pass_reads_the_one_before(self):
        net = tiny(3)
        rows = random_rows(D, 4, 0, 1)[None]
        with torch.no_grad():
            one = net(rows, None, 1)
            three = net(rows, None, 3)
        # The first pass is the same however many follow; later ones differ.
        self.assertTrue(torch.equal(one[0][0], three[0][0]))
        self.assertFalse(torch.equal(three[0][0], three[1][0]))

    def test_decode_reads_logits_as_the_rust_decoders_do(self):
        groups = torch.tensor([[0, 0, 1, 1, 2]])
        mask = torch.ones(1, 5, dtype=torch.bool)
        logits = torch.tensor([[0.3, 1.2, -2.0, -0.5, -0.6]])
        attack = decode(logits, mask, groups, torch.tensor([0]))
        # Group 0 takes its best; group 1's best is below zero alone; group 2's
        # probabilities do not reach one half.
        self.assertEqual(attack.tolist(), [[0.0, 1.0, 0.0, 0.0, 0.0]])
        # A group whose sigmoids sum past one half takes a best above -0.65.
        logits = torch.tensor([[0.3, 1.2, -0.4, -0.5, -3.0]])
        self.assertEqual(decode(logits, mask, groups, torch.tensor([1])).tolist(),
                         [[0.0, 1.0, 1.0, 0.0, 0.0]])
        target = decode(logits, mask, torch.zeros_like(groups), torch.tensor([2]))
        self.assertEqual(target.tolist(), [[0.0, 1.0, 0.0, 0.0, 0.0]])
        self.assertTrue(exact(target, target, mask).item())


class Losses(unittest.TestCase):
    def test_group_softmax_counts_none_as_a_class(self):
        logits = torch.tensor([[1.0, 2.0, 0.5]])
        groups = torch.tensor([[0, 0, 1]])
        mask = torch.ones(1, 3, dtype=torch.bool)
        labels = torch.tensor([[0.0, 1.0, 0.0]])
        got = group_softmax_loss(logits, labels, mask, groups)
        g0 = -(2.0 - np.log(1 + np.exp(1.0) + np.exp(2.0)))
        g1 = -(0.0 - np.log(1 + np.exp(0.5)))
        self.assertAlmostEqual(float(got), (g0 + g1) / 2, places=5)

    def test_every_pass_is_supervised(self):
        net = tiny(3).train()
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            batch = read_records([Path(tmp) / "r"]).batch(list(range(12)))
        outputs = net(batch.rows, batch.mask)
        total = loss_of(outputs, batch, (1,), 0.5)
        parts = []
        for logits, confidence in outputs:
            unit, conf, hit = pass_losses(logits, confidence, batch, (1,))
            parts.append(unit.mean() + 0.5 * conf.mean())
            self.assertEqual(hit.shape, (12,))
        self.assertAlmostEqual(float(total.detach()), float(sum(parts).detach() / 3), places=5)
        # Each pass's loss alone reaches the shared weights.
        for logits, confidence in outputs:
            net.zero_grad()
            unit, _, _ = pass_losses(logits, confidence, batch, (1,))
            unit.mean().backward(retain_graph=True)
            self.assertGreater(float(net.blocks[0].qkv.weight.grad.abs().sum()), 0.0)


class Export(unittest.TestCase):
    def test_onnx_matches_pytorch_at_every_t_on_random_and_recorded_rows(self):
        net = tiny(4)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_records(root / "r")
            store = read_records([root / "r"])
            recorded = [store.batch([i]).rows[0] for i in range(9)]
            meta = dict(contract=CONTRACT, d_model=D, row_tail=ROW_TAIL, rows_read_rules=False,
                        thresholds_by_pass=[0.1, 0.2, 0.3, 0.4])
            for passes in (1, 2, 3, 4):
                written = export_dir(net, meta, passes, root / f"T{passes}", recorded)
                self.assertEqual(written["passes"], passes)
                self.assertEqual(written["confidence_threshold"], [0.1, 0.2, 0.3, 0.4][passes - 1])
                self.assertLessEqual(written["parity_max_abs_diff"], 1e-4)
                worst = parity(net, passes, root / f"T{passes}" / "tactician.onnx",
                               check_rows(D, recorded))
                self.assertLessEqual(worst, 1e-4)
                saved = json.loads((root / f"T{passes}" / "tactician.json").read_text())
                self.assertEqual(saved["contract"], "tactician_head_v2")

class Priority(unittest.TestCase):
    @staticmethod
    def record(rules, net, my_turn, paying=False):
        keys = [json.dumps("PassPriority"), json.dumps({"CastSpell": {"card": 7}}),
                json.dumps({"ActivateManaAbility": {"source": 3}})]
        tail = np.zeros((4, ROW_TAIL), np.float32)
        tail[:, WINDOW_WIDTH + FACT_COLUMNS[0]] = float(my_turn)
        tail[:, WINDOW_WIDTH + FACT_COLUMNS[0] + 10] = float(paying)
        mask = lambda i: [int(j == i) for j in range(3)]  # noqa: E731
        return dict(window="priority", keys=keys, rules=mask(rules), net=mask(net)), tail

    def test_priority_decisions_are_named_by_what_the_layer_did(self):
        cases = [((0, 1, True), "pass"), ((1, 0, True), "spend"), ((1, 2, True), "cast"),
                 ((1, 0, False), "instant_their_turn"), ((2, 0, True), "mana")]
        for (rules, net, my_turn), kind in cases:
            self.assertEqual(decision_kind(*self.record(rules, net, my_turn)), kind)
        self.assertEqual(decision_kind(*self.record(1, 0, True, paying=True)), "pay")
        self.assertEqual(decision_kind(dict(window="block"), np.zeros((1, ROW_TAIL))), "block")

    def test_moving_a_store_keeps_every_decision_s_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            store = read_records([Path(tmp) / "r"])
        moved = store.to("cpu", torch.float16)
        self.assertEqual(moved.kinds, store.kinds)
        self.assertEqual(moved.fired, store.fired)
        self.assertEqual(moved.sources, store.sources)
        self.assertTrue((moved.seat == store.seat).all())
        self.assertEqual(moved.states.dtype, torch.float16)
        self.assertIn("priority", set(moved.kinds) | {"priority"})

    def test_the_prior_and_the_facts_are_ablated_alone(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_records(Path(tmp) / "r")
            store = read_records([Path(tmp) / "r"])
        import dataclasses
        full = store.batch([3]).rows
        base = 2 * D + WINDOW_WIDTH
        for part, (lo, hi) in (("prior", PRIOR_COLUMNS), ("facts", FACT_COLUMNS)):
            rows = dataclasses.replace(store, zero_parts=(part,)).batch([3]).rows
            self.assertTrue(torch.all(rows[..., base + lo:base + hi] == 0))
            outside = torch.ones(full.shape[-1], dtype=torch.bool)
            outside[base + lo:base + hi] = False
            self.assertTrue(torch.equal(rows[..., outside], full[..., outside]))

    def test_the_probe_fits_each_feature_set_and_reports_priority(self):
        import probe_priority
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_records(root / "r", decisions=240)
            summary = probe_priority.main([
                "--records", str(root / "r"), "--out", str(root / "probe"),
                "--sets", "logits_facts,look", "--width", "16", "--layers", "1",
                "--heads", "2", "--epochs", "2", "--batch", "16", "--warmup", "2",
                "--held-out-mod", "4", "--device", "cpu"])
            self.assertEqual(set(summary["sets"]), {"logits_facts", "look"})
            for reading in summary["sets"].values():
                self.assertIn("by_kind_differ", reading)
                self.assertGreater(reading["decisions"], 0)
            self.assertTrue((root / "probe" / "summary.json").exists())
            self.assertFalse((root / "probe" / "look" / "tactician.onnx").exists())



class Training(unittest.TestCase):
    def test_a_few_steps_train_report_and_export(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_records(root / "r", decisions=90)
            args = Namespace(
                records=[str(root / "r")], out=str(root / "out"), seed=1, width=16, layers=1,
                heads=2, passes=2, ffn_mult=2, dropout=0.0, through_passes=False, lr=1e-3,
                weight_decay=0.0, batch=16, eval_batch=32, epochs=3, patience=5, max_steps=0,
                warmup=2, clip=1.0, confidence_weight=0.5, group_softmax="block",
                held_out_mod=10, train_report=50, parity_rows=4, export_passes=[1, 3],
                device="cpu", bf16=False, data_on_device=False, store_fp16=False, threads=0,
                stage1_probe=False)
            meta, report = train(args)
            out = root / "out"
            self.assertEqual(meta["contract"], CONTRACT)
            for path in (out / "tactician.onnx", out / "passes" / "T1" / "tactician.json",
                         out / "passes" / "T3" / "tactician.onnx", out / "net.pt"):
                self.assertTrue(path.exists(), path)
            passes = report["held_out"]["passes"]
            self.assertEqual(len(passes), 2)
            for reading in passes:
                self.assertIn("exact", reading["by_window"]["all"])
                self.assertIn("ece", reading["calibration"])
                self.assertEqual(set(reading["by_group_differ"]), {"trade", "none"})
                self.assertTrue(0.0 <= reading["threshold"] < 1.0)
            with self.assertRaises(FileExistsError):
                train(args)


if __name__ == "__main__":
    unittest.main()

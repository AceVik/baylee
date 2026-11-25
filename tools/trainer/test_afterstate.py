"""Arm V (`training4.afterstate`): the afterstate rows, creativity, the gated gaps, the loss's
weights, the readout against the model's own forward, the tower's split board, V0's fit and the
sealed decision."""
import contextlib
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import torch

from training4 import advantage as V
from training4 import afterstate as S
from training4.coarse import CLASSES

PASS, TAP, CAST, LAND = (CLASSES.index(c) for c in ("pass", "tap", "cast", "land"))


def rows_of(roots=300, seed=5, d=8):
    """Synthetic rows: 4 searched candidates per root (own first), held out by game id % 4."""
    rng = np.random.default_rng(seed)
    n = np.full(roots, 4)
    offset = np.concatenate(([0], np.cumsum(n)[:-1]))
    own = np.zeros(roots, dtype=np.int64)
    A = rng.normal(scale=.05, size=int(n.sum())).astype(np.float32)
    A[offset] = 0.
    cls = rng.integers(0, 5, size=int(n.sum()))
    game = np.arange(roots) // 2
    rows = V.AdvantageRows(
        dict(game=game, seat=np.zeros(roots, dtype=np.int64), cursor=np.arange(roots) * 10,
             kind=np.zeros(roots, dtype=np.int64), own=own, choice4=np.zeros(roots, dtype=np.int64),
             half_a=np.zeros(roots, dtype=np.int64), half_b=np.zeros(roots, dtype=np.int64), offset=offset, n=n),
        rng.normal(size=(roots, d)).astype(np.float16), rng.normal(size=(int(n.sum()), d)).astype(np.float16),
        dict(A=A, SE=np.full(int(n.sum()), .01, dtype=np.float32), searched=np.ones(int(n.sum()), dtype=bool),
             payable=np.zeros(int(n.sum()), dtype=bool), cls=cls),
        game % 4 == 0, ("pass",))
    return rows


def after_of(rows, d=4, valid=None, halves=None, card=None, intent=None, emb=None, v=None):
    n = len(rows.cand)
    joined = dict(valid=np.ones(n, dtype=bool) if valid is None else valid,
                  candidate=np.tile(np.arange(4), len(rows.state)), stop=np.zeros(n, dtype=np.int64),
                  card=np.full(n, S.NONE, dtype=np.int64) if card is None else card,
                  halves=np.full((n, 4), np.nan, dtype=np.float32) if halves is None else halves,
                  frame_cursor=np.arange(n, dtype=np.int64))
    emb = np.zeros((n, 3 * d), dtype=np.float16) if emb is None else emb
    v = np.zeros(n, dtype=np.float32) if v is None else v
    intent = np.full(len(rows.state), S.NONE, dtype=np.int64) if intent is None else intent
    return S.AfterRows(rows, joined, emb, v, v.copy(), intent, dict(model_sha256="x"))


class Creativity(unittest.TestCase):
    def test_a_confirmed_answer_of_another_class_is_non_prior_and_a_tap_merges_with_its_spell(self):
        rows = rows_of(roots=2)
        cls = rows.labels["cls"]
        cls[:] = [TAP, CAST, CAST, PASS, LAND, LAND, CAST, LAND]
        card = np.array([S.NONE, 7, 8, S.NONE, S.NONE, S.NONE, 9, S.NONE])
        halves = np.zeros((8, 4), dtype=np.float32)
        halves[1] = halves[2] = halves[3] = [.10, .01, .02, .01]     # confirmed on the even half
        halves[6] = [.05, .02, -.01, .01]                          # the other half is negative
        halves[7] = [np.nan, np.nan, .2, .01]
        after = after_of(rows, card=card, halves=halves, intent=np.array([7, S.NONE]))
        self.assertEqual(after.non_prior.tolist(), [False, False, True, True, False, False, True, False])
        self.assertEqual(after.confirmed.tolist(), [False, True, True, True, False, False, False, False])
        target, w, nu = after.weights()
        self.assertEqual(nu.tolist(), [1, 1, 2, 2, 1, 1, 1, 1])
        self.assertEqual(w[0], 0.)
        has, rate = S.discovery(after, np.ones(2, dtype=bool))
        self.assertEqual(has.tolist(), [True, False])
        self.assertEqual(rate, 50.)

    def test_the_target_is_clipped_and_missing_afterstates_score_zero(self):
        rows = rows_of(roots=2)
        rows.labels["A"][1] = .9
        valid = np.ones(8, dtype=bool)
        valid[2] = False      # one candidate without its afterstate
        valid[4] = False      # the second root's own answer without one
        after = after_of(rows, valid=valid)
        target, w, _ = after.weights()
        self.assertAlmostEqual(float(target[1]), S.CLIP)
        self.assertEqual(w[2], 0.)
        self.assertTrue((w[4:] == 0).all())
        gaps = after.gate(np.arange(8, dtype=np.float64) + 1.)
        self.assertEqual(gaps.tolist(), [1., 2., 0., 4., 0., 0., 0., 0.])
        # The rule still counts a missing afterstate, at gap 0, with arm B's weight.
        rule = after.rule_weights()
        self.assertEqual((rule[0], rule[4]), (0., 0.))
        self.assertTrue((rule[[1, 2, 3, 5, 6, 7]] > 0).all())

    def test_reproduction_counts_the_heads_plays_of_confirmed_discoveries(self):
        rows = rows_of(roots=2)
        rows.labels["cls"][:] = [TAP, CAST, LAND, PASS, TAP, CAST, LAND, PASS]
        rows.labels["A"][:] = [0, .1, .05, -.1, 0, .2, .1, 0]
        halves = np.tile(np.array([[.1, .01, .05, .01]], dtype=np.float32), (8, 1))
        after = after_of(rows, halves=halves)
        gaps = np.array([0, .2, .1, 0, 0, 0, .3, 0])
        out = S.reproduction(after, gaps, .05, np.ones(2, dtype=bool))
        self.assertEqual(out["discovery_roots"], 2)
        self.assertEqual(out["reproduced"], 2)
        self.assertAlmostEqual(out["captured_A_share"], (.1 + .1) / (.1 + .2))
        out = S.reproduction(after, gaps, .25, np.ones(2, dtype=bool))
        self.assertEqual(out["reproduced"], 1)


class Sidecar(unittest.TestCase):
    def test_the_sidecar_joins_each_searched_candidate_once(self):
        rows = rows_of(roots=3)
        index = np.zeros(12, dtype=S.SIDECAR_INDEX)
        index["game"] = np.repeat(rows.roots["game"], 4)
        index["root_cursor"] = np.repeat(rows.roots["cursor"], 4)
        index["item"] = np.tile(np.arange(4), 3)
        index["candidate"] = np.tile(np.arange(4), 3)
        index["frame_cursor"] = index["root_cursor"] * 8 + index["candidate"]
        index["flags"] = 1
        index["flags"][5] = 6
        sidecar = S.Sidecar(index[::-1].copy(), {}, None)
        joined = sidecar.join(rows)
        self.assertEqual(joined["valid"].sum(), 11)
        self.assertFalse(joined["valid"][5])
        self.assertEqual(joined["frame_cursor"][5], S.NONE)
        self.assertEqual(joined["frame_cursor"][6], 10 * 8 + 2)
        with self.assertRaises(ValueError):
            S.Sidecar(index[:-1].copy(), {}, None).join(rows)
        swapped = index.copy()
        swapped["candidate"][0], swapped["candidate"][1] = 1, 0
        with self.assertRaises(ValueError):
            S.Sidecar(swapped, {}, None).join(rows)


class Intents(unittest.TestCase):
    def test_the_intents_are_the_headers_file(self):
        sidecar = SimpleNamespace(
            index=np.zeros(3, dtype=V.SIDECAR_INDEX),
            header=dict(intents=dict(sha256=hashlib.sha256(np.arange(3, dtype="<u4").tobytes()).hexdigest())))
        sidecar.index["game"], sidecar.index["root_cursor"] = (0, 0, 1), (0, 10, 20)
        rows = rows_of(roots=3)
        with tempfile.TemporaryDirectory() as temporary, \
                patch.object(V.SearchAdvantage, "load", return_value=sidecar):
            directory = Path(temporary) / "search-advantage"
            directory.mkdir()
            np.arange(3, dtype="<u4").tofile(directory / "intents.bin")
            self.assertEqual(S.root_intents(temporary, rows).tolist(), [0, 1, 2])
            (np.arange(3, dtype="<u4") + 1).tofile(directory / "intents.bin")   # another file of the same length
            with self.assertRaisesRegex(ValueError, "digest"):
                S.root_intents(temporary, rows)
            sidecar.header = {}
            with self.assertRaisesRegex(ValueError, "digest"):
                S.root_intents(temporary, rows)

    def test_a_capture_never_overwrites_the_file_it_would_write(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertEqual(S.new_npz(root / "after.npz"), root / "after.npz")
            self.assertEqual(S.new_npz(root / "after.v2"), root / "after.v2.npz")
            (root / "after.npz").touch()
            for name in ("after", "after.npz"):   # np.savez writes after.npz for both
                with self.subTest(name), self.assertRaises(FileExistsError):
                    S.new_npz(root / name)


class Structures(unittest.TestCase):
    """A dataset's rule references index its own catalog: rows, afterstates and net share one."""

    def datasets(self, root, rows="catalog-a", after="catalog-a"):
        for name, digest in (("rows", rows), ("after", after)):
            (root / name).mkdir()
            (root / name / "meta.json").write_text(json.dumps({"structure_sha256": digest, S.SHARDS_KEY: ["s"]}))
        return root / "rows", root / "after"

    net = SimpleNamespace(rules=SimpleNamespace(digest="catalog-a"))

    def test_rows_afterstates_and_net_share_one_catalog(self):
        for rows, after, refused in (("catalog-a", "catalog-a", None),
                                     ("catalog-a", "catalog-b", "The rows' dataset .* different public rule catalogs"),
                                     ("catalog-b", "catalog-a", "The rows' dataset .* different public rule catalogs"),
                                     ("catalog-b", "catalog-b", "and the net use different public rule catalogs")):
            with self.subTest(rows=rows, after=after), tempfile.TemporaryDirectory() as temporary:
                data, after_dir = self.datasets(Path(temporary), rows, after)
                if refused is None:
                    self.assertEqual(S.check_structures(data, after_dir, self.net), "catalog-a")
                else:
                    with self.assertRaisesRegex(ValueError, refused):
                        S.check_structures(data, after_dir, self.net)

    def test_the_store_reads_only_the_nets_catalog(self):
        index = np.array([(7, 1, 80)], dtype=[("game", "<u4"), ("seat", "u1"), ("cursor", "<u4")])
        with tempfile.TemporaryDirectory() as temporary, \
                patch("trajectory_store4.TrajectoryStore4", return_value=SimpleNamespace(index=index)) as store, \
                patch("dataset4.TrajectoryDataset4") as dataset:
            data, after_dir = self.datasets(Path(temporary), after="catalog-b")
            with self.assertRaisesRegex(ValueError, "different public rule catalogs"):
                S.FrameStore(after_dir, self.net)
            store.assert_not_called()
            dataset.assert_not_called()
            frames = S.FrameStore(data, self.net)
            store.assert_called_once_with(data, S.FRAMES_SPLIT, ["s"])
            self.assertEqual(frames.rows_of([7], [1], [80]).tolist(), [0])
            with self.assertRaisesRegex(ValueError, "missing"):
                frames.rows_of([7], [1], [81])

    def test_the_capture_and_the_tower_refuse_before_they_decode(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            data, after_dir = self.datasets(root, after="catalog-b")
            rows = rows_of(roots=4)
            rows.save(root / "rows.npz", data=str(data), model_sha256="net")
            args = SimpleNamespace(threads=1, device="cpu", model="m", rows=root / "rows.npz", state_dir=str(root),
                                   afterstates=str(after_dir), out=str(root / "after.npz"), batch=4, smoke_candidates=0)
            with patch("value_calibration4.checkpoint.load_source",
                       return_value=("m", {}, SimpleNamespace(rules=self.net.rules))), \
                    patch("torch_io4.file_digest", return_value="net"), \
                    patch("training4.advantage_tower.Roots.load", side_effect=AssertionError("decoded")):
                with self.assertRaisesRegex(ValueError, "different public rule catalogs"):
                    S.capture(args)
                with self.assertRaisesRegex(ValueError, "different public rule catalogs"):
                    S.TowerData(after_of(rows), [root / "rows.npz"], [root], [after_dir], self.net, "net")
                with self.assertRaisesRegex(ValueError, "One --state-dir and one --after-data"):
                    S.TowerData(after_of(rows), [root / "rows.npz"], [root], [], self.net, "net")
                # The same catalog: past the check, to the roots.
                (after_dir / "meta.json").write_text((data / "meta.json").read_text())
                with self.assertRaisesRegex(AssertionError, "decoded"):
                    S.TowerData(after_of(rows), [root / "rows.npz"], [root], [after_dir], self.net, "net")

    def test_captures_of_two_catalogs_do_not_pool(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows = rows_of(roots=4)
            joined = {name: getattr(after_of(rows), name) for name in S.PER_CANDIDATE}
            for name in ("a", "b"):
                rows.save(root / f"rows-{name}.npz", data=str(root), model_sha256="net")
            files = [root / "rows-a.npz", root / "rows-b.npz"], [root / "after-a.npz", root / "after-b.npz"]

            def capture(path, rows_path, digest):
                n = len(rows.cand)
                meta = dict(contract=S.ROWS_CONTRACT, rows_sha256=S.sha256(rows_path), model_sha256="net",
                            structure_sha256=digest)
                np.savez(path, meta=json.dumps(meta), emb=np.zeros((n, 12), np.float16), v=np.zeros(n, np.float32),
                         v0=np.zeros(n, np.float32), intent=np.full(len(rows.state), S.NONE), **joined)

            for rows_path, path in zip(*files):
                capture(path, rows_path, "catalog-a")
            self.assertEqual(len(S.AfterRows.load(*files, halves_judged=False).v), 2 * len(rows.cand))
            capture(files[1][1], files[0][1], "catalog-b")
            with self.assertRaisesRegex(ValueError, "another rule catalog"):
                S.AfterRows.load(*files, halves_judged=False)


def tiny_net(layers=4):
    from model4 import Config4, PlayerNet4
    torch.manual_seed(0)
    net = PlayerNet4(Config4(d_model=32, n_layers=layers, n_heads=4, d_ff=64))
    return net.eval()


def board_batch(b=3, e=5, seed=0):
    from model4.config import (ENTITY_FEAT_WIDTH, CARD_DSL_WIDTH, SEAT_FEAT_WIDTH, GLOBAL_FEAT_WIDTH,
                               SEAT_FEAT_PRESENT, SEAT_FEAT_SELF)
    g = torch.Generator().manual_seed(seed)
    seat = torch.zeros(b, 8, SEAT_FEAT_WIDTH)
    seat[:, :2, SEAT_FEAT_PRESENT] = 1.
    seat[:, 0, SEAT_FEAT_SELF] = 1.
    return [torch.randn(b, e, ENTITY_FEAT_WIDTH, generator=g), torch.randn(b, e, CARD_DSL_WIDTH, generator=g),
            torch.ones(b, e, dtype=torch.bool), seat, torch.randn(b, GLOBAL_FEAT_WIDTH, generator=g),
            torch.zeros(b, 2, 2, dtype=torch.long), torch.zeros(b, 2, dtype=torch.long),
            torch.zeros(b, 2, dtype=torch.bool), torch.zeros(b, e, 1, 2, dtype=torch.long)]


class Readout(unittest.TestCase):
    def test_the_readout_is_the_models_own_observation_then_decision(self):
        net = tiny_net()
        inputs = board_batch()
        b = inputs[0].shape[0]
        with torch.no_grad():
            board, pad = net.encode_board(*inputs)
            hidden = torch.randn(b, 9, 32)
            v, v0, emb = S.readout(net, board, pad, hidden, inputs[3])
            cand = dict(cand_feat=torch.zeros(b, 1, 48), cand_entity_idx=torch.full((b, 1), -1),
                        cand_seat_idx=torch.full((b, 1), -1), cand_mask=torch.zeros(b, 1, dtype=torch.bool),
                        pair_index=torch.zeros(b, 1, 2, dtype=torch.long), pair_mask=torch.zeros(b, 1, dtype=torch.bool),
                        cand_dsl=torch.zeros(b, 1, 160), cand_rules=torch.zeros(b, 1, 1, 2, dtype=torch.long))
            run = lambda h, k: net.forward_with_seat_logits(
                *inputs[:8], cand["cand_feat"], cand["cand_entity_idx"], cand["cand_seat_idx"], cand["cand_mask"],
                cand["pair_index"], cand["pair_mask"], cand["cand_dsl"], inputs[8], cand["cand_rules"], h,
                torch.full((b,), k, dtype=torch.long), encoded_board=(board, pad), policy_heads=(False,) * 4)
            observed, *_, value_k0, _ = run(hidden, 0)
            _, *_, value_k2, _ = run(observed, 2)
        torch.testing.assert_close(v0, value_k0)
        torch.testing.assert_close(v, value_k2)
        self.assertEqual(tuple(emb.shape), (b, 96))

    def test_the_towers_board_is_r1s_lower_layers_then_its_own(self):
        net = tiny_net(layers=12)
        tower = S.make_tower(net)
        inputs = board_batch()
        with torch.no_grad():
            want, _ = net.encode_board(*inputs)
        got, _ = S.TowerBoard(net, tower)(inputs)
        torch.testing.assert_close(got, want)
        self.assertTrue(got.requires_grad, "the upper layers carry gradients")
        self.assertEqual(len(net.trunk_layers), 12, "r1 keeps its layers")
        self.assertFalse(any(p.requires_grad for p in net.parameters()))
        trainable = {n for n, p in tower.named_parameters() if p.requires_grad}
        self.assertTrue(all(any(n.startswith(m + ".") for m in S.V1_MODULES) for n in trainable))
        self.assertTrue(any(n.startswith("trunk_layers.11.") for n in trainable))


class FitTower(unittest.TestCase):
    def test_v1_trains_the_towers_modules_and_r1_stays_as_it_was(self):
        torch.set_num_threads(1)
        net = tiny_net(layers=12)
        rows = rows_of(roots=24)
        n, root_of = len(rows.cand), rows.root_of()
        boards, hidden = board_batch(b=n, seed=3), torch.randn(len(rows.state), 9, 32)
        with torch.no_grad():
            board, pad = net.encode_board(*boards)
            v, _, emb = S.readout(net, board, pad, hidden[torch.from_numpy(root_of)], boards[3])
        valid = np.ones(n, dtype=bool)
        valid[[2, 5, 8]] = False   # two candidates and root 2's own answer
        after = after_of(rows, d=32, valid=valid, emb=emb.numpy().astype(np.float16), v=v.numpy())

        def collate(frames):
            host = {name: value[torch.tensor(frames)][:, None] for name, value in zip(S.BOARD_INPUTS, boards)}
            return dict(host, cand_rules=torch.zeros(len(frames), 1, 1, 1, 2, dtype=torch.long))

        before = {k: x.clone() for k, x in net.state_dict().items()}
        data = SimpleNamespace(hidden=hidden, frames={i: i for i in range(n) if after.scored[i]})
        with patch.object(S, "collate", collate):
            best, gaps, extra = S.fit_v1(after, data, net, torch.device("cpu"), tower_lr=1e-3, epochs=2, batch=8,
                                         minutes=0, seed=1, log=lambda *_: None)
        self.assertTrue(all(torch.equal(before[k], x) for k, x in net.state_dict().items()))
        self.assertEqual(len(net.trunk_layers), 12)
        self.assertFalse(any(p.grad is not None for p in net.parameters()))
        self.assertLess(extra["reproduce"]["max_abs"], 1e-5)
        moved = {k for k in best["tower"] if not torch.equal(best["tower"][k], before[k])}
        for module in S.V1_MODULES:
            self.assertTrue(any(k.startswith(module + ".") for k in moved), module)
        held = rows.held_out[root_of]
        self.assertTrue((gaps[held & after.scored] != 0).any())
        self.assertTrue((gaps[~held] == 0).all() and (gaps[~after.scored] == 0).all())
        self.assertLessEqual(extra["best_select_loss"], max(h["select"] for h in extra["history"]))


class FitAndDecide(unittest.TestCase):
    def synthetic_after(self, roots=400):
        """Afterstate embeddings that carry the advantage: e(s'_i) - e(s'_own) is linear in A."""
        rows = rows_of(roots=roots)
        rng = np.random.default_rng(1)
        d = 4
        emb = rng.normal(size=(len(rows.cand), 3 * d)).astype(np.float32)
        root_of = rows.root_of()
        base = emb[rows.roots["offset"]][root_of]
        emb[:, 0] = base[:, 0] + rows.labels["A"] * 20
        return after_of(rows, d=d, emb=emb.astype(np.float16))

    def test_a_missing_afterstates_nan_readout_never_reaches_the_head(self):
        rows = rows_of(roots=40)
        valid = np.ones(len(rows.cand), dtype=bool)
        valid[1::5] = False
        v = np.where(valid, .5, np.nan).astype(np.float32)
        emb = np.where(valid[:, None], 1., np.nan).astype(np.float16) * np.ones((len(rows.cand), 12), np.float16)
        after = after_of(rows, valid=valid, v=v, emb=emb)
        self.assertTrue(np.isfinite(after.v).all() and np.isfinite(after.emb.astype(np.float32)).all())
        with contextlib.redirect_stdout(io.StringIO()):
            _, gaps, extra = S.fit_v0(after, epochs=2, batch=8, seed=1, log=lambda *_: None)
        self.assertTrue(np.isfinite(gaps).all())
        self.assertTrue(all(np.isfinite(h["select"]) for h in extra["history"]))

    def test_v0_learns_the_gap_and_its_fit_keeps_the_report_half_sealed(self):
        after = self.synthetic_after()
        with contextlib.redirect_stdout(io.StringIO()):
            head, gaps, extra = S.fit_v0(after, epochs=30, batch=64, patience=30, seed=1, log=lambda *_: None)
        select, final = V.held_out_halves(after.rows)
        chooser, margin, gain = S.selection_report(after, after.gate(gaps), select)
        self.assertGreater(chooser["pearson_pooled"], .8)
        with tempfile.TemporaryDirectory() as temporary:
            prereg = Path(temporary) / "prereg.json"
            prereg.write_text(json.dumps(dict(contract=S.PREREGISTRATION_CONTRACT, rule=dict(
                pearson_pooled_min=.4, gain_per_100_roots_min=.2, z_min=2.))))
            args = type("Args", (), dict(preregistration=str(prereg), seed=1))
            digest = S.read_preregistration(prereg)[1]
            result = S.seal(Path(temporary) / "V0", gaps, S.stage_result("V0", after, gaps, extra, args, digest))
            text = (Path(temporary) / "V0" / "fit.json").read_text()
            self.assertNotIn("held_out", json.loads(text))
            self.assertIn("falsifier", result)
            # A second stage with no skill: decide picks V0 on the selection half and judges it alone.
            S.seal(Path(temporary) / "V1", np.zeros(len(gaps)), S.stage_result("V1", after, np.zeros(len(gaps)), {},
                                                                              args, digest))
            with contextlib.redirect_stdout(io.StringIO()):
                out = S.decide(prereg, [("V0", Path(temporary) / "V0"), ("V1", Path(temporary) / "V1")], after,
                               Path(temporary) / "decision.json")
            self.assertEqual(out["chosen_stage"], "V0")
            self.assertEqual([S.falsified(value) for value in (None, float("nan"), .25, .26)],
                             [True, True, True, False])
            self.assertGreater(out["verdict"]["pearson_pooled"], .8)
            self.assertIn("first_four", out["post_hoc_not_preregistered"])
            self.assertEqual((list(out["stages"]), out["left_out"]), (["V0", "V1"], []))
            # The report half is read once: an existing decision stays.
            kept_decision = (Path(temporary) / "decision.json").read_text()
            with self.assertRaisesRegex(FileExistsError, "read once"):
                S.decide(prereg, [("V0", Path(temporary) / "V0")], after, Path(temporary) / "decision.json")
            self.assertEqual((Path(temporary) / "decision.json").read_text(), kept_decision)
            # A stage without its outputs is no candidate and is named.
            with contextlib.redirect_stdout(io.StringIO()) as said:
                out = S.decide(prereg, [("V0", Path(temporary) / "V0"), ("V1", Path(temporary) / "missing")], after,
                               Path(temporary) / "alone.json")
            self.assertEqual([stage["stage"] for stage in out["left_out"]], ["V1"])
            self.assertEqual(out["stages"]["V0"], dict(stage="V0", seed=1, directory=str(Path(temporary) / "V0")))
            self.assertIn("leaves out stage V1", said.getvalue())
            with self.assertRaisesRegex(ValueError, "named twice"):
                S.decide(prereg, [("V0", Path(temporary) / "V0")] * 2, after, Path(temporary) / "twice.json")
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(ValueError, "No stage produced"):
                S.decide(prereg, [("V1", Path(temporary) / "missing")], after, Path(temporary) / "none.json")
            # A fit of another contract is refused.
            other = Path(temporary) / "other-contract"
            S.seal(other, gaps, dict(result, contract="search_advantage_tower_v1"))
            with self.assertRaisesRegex(ValueError, "search_advantage_tower_v1"):
                S.decide(prereg, [("V0", other)], after, Path(temporary) / "other.json")
            self.assertFalse((Path(temporary) / "other.json").exists())
            # The gaps belong to the afterstate files they were fit on, in their order.
            kept = dict(after.meta)
            after.meta["sources"] = [dict(sha256="another file")]
            with self.assertRaisesRegex(ValueError, "other afterstate files"):
                S.decide(prereg, [("V0", Path(temporary) / "V0")], after, Path(temporary) / "other.json")
            after.meta.clear()
            after.meta.update(kept)
            # A fit under another pre-registration is refused.
            prereg.write_text(prereg.read_text() + " ")
            with self.assertRaises(ValueError):
                S.decide(prereg, [("V0", Path(temporary) / "V0")], after, Path(temporary) / "again.json")

    def test_a_fit_without_a_selection_half_is_refused(self):
        rows = rows_of(roots=40)
        rows.held_out[:] = False
        after = after_of(rows)
        with self.assertRaisesRegex(ValueError, "selection half is empty"):
            S.fit_v0(after, epochs=1, log=lambda *_: None)
        with self.assertRaisesRegex(ValueError, "selection half is empty"):
            S.fit_v1(after, None, None, torch.device("cpu"), log=lambda *_: None)
        with self.assertRaisesRegex(ValueError, "selection half is empty"):
            S.stage_result("V0", after, np.zeros(len(rows.cand)), {}, SimpleNamespace(preregistration="p", seed=1), "d")
        # Held-out games, but nothing searched besides their own answers: no loss either.
        rows = rows_of(roots=40)
        rows.labels["searched"][np.repeat(rows.held_out, 4) & (np.tile(np.arange(4), 40) > 0)] = False
        after = after_of(rows)
        select, _ = V.held_out_halves(rows)
        with self.assertRaisesRegex(ValueError, "no loss to select on"):
            S.pairwise(after, np.zeros(len(rows.cand)), select)
        with self.assertRaisesRegex(ValueError, "no loss to select on"):
            S.fit_v0(after, epochs=1, log=lambda *_: None)

    def test_a_decision_without_a_report_half_is_refused(self):
        after = self.synthetic_after(roots=40)
        after.rows.held_out[:] = after.rows.roots["game"] == 4   # one held-out game: all of it selects
        with tempfile.TemporaryDirectory() as temporary:
            prereg = Path(temporary) / "prereg.json"
            prereg.write_text(json.dumps(dict(contract=S.PREREGISTRATION_CONTRACT, rule={})))
            args = SimpleNamespace(preregistration=str(prereg), seed=1)
            gaps = np.zeros(len(after.rows.cand))
            S.seal(Path(temporary) / "V0", gaps,
                   S.stage_result("V0", after, gaps, {}, args, S.read_preregistration(prereg)[1]))
            with self.assertRaisesRegex(ValueError, "report half is empty"):
                S.decide(prereg, [("V0", Path(temporary) / "V0")], after, Path(temporary) / "decision.json")


if __name__ == "__main__":
    unittest.main()

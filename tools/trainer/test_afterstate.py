"""Arm V (`training4.afterstate`): the afterstate rows, creativity, the gated gaps, the loss's
weights, the readout against the model's own forward, the tower's split board, V0's fit and the
sealed decision."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

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
            self.assertGreater(out["verdict"]["pearson_pooled"], .8)
            self.assertIn("first_four", out["post_hoc_not_preregistered"])
            # A fit under another pre-registration is refused.
            prereg.write_text(prereg.read_text() + " ")
            with self.assertRaises(ValueError):
                S.decide(prereg, [("V0", Path(temporary) / "V0")], after, Path(temporary) / "again.json")


if __name__ == "__main__":
    unittest.main()

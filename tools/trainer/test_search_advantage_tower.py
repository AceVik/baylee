"""Arm B of the search head (`training4.advantage_tower`) on a tiny net: the tower, its pass,
its weights, a fit end to end, and the pre-registered decision."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

from training4 import advantage as V
from training4 import advantage_tower as T


def tiny_net():
    import model5 as V5
    torch.manual_seed(4)
    net = V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64, structure_width=16,
                                   relational_width=16, relational_revision=2, candidate_context_revision=1)).eval()
    net.decision_chunk, net.prune_unused_policy_heads = 2, False
    return net


def frames_of(b, seed=5):
    """`b` decision frames with full menus, the candidates' features varied per frame."""
    from test_training_execution4 import batch_fixture
    batch = batch_fixture(b=b, u=1)
    batch["is_decision"][:] = True
    batch["step_valid"][:] = True
    batch["cand_mask"][:] = True
    generator = torch.Generator().manual_seed(seed)
    batch["cand_feat"] = batch["cand_feat"] + torch.randn(batch["cand_feat"].shape, generator=generator)
    return [{name: value[i, 0].numpy() for name, value in batch.items() if value.dim() >= 2} for i in range(b)]


def rows_for(net, frames, hidden, seed=6):
    """Rows captured from `net` on these frames (as the capture would), with planted labels."""
    import model4 as M  # noqa: F401
    rng = np.random.default_rng(seed)
    state, cands = V.RootPass(net, 2, 2)(frames, hidden)
    r = len(frames)
    n = np.array([c.shape[0] for c in cands])
    offset = np.concatenate(([0], np.cumsum(n)[:-1]))
    A = rng.normal(scale=.05, size=int(n.sum())).astype(np.float32)
    own = np.zeros(r, dtype=np.int64)
    A[offset + own] = 0.
    none = np.full(r, V.NONE, dtype=np.int64)
    rows = V.AdvantageRows(
        dict(game=np.arange(r), seat=np.zeros(r, dtype=np.int64), cursor=np.arange(r), kind=np.zeros(r, dtype=np.int64),
             own=own, choice4=none, half_a=none, half_b=none, offset=offset, n=n),
        state.half().numpy(), torch.cat(cands).half().numpy(),
        dict(A=A, SE=np.full(len(A), .01, dtype=np.float32), searched=np.ones(len(A), dtype=bool),
             payable=np.zeros(len(A), dtype=bool), cls=np.zeros(len(A), dtype=np.int64)),
        np.arange(r) % 4 == 0, ("tap",))
    return rows


class SearchAdvantageTower(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)

    def test_only_the_arms_modules_train_and_r1_stays_frozen(self):
        net = tiny_net()
        for arm, modules in T.ARMS.items():
            tower = T.make_tower(net, arm)
            trainable = {name for name, p in tower.named_parameters() if p.requires_grad}
            self.assertTrue(trainable)
            self.assertTrue(all(any(name.startswith(m + ".") for m in modules) for name in trainable))
            self.assertFalse(any(p.requires_grad for p in net.parameters()))
            self.assertFalse(tower.cfg.activation_checkpointing)
        self.assertLess(sum(p.numel() for p in T.make_tower(net, "B1").parameters() if p.requires_grad),
                        sum(p.numel() for p in T.make_tower(net, "B2").parameters() if p.requires_grad))
        with self.assertRaises(ValueError):
            T.make_tower(net, "B9")

    def test_untrained_tower_reproduces_the_root_pass_and_trains_only_its_arm(self):
        import model4 as M
        net = tiny_net()
        frames, hidden = frames_of(3), torch.randn(3, M.RECURRENT_SLOTS, 32)
        want_state, want_cands = V.RootPass(net, 2, 2)(frames, hidden)
        for arm in ("B1", "B2"):
            tower = T.make_tower(net, arm).train()
            state, cands = T.TowerPass(net, tower, 2, 2)(frames, hidden)
            torch.testing.assert_close(state.detach(), want_state, atol=1e-5, rtol=1e-5)
            for got, want in zip(cands, want_cands):
                torch.testing.assert_close(got.detach(), want, atol=1e-5, rtol=1e-5)
            (state.square().sum() + sum(c.square().sum() for c in cands)).backward()
            moved = {name.split(".")[0] for name, p in tower.named_parameters()
                     if p.grad is not None and p.grad.abs().sum() > 0}
            self.assertTrue(moved)
            self.assertTrue(moved <= {m.split(".")[0] for m in T.ARMS[arm]}, (arm, moved))
            self.assertEqual(arm == "B2", "thinking_block" in moved)
            self.assertFalse(any(p.grad is not None for p in net.parameters()))

    def test_trainable_weights_round_trip_and_refuse_other_names(self):
        net = tiny_net()
        tower = T.make_tower(net, "B1")
        state = T.trainable_state(tower, "B1")
        self.assertEqual({k.split(".")[0] for k in state}, {"cand_scorer", "cand_ctx_proj"})
        with torch.no_grad():
            tower.cand_ctx_proj.weight.add_(1.)
        T.load_trainable(tower, "B1", state)
        torch.testing.assert_close(tower.cand_ctx_proj.weight, state["cand_ctx_proj.weight"])
        with self.assertRaises(ValueError):
            T.load_trainable(tower, "B1", dict(state, extra=torch.zeros(1)))

    def test_fit_trains_the_tower_and_reports_like_the_frozen_head(self):
        import model4 as M
        net = tiny_net()
        frames = frames_of(16)
        hidden = torch.randn(16, M.RECURRENT_SLOTS, 32)
        rows = rows_for(net, frames, hidden)
        roots = T.Roots(frames, hidden)
        before = {k: v.clone() for k, v in net.state_dict().items()}
        tower, head, state, result = T.fit(rows, roots, net, "B1", torch.device("cpu"), kind="linear", epochs=3,
                                           batch=4, patience=5, seed=1, tower_lr=1e-2, head_lr=1e-2, log=lambda *_: None)
        for k, v in net.state_dict().items():
            torch.testing.assert_close(v, before[k])   # r1 never moves
        self.assertGreater(result["reproduce"]["cand_cosine"], .999)
        self.assertEqual(len(result["history"]), 3)
        self.assertIn(result["margin"], V.MARGINS)
        self.assertEqual(result["held_out"]["roots"], 2)   # held out: games 0, 4, 8, 12; report: 4 and 12
        self.assertEqual(result["margin_selection"]["roots"] + result["held_out"]["roots"], 4)
        self.assertIn("overrides", result["held_out"]["by_kind"]["tap"])
        self.assertTrue(any(not torch.equal(v, before[k]) for k, v in state["tower"].items()))
        with self.assertRaises(ValueError):   # a tower that no longer gives the captured embeddings back
            bad = rows_for(net, frames, hidden)
            bad.state[:] = 0
            T.fit(bad, roots, net, "B1", torch.device("cpu"), epochs=1, batch=4, seed=1, log=lambda *_: None)

    def test_decide_picks_the_arm_on_the_selection_half_and_judges_it_once(self):
        def result(select_gain, pearson, gain, se, digest):
            margins = {str(m): dict(gain_per_100_roots=0., se=None, overrides=0) for m in V.MARGINS}
            chosen = dict(margins, **{"0.01": dict(gain_per_100_roots=select_gain, se=.01, overrides=5)})
            held = dict(margins, **{"0.01": dict(gain_per_100_roots=gain, se=se, overrides=9)})
            return dict(preregistration_sha256=digest, margin=0.01, margin_selection=dict(margins=chosen),
                        held_out=dict(pearson_pooled=pearson, margins=held),
                        held_out_by_source={"0": dict(pearson_pooled=pearson, margins=held, search_halves={})})
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()):
            folder = Path(temporary)
            prereg = folder / "prereg.json"
            prereg.write_text(json.dumps(dict(contract=T.PREREGISTRATION_CONTRACT,
                                              rule=dict(pearson_pooled_min=.4, gain_per_100_roots_min=.2, z_min=2.))))
            digest = T.read_preregistration(prereg)[1]
            for name, data in (("b1", result(.3, .45, .25, .1, digest)), ("b2", result(.5, .6, .9, .2, digest)),
                               ("b3", result(.3, .45, .25, .1, "other"))):
                (folder / name).mkdir()
                (folder / name / "advantage_tower.json").write_text(json.dumps(data))
            out = T.decide(prereg, [("B1", folder / "b1"), ("B2", folder / "b2")], folder / "decision.json")
            self.assertEqual(out["chosen_arm"], "B2")
            self.assertTrue(out["verdict"]["passed"])   # 0.9 per 100, z 4.5, Pearson 0.6
            out = T.decide(prereg, [("B1", folder / "b1")], folder / "decision.json")
            self.assertEqual(out["verdict"]["checks"], dict(pearson=True, gain=True, z=True))
            (folder / "b2" / "advantage_tower.json").write_text(json.dumps(result(.3, .45, .25, .2, digest)))
            out = T.decide(prereg, [("B1", folder / "b1"), ("B2", folder / "b2")], folder / "decision.json")
            self.assertEqual(out["chosen_arm"], "B1")   # a tie on the selection half: the first listed
            self.assertFalse(out["informational"]["B2"]["passed"])   # z 1.25
            with self.assertRaises(ValueError):   # fit under another pre-registration
                T.decide(prereg, [("B3", folder / "b3")], folder / "decision.json")


if __name__ == "__main__":
    unittest.main()

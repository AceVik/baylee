"""The search head (`training4.advantage`): rows, the weighted pairwise loss, held-out report, export."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

from training4 import advantage as V

KINDS = ("tap", "land", "pass")


def synthetic(roots=600, d=16, seed=3, noise=.01):
    """Advantages linear in the candidate state (relative to the bundle's own), observed with
    noise of the stated SE; one root in five held out by game."""
    rng = np.random.default_rng(seed)
    truth = rng.normal(size=d) * .05
    n = rng.integers(3, 8, size=roots)
    offset = np.concatenate(([0], np.cumsum(n)[:-1]))
    cand = rng.normal(size=(int(n.sum()), d)).astype(np.float16)
    own = rng.integers(0, n)
    value = cand.astype(np.float64) @ truth
    A = np.zeros(len(cand))
    choice4, half_a, half_b = np.zeros(roots, dtype=np.int64), np.zeros(roots, dtype=np.int64), np.zeros(roots, dtype=np.int64)
    for r in range(roots):
        lo = offset[r]
        true = value[lo:lo + n[r]] - value[lo + own[r]]
        A[lo:lo + n[r]] = true + rng.normal(scale=noise, size=n[r])
        A[lo + own[r]] = 0.
        best = int(np.argmax(true))
        choice4[r] = best if true[best] > 0 else own[r]
        half_a[r], half_b[r] = int(np.argmax(true + rng.normal(scale=.05, size=n[r]))), int(np.argmax(true + rng.normal(scale=.05, size=n[r])))
    cls = rng.integers(0, 6, size=len(cand))
    rows = V.AdvantageRows(
        dict(game=np.arange(roots) // 3, seat=np.zeros(roots, dtype=np.int64), cursor=np.arange(roots),
             kind=rng.integers(0, len(KINDS), size=roots), own=own, choice4=choice4, half_a=half_a, half_b=half_b,
             offset=offset, n=n),
        rng.normal(size=(roots, d)).astype(np.float16), cand,
        dict(A=A.astype(np.float32), SE=np.full(len(cand), noise, dtype=np.float32), searched=np.ones(len(cand), dtype=bool),
             payable=rng.random(len(cand)) < .2, cls=cls),
        (np.arange(roots) // 3) % 5 == 0, KINDS)
    return rows


class SearchAdvantage(unittest.TestCase):
    def test_rows_validate_round_trip_and_concatenate(self):
        rows = synthetic(roots=30)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rows.npz"
            rows.save(path, source="synthetic")
            again = V.AdvantageRows.load(path)
        np.testing.assert_array_equal(again.cand, rows.cand)
        self.assertEqual(again.kinds, KINDS)
        both = V.AdvantageRows.concat([rows, again])
        self.assertEqual(len(both.cand), 2 * len(rows.cand))
        np.testing.assert_array_equal(both.cand[both.roots["offset"][30]:][:3], rows.cand[:3])
        bad = dict(rows.labels, searched=rows.labels["searched"].copy())
        bad["searched"][rows.roots["offset"][0] + rows.roots["own"][0]] = False
        with self.assertRaises(ValueError):   # the baseline must be searched
            V.AdvantageRows(rows.roots, rows.state, rows.cand, bad, rows.held_out, KINDS)
        with self.assertRaises(ValueError):
            V.AdvantageRows(dict(rows.roots, own=rows.roots["n"]), rows.state, rows.cand, rows.labels)

    def test_pairwise_loss_is_the_weighted_gap_error_over_searched_others(self):
        rows = synthetic(roots=2)
        prepared = V.Prepared(rows, tau=.02)
        scores = torch.randn(len(rows.cand))
        chosen = torch.tensor([True, False])
        lo, n, own = (int(rows.roots[k][0]) for k in ("offset", "n", "own"))
        gaps = scores[lo:lo + n] - scores[lo + own]
        w = 1. / (.01 ** 2 + .02 ** 2)
        others = [i for i in range(n) if i != own]
        expected = sum(w * (gaps[i] - float(rows.labels["A"][lo + i])) ** 2 for i in others) / (w * len(others))
        self.assertAlmostEqual(float(V.pairwise_loss(prepared, scores, chosen)), float(expected), places=4)

    def test_ranks_share_ties_and_correlation_weights(self):
        np.testing.assert_array_equal(V.ranks(np.array([3., 1., 3., 2.])), [2.5, 0., 2.5, 1.])
        self.assertAlmostEqual(V.correlation(np.arange(5.), 2 * np.arange(5.) + 1), 1.)
        self.assertIsNone(V.correlation(np.ones(4), np.arange(4.)))

    def test_linear_head_recovers_a_planted_advantage_on_held_out_games(self):
        rows = synthetic()
        head, result = V.fit(rows, kind="linear", lr=3e-2, epochs=60, batch=64, patience=10, seed=1,
                             log=lambda *_: None)
        held = result["held_out"]
        self.assertGreater(held["spearman_per_root"], .8)
        self.assertGreater(held["pearson_pooled"], .9)
        self.assertGreater(held["calibration_slope"], .7)
        self.assertGreater(held["margins"]["0.0"]["gain_per_100_roots"], 0.)
        self.assertEqual(set(held["by_kind"]), set(KINDS))
        for cell in held["by_kind"].values():
            self.assertGreater(cell["top1_vs_choice4"], .5)
            self.assertIsNotNone(cell["halves_same_answer"])
        self.assertEqual(set(held["items"]), {"ordinary", "payable_added"})
        self.assertIn(result["margin"], V.MARGINS)
        self.assertTrue(held["overrides"])

    def test_an_untrained_head_never_overrides(self):
        rows = synthetic(roots=60)
        prepared = V.Prepared(rows, tau=.02)
        head = V.SearchHead(prepared.rows.shape[1], "mlp")
        with torch.no_grad():
            gaps = prepared.gaps(head(prepared.rows)).numpy()
        np.testing.assert_allclose(gaps, 0., atol=1e-6)   # zero-initialised: the bundle plays
        out = V.report(rows, gaps, prepared.weight.numpy(), rows.held_out | ~rows.held_out, margin=0.)
        self.assertEqual(out["margins"]["0.0"]["overrides"], 0)

    def test_directions_and_export_round_trip(self):
        self.assertEqual(V.direction(0, 2, False), "pass->act")
        self.assertEqual(V.direction(2, 0, False), "act->pass")
        self.assertEqual(V.direction(1, 2, True), "tap->cast (payable-added)")
        self.assertEqual(V.direction(4, 2, False), "land<->cast")
        import onnxruntime
        width = 2 * 16 + V.TAIL
        head = V.SearchHead(width, "mlp", hidden=8)
        with torch.no_grad():
            for parameter in head.parameters():
                parameter.normal_(std=.1)
        rows = torch.randn(5, width)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "advantage.onnx"
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                V.export(head, width, path)
            session = onnxruntime.InferenceSession(str(path), providers=["CPUExecutionProvider"])
            (score,) = session.run(["score"], {"rows": rows.numpy()})
        np.testing.assert_allclose(score, head(rows).detach().numpy(), atol=1e-5)

    def test_fit_command_writes_the_head_and_its_contract(self):
        rows = synthetic(roots=120)
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()):
            path = Path(temporary) / "rows.npz"
            rows.save(path)
            out = Path(temporary) / "head"
            meta = V.main(["fit", "--rows", str(path), "--out", str(out), "--head", "linear", "--seed", "2",
                           "--epochs", "3", "--threads", "1"])
            saved = json.loads((out / "advantage.json").read_text())
            self.assertTrue((out / "advantage.onnx").exists())
            with self.assertRaises(FileExistsError):
                V.main(["fit", "--rows", str(path), "--out", str(out), "--seed", "2"])
        self.assertEqual((saved["contract"], saved["row_tail"], saved["width"]), (V.CONTRACT, V.TAIL, 2 * 16 + V.TAIL))
        self.assertEqual(meta["margin"], saved["margin"])


if __name__ == "__main__":
    unittest.main()

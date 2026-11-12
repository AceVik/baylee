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

    def test_held_out_halves_split_strided_games(self):
        rows = synthetic(roots=240)
        rows.held_out = rows.roots["game"] % 4 == 0   # every held-out id is even
        select, final = V.held_out_halves(rows)
        self.assertTrue(select.any() and final.any())
        self.assertFalse((select & final).any())
        np.testing.assert_array_equal(select | final, rows.held_out)
        self.assertFalse(set(rows.roots["game"][select]) & set(rows.roots["game"][final]))   # whole games

    def test_an_untrained_head_never_overrides(self):
        rows = synthetic(roots=60)
        prepared = V.Prepared(rows, tau=.02)
        head = V.SearchHead(prepared.rows.shape[1], "mlp")
        with torch.no_grad():
            gaps = prepared.gaps(head(prepared.rows)).numpy()
        np.testing.assert_allclose(gaps, 0., atol=1e-6)   # zero-initialised: the bundle plays
        out = V.report(rows, gaps, prepared.weight.numpy(), rows.held_out | ~rows.held_out, margin=0.)
        self.assertEqual(out["margins"]["0.0"]["overrides"], 0)

    def test_sidecar_reads_fail_closed_and_maps_to_rows(self):
        index = np.zeros(2, dtype=V.SIDECAR_INDEX)
        index["game"], index["cursor"], index["root_cursor"], index["seat"] = [7, 7], [101, 205], [100, 204], 1
        index["root_kind"], index["own_idx"], index["n_cand"], index["offset"] = [0, 2], [0, 1], [3, 2], [0, 3]
        index["choice4_idx"], index["half_a_idx"], index["half_b_idx"] = [2, V.MISSING], [2, 0], [1, V.MISSING]
        items = np.zeros(5, dtype=V.SIDECAR_ITEM)
        items["A"] = [0., .1, .3, -.2, 0.]
        items["SE"] = [0., .05, .04, np.nan, 0.]
        items["searched"] = [1, 1, 1, 0, 1]
        items["payable_added"] = [0, 0, 1, 0, 0]
        items["cls"] = [0, 1, 2, 2, 1]
        self.assertEqual((V.SIDECAR_INDEX.itemsize, V.SIDECAR_ITEM.itemsize), (48, 12))
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            index.tofile(folder / "index.bin")
            items.tofile(folder / "cands.bin")
            import hashlib
            digest = lambda name: hashlib.sha256((folder / name).read_bytes()).hexdigest()
            header = dict(contract=V.SIDECAR_CONTRACT,
                          index=dict(rows=2, sha256=digest("index.bin"), root_kinds=["tap", "land", "pass"]),
                          cands=dict(rows=5, sha256=digest("cands.bin"),
                                     coarse_classes=["pass", "tap", "cast", "activate", "land", "other"]))
            (folder / "header.json").write_text(json.dumps(header))
            sidecar = V.SearchAdvantage.load(folder)
            for broken in (dict(index=dict(header["index"], rows=3)), dict(cands=dict(header["cands"], sha256="0" * 64)),
                           dict(cands=dict(header["cands"], coarse_classes=["tap", "pass"]))):
                (folder / "header.json").write_text(json.dumps(dict(header, **broken)))
                with self.subTest(header=broken), self.assertRaises(ValueError):
                    V.SearchAdvantage.load(folder)
        self.assertTrue(np.array_equal(V.held_out_of("train", np.array([3, 4, 8])), [False, False, False]))
        self.assertTrue(np.array_equal(V.held_out_of("train", np.array([3, 4, 8]), 4), [False, True, True]))
        self.assertTrue(V.held_out_of("val", np.array([3])).all())
        np.testing.assert_array_equal(sidecar.find(np.array([7, 7, 7]), np.array([1, 1, 0]), np.array([205, 101, 101])),
                                      [1, 0, -1])
        roots, labels = sidecar.labels(np.array([1, 0]))
        self.assertEqual((roots["cursor"].tolist(), roots["choice4"].tolist(), roots["half_b"].tolist()),
                         ([204, 100], [V.NONE, 2], [V.NONE, 1]))
        self.assertEqual(roots["offset"].tolist(), [0, 2])
        self.assertEqual((labels["searched"].tolist(), float(labels["A"][0])), ([False, True, True, True, True], 0.))
        rows = V.AdvantageRows(roots, np.zeros((2, 4), np.float16), np.zeros((5, 4), np.float16), labels,
                               kinds=sidecar.kinds)
        self.assertEqual(rows.kinds, ("tap", "land", "pass"))
        for broken in (dict(own=(1, 1)), dict(n=(3, 9)), dict(choice=(5, 0))):
            bad = index.copy()
            if "own" in broken:
                bad["own_idx"] = broken["own"]      # root 0's own item would be the unsearched-baseline .1
            if "n" in broken:
                bad["n_cand"] = broken["n"]
            if "choice" in broken:
                bad["choice4_idx"] = broken["choice"]
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                V.SearchAdvantage(bad, items, {})

    def test_twin_block_gives_the_root_cursor_and_refuses_a_malformed_one(self):
        from types import SimpleNamespace
        import dataset4 as D4
        decode = lambda extension: D4.TrajectoryDataset4._with_distillation(SimpleNamespace(), {}, 0, 0, None,
                                                                             extension=extension)
        block = b"BSA1" + bytes([1, 0, 0, 0]) + (4242).to_bytes(4, "little") + bytes(4)
        self.assertEqual(int(decode(block)["twin_root_cursor"]), 4242)
        self.assertNotIn("twin_root_cursor", decode(b""))
        for broken in (b"BSA1" + bytes([2, 0, 0, 0]) + bytes(8), block[:-1], block[:5] + b"\x01" + block[6:],
                       block[:12] + b"\x01\x00\x00\x00"):
            with self.subTest(broken=broken), self.assertRaises(ValueError):
                decode(broken)

    def test_root_pass_reads_what_the_exported_graph_exports(self):
        import model4 as M
        import model5 as V5
        from export_onnx4 import EmbeddingGraph
        from test_training_execution4 import batch_fixture
        torch.manual_seed(4)
        net = V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64, structure_width=16,
                                       relational_width=16, relational_revision=2, candidate_context_revision=2)).eval()
        net.decision_chunk, net.prune_unused_policy_heads = 2, False
        batch = batch_fixture(b=2, u=1)
        batch["is_decision"][:] = True
        batch["step_valid"][:] = True
        batch["cand_mask"][:] = True
        frames = [{name: value[i, 0].numpy() for name, value in batch.items() if value.dim() >= 2}
                  for i in range(2)]
        hidden = torch.randn(2, M.RECURRENT_SLOTS, 32)
        state, cands = V.RootPass(net, 2, 2)(frames, hidden)
        with torch.no_grad():
            inputs = [batch[name][:, 0] for name in M.INPUT_NAMES[:-2]] + [hidden, torch.full((2,), 2)]
            *_, want_state, want_cand, _ = EmbeddingGraph(net)(*inputs)
        torch.testing.assert_close(state, want_state.float(), atol=1e-4, rtol=1e-4)
        for i, cand in enumerate(cands):
            torch.testing.assert_close(cand, want_cand[i, :cand.shape[0]].float(), atol=1e-4, rtol=1e-4)

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

    def test_smoke_head_prefers_payable_added_items_and_says_so(self):
        import onnxruntime
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()):
            net = Path(temporary) / "net"
            (net / "fp16").mkdir(parents=True)
            (net / "fp16" / "model.onnx.json").write_text(json.dumps(dict(d_model=4, source_checkpoint_sha256="ab")))
            out = Path(temporary) / "smoke"
            meta = V.main(["smoke", "--model", str(net), "--out", str(out)])
            session = onnxruntime.InferenceSession(str(out / "advantage.onnx"), providers=["CPUExecutionProvider"])
            rows = np.random.default_rng(0).normal(size=(3, 8 + V.TAIL)).astype(np.float32)
            rows[:, 8:] = 0.
            rows[1, 8 + 6] = 1.                     # the second item is payable-added
            (score,) = session.run(["score"], {"rows": rows})
            saved = json.loads((out / "advantage.json").read_text())
        np.testing.assert_allclose(score, [0., 1., 0.], atol=1e-6)
        self.assertTrue(saved["smoke"] and meta["smoke"])
        self.assertEqual((saved["margin"], saved["source_checkpoint_sha256"], saved["width"]), (0., "ab", 8 + V.TAIL))

    def test_ranking_loss_is_the_kl_to_the_tempered_advantages(self):
        rows = synthetic(roots=3)
        rows.labels["searched"][rows.roots["offset"][2]:] = False   # root 2 searched one item: no list
        rows.labels["searched"][rows.roots["offset"][2] + rows.roots["own"][2]] = True
        prepared = V.Prepared(rows, tau=.02)
        scores, T = torch.randn(len(rows.cand)) * .05, .02
        chosen = torch.tensor([True, False, True])
        lo, n, own = (int(rows.roots[k][0]) for k in ("offset", "n", "own"))
        a = torch.tensor(rows.labels["A"][lo:lo + n], dtype=torch.float32)
        a[own] = 0.
        p, log_q = torch.softmax(a / T, 0), torch.log_softmax(scores[lo:lo + n] / T, 0)
        expected = float((p * (p.log() - log_q)).sum())   # one list: root 2 has a single searched item
        self.assertAlmostEqual(float(V.ranking_loss(prepared, scores, chosen, T)), expected, places=4)
        exact = prepared.rank_target + 1.   # q_i - q_own = A_i (a shift per root changes nothing)
        self.assertAlmostEqual(float(V.ranking_loss(prepared, exact, torch.ones(3, dtype=torch.bool), T)), 0., places=5)
        scores.requires_grad_(True)
        V.ranking_loss(prepared, scores, chosen, T).backward()
        self.assertTrue(torch.isfinite(scores.grad).all())
        self.assertEqual(float(scores.grad[rows.roots["offset"][1]:rows.roots["offset"][2]].abs().sum()), 0.)

    def test_ranking_fit_keeps_gaps_in_the_advantages_units(self):
        rows = synthetic()
        head, result = V.fit(rows, kind="linear", lr=3e-2, epochs=60, batch=64, patience=10, seed=1,
                             log=lambda *_: None, objective="ranking", temperature=.05)
        held = result["held_out"]
        self.assertEqual((result["objective"], result["temperature"]), ("ranking", .05))
        self.assertGreater(held["pearson_pooled"], .9)
        self.assertGreater(held["calibration_slope"], .6)
        self.assertLess(held["calibration_slope"], 1.5)
        self.assertGreater(held["agreement"]["student_vs_a_half"], .5)
        self.assertIsNotNone(held["agreement"]["halves_same_answer"])
        self.assertGreater(held["top1_vs_choice4"]["share"], .5)
        self.assertLess(result["history"][-1]["select"], result["zero_head_loss"]["select"])

    def test_sources_keep_their_games_apart_and_train_alone_or_equal(self):
        small, large = synthetic(roots=120, seed=4), synthetic(roots=360, seed=5)
        for part in (small, large):
            part.held_out = part.roots["game"] % 4 == 0
        both = V.AdvantageRows.concat([small, large])
        self.assertEqual(len(np.unique(both.game_key())), len(np.unique(small.roots["game"])) +
                         len(np.unique(large.roots["game"])))
        select, final = V.held_out_halves(both)
        alone_select, alone_final = V.held_out_halves(large)   # a source splits the same pooled or alone
        np.testing.assert_array_equal(final[both.source == 1], alone_final)
        np.testing.assert_array_equal(select[both.source == 1], alone_select)
        rng = np.random.default_rng(1)
        only = V.training_roots(both, rng, train_sources=[1])
        self.assertTrue((both.source[only] == 1).all() and not both.held_out[only].any())
        self.assertEqual(len(only), int((~large.held_out).sum()))
        equal = V.training_roots(both, rng, equal_sources=True)
        per = [int((both.source[equal] == s).sum()) for s in (0, 1)]
        self.assertEqual(per[0], int((~small.held_out).sum()))
        self.assertLess(abs(per[1] - per[0]), 6)   # whole games of three roots
        half = V.training_roots(both, rng, train_fraction=.5, equal_sources=True)
        self.assertLess(abs(int((both.source[half] == 1).sum()) - per[0] / 2), 6)
        with self.assertRaises(ValueError):
            V.training_roots(both, rng, train_sources=[2])
        _, result = V.fit(both, kind="linear", lr=3e-2, epochs=3, batch=64, seed=1, log=lambda *_: None,
                          train_sources=[0])
        self.assertEqual(set(result["held_out_by_source"]), {"0", "1"})
        self.assertEqual(result["train_roots_by_source"], {"0": int((~small.held_out).sum()), "1": 0})

    def test_fit_command_picks_the_temperature_on_the_selection_half(self):
        rows = synthetic(roots=120)
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()):
            first, second = Path(temporary) / "c1.npz", Path(temporary) / "c2.npz"
            rows.save(first, model_sha256="48a5")
            synthetic(roots=90, seed=7).save(second, model_sha256="48a5")
            out = Path(temporary) / "head"
            V.main(["fit", "--rows", str(first), "--rows", str(second), "--out", str(out), "--head", "linear",
                    "--seed", "2", "--epochs", "3", "--threads", "1", "--objective", "ranking",
                    "--temperatures", "0.01,0.05", "--equal-sources"])
            saved = json.loads((out / "advantage.json").read_text())
        self.assertEqual([t["temperature"] for t in saved["temperature_selection"]], [.01, .05])
        self.assertIn(saved["temperature"], (.01, .05))
        self.assertEqual((saved["objective"], saved["equal_sources"]), ("ranking", True))
        self.assertEqual([r["source"] for r in saved["rows"]], [0, 1])
        self.assertEqual(set(saved["held_out_by_source"]), {"0", "1"})

    def test_fit_command_writes_the_head_and_its_contract(self):
        rows = synthetic(roots=120)
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()):
            path, other = Path(temporary) / "rows.npz", Path(temporary) / "other.npz"
            rows.save(path, model_sha256="48a5")
            rows.save(other, model_sha256="ffff")
            out = Path(temporary) / "head"
            meta = V.main(["fit", "--rows", str(path), "--out", str(out), "--head", "linear", "--seed", "2",
                           "--epochs", "3", "--threads", "1"])
            saved = json.loads((out / "advantage.json").read_text())
            self.assertTrue((out / "advantage.onnx").exists())
            with self.assertRaises(FileExistsError):
                V.main(["fit", "--rows", str(path), "--out", str(out), "--seed", "2"])
            with self.assertRaises(ValueError):   # one head reads one net's embeddings
                V.main(["fit", "--rows", str(path), "--rows", str(other), "--out", str(out) + "2", "--seed", "2"])
        self.assertEqual((saved["contract"], saved["row_tail"], saved["width"]), (V.CONTRACT, V.TAIL, 2 * 16 + V.TAIL))
        self.assertEqual((meta["margin"], saved["source_checkpoint_sha256"]), (saved["margin"], "48a5"))
        self.assertEqual(saved["tail"][:6], [f"class:{name}" for name in ("pass", "tap", "cast", "activate", "land",
                                                                           "other")])


if __name__ == "__main__":
    unittest.main()

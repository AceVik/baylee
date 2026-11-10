"""Thinking revision 2: the deliberation block, its training terms and its export."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import onnxruntime as ort
import torch
from torch.nn import functional as F

import model4 as V4
import model5 as V5
from export_onnx4 import THINKING_NAMES, export_graph, export_v4_onnx, main, make_dummy_inputs
from model5.deliberation import Deliberation
import test_deferred4 as deferred_tests
import test_export_models as export_tests
from training4.device_loss import compute_unroll_device_loss
from training4.think_loss import ThinkLossConfig, thinking_loss


def config(revision=2, **overrides):
    return V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64, structure_width=16,
                      relational_width=16, relational_revision=2, thinking_revision=revision,
                      **({"deliberation_width": 32, "deliberation_heads": 4, "deliberation_candidates": 4}
                         if revision == 2 else {}), **overrides)


def stir(net, scale=0.3):
    """A block that has learnt something: its correction is no longer zero."""
    with torch.no_grad():
        net.deliberation.correction.weight.normal_(std=scale)
        net.relational.output.weight.normal_(std=.05)
    return net


class ThinkingTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(2718)

    def test_archived_configuration_and_construction_are_untouched(self):
        self.assertEqual(set(V5.THINKING_FIELDS) & set(config(1).to_dict()), set())
        self.assertEqual(V5.Config5(**config(1).to_dict()).to_dict(), config(1).to_dict())
        self.assertEqual(V5.Config5(**config().to_dict()).to_dict(), config().to_dict())
        torch.manual_seed(5); old = V5.PlayerNet5(config(1)); state = torch.get_rng_state()
        torch.manual_seed(5); new = V5.PlayerNet5(config())
        self.assertTrue(torch.equal(state, torch.get_rng_state()))
        added = set(new.state_dict()) - set(old.state_dict())
        self.assertTrue(added and all(key.startswith("deliberation.") for key in added))
        for bad in (dict(thinking_revision=3), dict(thinking_revision=2, relational_revision=1),
                    dict(thinking_revision=2, deliberation_steps=9), dict(thinking_revision=2, scratchpad_steps=9)):
            with self.assertRaises(ValueError):
                V5.Config5(**bad)

    def test_a_fresh_block_plays_exactly_as_its_source_at_every_depth(self):
        torch.manual_seed(5); old = V5.PlayerNet5(config(1)).eval()
        new = V5.PlayerNet5(config()).eval()
        with torch.no_grad():
            old.relational.output.weight.normal_(std=.05)
        new.load_state_dict(V5.warmstart_thinking(old.state_dict(), new), strict=True)
        inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=7, P=2))
        inputs[11][:, -2:] = False
        with torch.no_grad():
            # The trunk was trained at scratchpad depth 2: every depth of the
            # new net reads that scratchpad and adds a zero correction.
            inputs[-1].fill_(2); want = old(*inputs)
            for k in (1, 2, 4, 8):
                inputs[-1].fill_(k); got = new(*inputs)
                for a, b in zip(want, got):
                    self.assertTrue(torch.equal(a, b))
            inputs[-1].fill_(0)
            for a, b in zip(old(*inputs), new(*inputs)):
                self.assertTrue(torch.equal(a, b))
        for state, target in ((new.state_dict(), new), (old.state_dict(), old)):
            with self.assertRaises(ValueError):
                V5.warmstart_thinking(state, target)

    def block(self, feedback=True):
        block = Deliberation(32, 32, 4, steps=3, candidates=4, feedback=feedback).eval()
        with torch.no_grad():
            block.correction.weight.normal_(std=.5)
        return block

    def menu(self, b=3, c=6, e=5):
        hidden, logits = torch.randn(b, c, 32), torch.randn(b, c)
        mask = torch.ones(b, c, dtype=torch.bool); mask[0, 2:] = False; mask[1, -1] = False
        tokens = torch.randn(b, 9 + e, 32); pad = torch.zeros(b, 9 + e, dtype=torch.bool); pad[:, -1] = True
        return hidden, logits.masked_fill(~mask, -1e9), mask, tokens, pad

    def test_steps_read_the_proposal_and_only_the_best_answers_move(self):
        block = self.block(); hidden, logits, mask, tokens, pad = self.menu()
        block.collect = True
        with torch.no_grad():
            out = block(hidden, logits, mask, tokens, pad)
            trace = block.trace
            self.assertEqual(trace["logits"].shape, (4, 3, 6))
            self.assertTrue(torch.equal(trace["logits"][0], logits))
            self.assertTrue(torch.equal(trace["logits"][-1], out))
            self.assertTrue((out[~mask] == -1e9).all())
            # Row 2 offers six answers and the block holds four: the two
            # worst by the first logits keep their scores at every step.
            outside = torch.ones(6, dtype=torch.bool); outside[trace["index"][2]] = False
            self.assertEqual(int(outside.sum()), 2)
            self.assertTrue(torch.equal(out[2, outside], logits[2, outside]))
            self.assertFalse(torch.equal(out[2, ~outside], logits[2, ~outside]))
            # The same candidates with another first proposal are corrected
            # differently: the step reads the proposal, not only the answers.
            shifted = logits.clone(); shifted[2, trace["index"][2, 1]] += .25
            again = block(hidden, shifted, mask, tokens, pad)
            delta, other = (out - logits)[2, ~outside], (again - shifted)[2, ~outside]
            self.assertGreater((delta - other).abs().max().item(), 1e-4)
            blind = self.block(feedback=False); blind.load_state_dict(block.state_dict())
            a, b2 = blind(hidden, logits, mask, tokens, pad), blind(hidden, shifted, mask, tokens, pad)
            torch.testing.assert_close((a - logits)[2], (b2 - shifted)[2], atol=1e-6, rtol=0)

    def test_per_row_depth_matches_running_each_depth_alone_and_rows_never_mix(self):
        block = self.block(); hidden, logits, mask, tokens, pad = self.menu()
        with torch.no_grad():
            mixed = block(hidden, logits, mask, tokens, pad, depth=torch.tensor([0, 1, 3]))
            mixed_confidence = block.last_confidence.clone()
            self.assertTrue(torch.equal(mixed[0], logits[0]))
            self.assertEqual(float(mixed_confidence[0]), 1.0)
            for row, k in ((1, 1), (2, 3)):
                alone = block(hidden[row:row + 1], logits[row:row + 1], mask[row:row + 1],
                              tokens[row:row + 1], pad[row:row + 1], depth=torch.tensor([k]))
                torch.testing.assert_close(alone[0], mixed[row], atol=1e-5, rtol=1e-5)
                torch.testing.assert_close(block.last_confidence[0], mixed_confidence[row], atol=1e-5, rtol=1e-5)
            # Padding of the board or the menu changes nothing.
            wide = block(torch.cat((hidden, torch.randn(3, 2, 32)), 1),
                         torch.cat((logits, torch.full((3, 2), -1e9)), 1),
                         torch.cat((mask, torch.zeros(3, 2, dtype=torch.bool)), 1),
                         torch.cat((tokens, torch.randn(3, 4, 32)), 1),
                         torch.cat((pad, torch.ones(3, 4, dtype=torch.bool)), 1), depth=torch.tensor([0, 1, 3]))
            torch.testing.assert_close(wide[:, :6], mixed, atol=1e-5, rtol=1e-5)

    def deferred(self, net, batch, chunk):
        return deferred_tests.DeferredReadouts().outputs(net, batch, chunk, thinking_steps=2)

    def test_reference_and_deferred_execution_agree_and_the_trace_is_the_readout(self):
        batch = deferred_tests.DeferredReadouts().fixture()
        net = stir(V5.PlayerNet5(config(activation_checkpointing=True)))
        net.deliberation.steps_override = net.cfg.deliberation_steps
        other = copy.deepcopy(net); other.deliberation.collect = True
        left, lh, plans = self.deferred(net, batch, 0)
        right, rh, _ = self.deferred(other, batch, 3)
        torch.testing.assert_close(lh, rh, atol=0, rtol=0)
        for a, b in zip(left, right):
            for x, y in zip(a, b):
                torch.testing.assert_close(x, y, atol=3e-6, rtol=3e-5)
        frames = other.thinking_frames
        self.assertEqual(len(frames), len(plans))
        seen = 0
        for t, frame in enumerate(frames):
            if frame is None:
                continue
            seen += 1
            self.assertEqual(frame["logits"].shape[0], net.cfg.deliberation_steps + 1)
            self.assertTrue(torch.equal(frame["logits"][-1], right[t][1]))
        self.assertGreater(seen, 0)

    def test_every_term_reaches_the_block_and_only_the_block_when_frozen(self):
        batch = deferred_tests.DeferredReadouts().fixture()
        net = stir(V5.PlayerNet5(config()))
        net.deliberation.steps_override = net.cfg.deliberation_steps
        net.deliberation.collect = True
        for name, parameter in net.named_parameters():
            parameter.requires_grad_(name.startswith("deliberation."))
        # Lane 1 answers single choices; its target is an answer the block holds.
        batch["head"][1] = 0
        batch["single_idx"][1] = 0
        outputs, _, plans = self.deferred(net, batch, 3)
        held = next(frame for frame in net.thinking_frames if frame is not None)["index"][1, 1]
        batch["single_idx"][1] = held
        for term in ("deep_weight", "improve_weight", "confidence_weight", "consequence_weight"):
            with self.subTest(term=term):
                only = ThinkLossConfig(**{key: 0. for key in ("deep_weight", "improve_weight",
                                          "confidence_weight", "consequence_weight")} | {term: 1.})
                loss, stats = thinking_loss(net.thinking_frames, batch, plans, only)
                self.assertTrue(torch.isfinite(loss))
                net.zero_grad()
                loss.backward(retain_graph=True)
                touched = {name for name, p in net.named_parameters()
                           if p.grad is not None and p.grad.abs().max() > 0}
                self.assertTrue(touched and all(name.startswith("deliberation.") for name in touched), term)
                head = {"confidence_weight": "deliberation.confidence.weight",
                        "consequence_weight": "deliberation.consequence.weight"}.get(term, "deliberation.correction.weight")
                self.assertIn(head, touched)
                self.assertEqual(stats.numel(), net.cfg.deliberation_steps + 2)
                self.assertGreater(float(stats[-1]), 0)
        with self.assertRaises(ValueError):
            ThinkLossConfig(deep_weight=-1.)
        with self.assertRaises(ValueError):
            thinking_loss([None] * len(plans), batch, plans, ThinkLossConfig())

    def test_training_makes_later_steps_correct_earlier_ones(self):
        """A frozen first proposal that is wrong wherever a feature says so: the
        block must learn the correction, later steps at least as well as the
        first, and its confidence must tell right answers from wrong ones."""
        torch.manual_seed(3)
        block = Deliberation(16, 32, 4, steps=3, candidates=4)
        block.collect = True
        optimizer = torch.optim.Adam(block.parameters(), lr=3e-3)

        def sample(n):
            hidden = torch.randn(n, 4, 16)
            target = hidden[:, :, 0].argmax(-1)
            # The first proposal prefers the target, except where the board's
            # first summary slot says the position is a trap: there it prefers
            # the runner-up, and only reading both tells which case holds.
            tokens = torch.randn(n, 10, 16)
            trap = tokens[:, 0, 0] > 0.3
            order = hidden[:, :, 0].argsort(-1, descending=True)
            first_choice = torch.where(trap, order[:, 1], order[:, 0])
            logits = F.one_hot(first_choice, 4).float() * 2.0
            return hidden, logits, tokens, target

        mask = torch.ones(256, 4, dtype=torch.bool); pad = torch.zeros(256, 10, dtype=torch.bool)
        for _ in range(500):
            hidden, logits, tokens, target = sample(256)
            block(hidden, logits, mask, tokens, pad)
            steps = block.trace["logits"]
            losses = torch.stack([F.cross_entropy(step, target, reduction="none") for step in steps])
            right = (steps.argmax(-1) == target).float()
            loss = (losses[1:].mean() + .5 * F.relu(losses[1:] - losses[:-1].detach()).mean()
                    + .25 * F.binary_cross_entropy_with_logits(block.trace["confidence"], right[1:]))
            optimizer.zero_grad(); loss.backward(); optimizer.step()
        block.eval()
        with torch.no_grad():
            hidden, logits, tokens, target = sample(256)
            block(hidden, logits, mask, tokens, pad)
            accuracy = (block.trace["logits"].argmax(-1) == target).float().mean(-1)
            right = block.trace["logits"][-1].argmax(-1) == target
            confidence = block.last_confidence
        self.assertLess(float(accuracy[0]), 0.75)
        self.assertGreater(float(accuracy[-1]), float(accuracy[0]) + 0.1)
        self.assertGreaterEqual(float(accuracy[-1]), float(accuracy[1]) - 0.02)
        if right.any() and (~right).any():
            self.assertGreater(float(confidence[right].mean()), float(confidence[~right].mean()))

    def test_cpu_onnx_parity_at_every_depth_with_confidence(self):
        net = stir(V5.PlayerNet5(config())).eval(); net.attach_structure(export_tests.FOREST)
        with torch.no_grad():
            net.deliberation.confidence.weight.normal_(std=.5)
        net.rules = net.rules.freeze()
        options = ort.SessionOptions(); options.intra_op_num_threads = 1
        names = list(V4.OUTPUT_NAMES) + list(THINKING_NAMES)
        with tempfile.TemporaryDirectory() as root:
            answers = {}
            for depth in (0, 1, 2, 4, 8):
                path = Path(root) / f"model.k{depth}.onnx"
                export_v4_onnx(net, path, thinking_steps=depth, fixed_depth=True)
                session = ort.InferenceSession(str(path), sess_options=options, providers=["CPUExecutionProvider"])
                self.assertEqual([o.name for o in session.get_outputs()], names)
                for count, entities in ((1, 1), (3, 4), (7, 17), (40, 5)):
                    inputs = list(make_dummy_inputs(32, E=entities, M_edges=2, C=count, P=2, K=depth))
                    if count > 2:
                        inputs[11][:, -1] = False
                    with torch.no_grad():
                        expected = export_graph(net, False)(*inputs)
                    actual = session.run(names, {n: t.numpy() for n, t in zip(V4.INPUT_NAMES, inputs)})
                    for index, (want, got) in enumerate(zip(expected, actual)):
                        if index == 1:
                            got = got[:, :want.shape[1]]
                        np.testing.assert_allclose(got, want.numpy(), rtol=2e-4, atol=2e-5)
                    if (count, entities) == (7, 17):
                        answers[depth] = actual[1]
            # Depth is a real lever in the graphs: the steps change the logits.
            self.assertGreater(np.abs(answers[4] - answers[1]).max(), 1e-3)
            self.assertGreater(np.abs(answers[1] - answers[0]).max(), 1e-3)

    def test_cli_export_declares_the_thinking_outputs(self):
        net = stir(V5.PlayerNet5(config())).eval(); net.attach_structure(export_tests.FOREST)
        saved = export_tests.ExportModels().checkpoint(V5, net.cfg, net)
        saved["encoder_version"] = 8
        with tempfile.TemporaryDirectory() as root:
            path = Path(root)
            torch.save(saved, path / "net.pt")
            with patch("sys.argv", ["export_onnx4.py", "--model", root, "--check", "0", "--thinking-steps", "4"]):
                main()
            metadata = json.loads((path / "model.onnx.json").read_text())
            self.assertEqual(metadata["architecture"], V5.ARCHITECTURES[2])
            self.assertEqual(metadata["config"]["thinking_revision"], 2)
            self.assertEqual(metadata["thinking_outputs"], {
                "contract": "deliberation_v1", "names": ["think_confidence"], "depth": "deliberation_steps",
                "trained_steps": 4, "scratchpad_steps": 2})
            self.assertEqual(metadata["thinking_steps"], 4)
            self.assertLess(metadata["parity_max_abs_diff"], 1e-3)
            # The FP16 graphs keep the block: same outputs, the same top answer.
            names = list(V4.OUTPUT_NAMES) + list(THINKING_NAMES)
            inputs = make_dummy_inputs(32, E=6, M_edges=3, C=7, P=2, K=4)
            feed = {n: t.numpy() for n, t in zip(V4.INPUT_NAMES, inputs)}
            full, half = (ort.InferenceSession(str(path / part / "model.k4.onnx"), providers=["CPUExecutionProvider"])
                          .run(names, feed) for part in ("", "fp16"))
            np.testing.assert_allclose(half[1], full[1], atol=5e-2)
            np.testing.assert_allclose(half[5], full[5], atol=2e-2)
            self.assertEqual(half[1].argmax(), full[1].argmax())

    def test_the_driver_trains_only_the_block_from_a_revision_one_checkpoint(self):
        """Stage A's shape through the real driver: a revision 1 net, the block
        attached by warm start, everything else frozen, the per-step terms on."""
        import contextlib
        import io
        from types import SimpleNamespace
        from model4.structure import structure_digest
        from test_training_execution4 import batch_fixture
        from training4 import driver

        batch = batch_fixture(b=4, u=4); batch["is_new_traj"] = torch.ones(4, dtype=torch.bool)
        batch["head"][1] = 0; batch["single_idx"][1] = 0
        forest = V4.PlayerNet4(V4.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).rules.data

        class Dataset:
            def __init__(self, *args, split, **kw):
                self.structure = SimpleNamespace(data=forest, digest=structure_digest(forest)); self.fingerprint = split

            def __len__(self):
                return 1

        class Sampler:
            VERSION = driver.D4.StatefulBatchSampler4.VERSION

            def __init__(self, *args, **kw):
                self.epoch = 0; self.start_batch = 0

        metrics = dict(val_pol_loss=1., val_mse=1., val_aux_mse=1., val_top1_acc=0.)

        def run(argv):
            with patch("sys.argv", argv), patch.object(driver.D4, "TrajectoryDataset4", Dataset), \
                    patch.object(driver.D4, "StatefulBatchSampler4", Sampler), \
                    patch.object(driver, "DataLoader", return_value=[]), \
                    patch.object(driver, "prefetch_batches", return_value=[(copy.deepcopy(batch), None)]), \
                    patch.object(driver, "evaluate_device", return_value=metrics), \
                    patch.object(driver.torch, "set_num_threads"), contextlib.redirect_stdout(io.StringIO()):
                driver.main("v5")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            common = ["--data", "fake", "--d-model", "32", "--layers", "1", "--heads", "4", "--d-ff", "64",
                      "--batch-size", "4", "--unroll", "4", "--board-chunk", "2", "--decision-chunk", "2",
                      "--max-steps", "1", "--epochs", "1", "--loss-version", "5",
                      "--loss-normalization", "unroll_per_head", "--outcome-weighting-off"]
            run(["train5", "--out", str(root / "source"), *common])
            thinking = ["--thinking-revision", "2", "--deliberation-width", "32", "--deliberation-candidates", "4",
                        "--deliberation-steps", "3", "--thinking-warmstart", "--train-thinking-only",
                        "--think-supervision", "--init", str(root / "source"), "--lr", "1e-2"]
            run(["train5", "--out", str(root / "think"), *common, *thinking])
            before = torch.load(root / "source" / "net.pt", map_location="cpu", weights_only=False)
            after = torch.load(root / "think" / "last.pt", map_location="cpu", weights_only=False)
            self.assertEqual(set(V5.THINKING_FIELDS) & set(before["config"]), set())
            self.assertEqual(after["config"]["thinking_revision"], 2)
            self.assertEqual(after["config"]["deliberation_steps"], 3)
            self.assertEqual(after["config"]["scratchpad_steps"], 2)
            self.assertEqual(after["architecture"], before["architecture"])
            self.assertEqual(after["train_parts"], "thinking")
            self.assertEqual(after["thinking"]["contract"], "deliberation_v1")
            self.assertEqual(after["thinking"]["loss"]["version"], 1)
            self.assertEqual(after["thinking_warmstart"]["kind"], "weights_only")
            self.assertEqual(after["depth_schedule"]["report_k"], (2,))
            moved = 0
            for name, value in after["state_dict"].items():
                if name.startswith("deliberation."):
                    moved += int(name not in before["state_dict"])
                else:
                    torch.testing.assert_close(value, before["state_dict"][name], rtol=0, atol=0)
            self.assertGreater(moved, 0)
            self.assertGreater(float(after["state_dict"]["deliberation.correction.weight"].abs().max()), 0.)
            # The trained net exports and loads again as what it declares.
            from export_onnx4 import load_export_checkpoint
            _, loaded, _ = load_export_checkpoint(root / "think" / "net.pt")
            self.assertIsNotNone(loaded.deliberation)
            for flags in (["--think-supervision"], ["--thinking-warmstart", "--init", str(root / "source")],
                          ["--thinking-revision", "2", "--think-supervision", "--decision-chunk", "1"],
                          ["--thinking-revision", "2", "--train-thinking-only"]):
                with self.subTest(flags=flags), patch("sys.argv", ["train5", "--out", str(root / "bad"), *common, *flags]), \
                        contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    from training4.cli import parse_args
                    parse_args("v5")


class Replay:
    """A held-out store of one seat's game, as the probes read one."""

    def __init__(self, frames=6, candidates=5):
        inputs = make_dummy_inputs(32, E=4, M_edges=3, C=candidates, P=2)
        segment = {name: tensor.numpy().repeat(frames, axis=0)
                   for name, tensor in zip(V4.INPUT_NAMES[:-2], inputs[:-2])}
        segment["cand_feat"] = np.random.default_rng(3).normal(size=segment["cand_feat"].shape).astype(np.float32)
        segment.update(step_valid=np.ones(frames, dtype=bool), is_decision=np.arange(frames) % 2 == 1,
                       head=np.zeros(frames, dtype=np.int64), loss_weight=np.ones(frames, dtype=np.float32),
                       single_idx=np.arange(frames) % candidates)
        self.trajectories, self.segment = [[0]], segment

    def __getitem__(self, index):
        return self.segment


class ToolTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(31)

    def source(self):
        net = V5.PlayerNet5(config(1)).eval(); net.attach_structure(export_tests.FOREST)
        with torch.no_grad():
            net.relational.output.weight.normal_(std=.05)
        return net, export_tests.ExportModels().checkpoint(V5, net.cfg, net)

    def test_attaching_the_block_keeps_the_source_and_marks_a_plumbing_model(self):
        from attach_thinking5 import attach
        net, saved = self.source()
        attached = attach(saved, steps=3, width=32, candidates=4)
        self.assertEqual(attached["config"]["thinking_revision"], 2)
        self.assertEqual(attached["config"]["scratchpad_steps"], net.cfg.default_thinking_steps)
        self.assertEqual(attached["architecture"], saved["architecture"])
        self.assertNotIn("use", attached["thinking_warmstart"])
        think = V5.PlayerNet5(V5.Config5(**attached["config"])).eval(); think.attach_structure(saved["structure"])
        think.load_state_dict(attached["state_dict"], strict=True)
        inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=7, P=2))
        with torch.no_grad():
            for a, b in zip(net(*inputs), think(*inputs)):
                self.assertTrue(torch.equal(a, b))
        stirred = attach(saved, steps=3, width=32, candidates=4, random_correction=.5, seed=7)
        self.assertEqual(stirred["thinking_warmstart"]["use"], "plumbing_measurement_only")
        self.assertGreater(float(stirred["state_dict"]["deliberation.correction.weight"].abs().max()), 0.)
        with self.assertRaises(ValueError):
            attach(attached, steps=3, width=32, candidates=4)

    def test_the_probes_read_depths_and_steps_of_a_replayed_game(self):
        from thinking_probe5 import probe, probe_steps
        net, _ = self.source()
        net.rules = net.rules.freeze()
        with torch.inference_mode():
            depths = probe(net, Replay(), [0, 1, 2, 4], 2, 100)
        self.assertEqual(depths["decisions"], 3)
        self.assertEqual(depths["per_depth"]["2"]["top_answer_differs"], 0.)
        self.assertEqual(depths["per_depth"]["2"]["scratchpad_relative_move"], 0.)
        self.assertGreater(depths["per_depth"]["0"]["scratchpad_relative_move"], 0.)
        think = stir(V5.PlayerNet5(config())).eval(); think.attach_structure(export_tests.FOREST)
        think.rules = think.rules.freeze()
        with torch.inference_mode():
            steps = probe_steps(think, Replay(), 100)
        self.assertEqual((steps["decisions"], steps["steps"]), (3, 4))
        self.assertEqual(len(steps["right_by_step"]), 5)
        self.assertEqual(steps["first_mistakes_corrected_by_step"][0], 0.)
        self.assertEqual(steps["right_answers_broken_by_step"][0], 0.)
        self.assertEqual(set(steps["thresholds"]), {"0.5", "0.6", "0.7", "0.8", "0.9"})
        self.assertTrue(0. <= steps["final_confidence_ece"] <= 1.)


if __name__ == "__main__":
    unittest.main()

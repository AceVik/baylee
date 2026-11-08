"""The pure parts of `vram_by_part4`: trace replay, peak attribution, frame parts."""
import tempfile
import textwrap
import unittest
from pathlib import Path

import vram_by_part4 as V


def alloc(addr, size, meta="", frames=()):
    return {"action": "alloc", "addr": addr, "size": size, "user_metadata": meta, "frames": list(frames)}


def free(addr, size, meta=""):
    return [{"action": "free_requested", "addr": addr, "size": size, "user_metadata": meta},
            {"action": "free_completed", "addr": addr, "size": size, "user_metadata": meta}]


def note(addr, text):
    return {"action": "annotate", "addr": addr, "size": 0, "user_metadata": text}


class ReplayTests(unittest.TestCase):
    def test_peak_live_set_and_late_annotations(self):
        trace = [
            alloc(0x100, 400, "setup"),                      # 0 parameter
            alloc(0x200, 100, "s0:encode_boards"),           # 1 activation, freed before the peak
            *free(0x200, 100, "s0:encode_boards"),           # 2, 3
            alloc(0x300, 300, "s0:encode_boards"),           # 4 retained activation
            alloc(0x400, 200, "s0:backward"),                # 5 gradient, peak here: 900
            *free(0x300, 300, "s0:backward"),                # 6, 7
            note(0x100, "param:trunk_layers"),               # 8
            note(0x400, "grad:trunk_layers"),                # 9 annotated after the peak
        ]
        final = [{"address": 0x100, "size": 400, "state": "active_allocated"},
                 {"address": 0x400, "size": 200, "state": "active_allocated"}]
        allocated, _, _, start = V.replay(trace, final)
        self.assertEqual(start, 0)
        self.assertEqual(allocated[5], 900)
        peak = V.argmax_in(allocated, range(len(trace)))
        self.assertEqual(peak, 5)
        _, captured, _, _ = V.replay(trace, final, capture={peak})
        result = V.breakdown(captured[peak], None)
        self.assertEqual(result["total"], 900)
        self.assertEqual(result["by_kind"], {"parameters": 400, "activations (forward)": 300, "gradients": 200})

    def test_window_opened_after_allocations(self):
        # 0x10 was allocated before the ring buffer's window and is freed inside
        # it; 0x20 lives through the whole window; 0x30 was only awaiting its
        # free when the window opened and no longer counts as allocated.
        trace = [
            {"action": "free_completed", "addr": 0x30, "size": 64, "user_metadata": ""},
            alloc(0x40, 50, "s3:loss"),
            *free(0x10, 100, "s3:loss"),
        ]
        final = [{"address": 0x20, "size": 1000, "state": "active_allocated"},
                 {"address": 0x40, "size": 50, "state": "active_allocated"}]
        allocated, captured, _, start = V.replay(trace, final, capture={-1})
        self.assertEqual(start, 1100)
        self.assertEqual(allocated, [1100, 1150, 1050, 1050])
        self.assertEqual(sorted(b.addr for b in captured[-1]), [0x10, 0x20])
        self.assertTrue(all(b.index == -1 for b in captured[-1]))

    def test_reserved_series_walks_segments_backward(self):
        trace = [{"action": "segment_map", "addr": 0, "size": 2048},
                 alloc(0x1, 1024),
                 {"action": "segment_unmap", "addr": 0, "size": 1024}]
        _, _, reserved, _ = V.replay(trace, [{"address": 0x1, "size": 1024, "state": "active_allocated"}],
                                     reserved_end=4096)
        self.assertEqual(reserved, [5120, 5120, 4096])

    def test_step_windows_carry_untagged_events(self):
        trace = [alloc(1, 1, ""), alloc(2, 1, "s0:rules_encode"), alloc(3, 1, ""),
                 alloc(4, 1, "s1:between"), alloc(5, 1, "s0:backward")]
        self.assertEqual(V.step_windows(trace), {0: [1, 2, 4], 1: [3]})
        self.assertEqual(V.step_of("s12:backward"), 12)
        self.assertIsNone(V.step_of("max:forward"))
        self.assertEqual(V.phase_of("s12:backward"), "backward")


SOURCE = '''\
import torch


class PreNormTransformerLayer:
    def forward(self, x):
        y = x + 1
        return y


class PlayerNet4:
    def encode_board(self, x):
        e_tok = self.entity_proj(x)
        for index, layer in enumerate(self.trunk_layers):
            x = checkpoint_activation(
                layer, x,
            )
        return self.trunk_norm(x)

    def rule_features(self, refs):
        return self.rules_proj(self.rules(refs))


def deferred_readouts(net, batch):
    c_feat = torch.cat([batch["cand_feat"].to(device) for t in times])
    s_pred = torch.sigmoid(net.scalar_head(g_scal))
    value_all, _ = net.seat_values(s, g, f)
    return c_feat
'''


class FrameTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        (root / "model4").mkdir()
        (root / "model4" / "player.py").write_text(SOURCE)
        self.file = str(root / "model4" / "player.py")
        self.classifier = V.FrameClassifier(root)

    def tearDown(self):
        self.tmp.cleanup()

    def frame(self, line, name="f", filename=None):
        return {"filename": filename or self.file, "line": line, "name": name}

    def test_parts_from_qualnames_and_statements(self):
        c = self.classifier
        torch_frame = {"filename": "/venv/torch/nn/modules/linear.py", "line": 10, "name": "forward"}
        self.assertEqual(c.part([torch_frame, self.frame(6)]), "trunk layers")
        self.assertEqual(c.part([self.frame(12)]), "embeddings + edge messages")
        self.assertEqual(c.part([self.frame(14)]), "trunk layers")
        self.assertEqual(c.part([self.frame(17)]), "trunk layers")
        self.assertEqual(c.part([self.frame(20), self.frame(12)]), "rule lookup")
        self.assertEqual(c.part([self.frame(24)]), "inputs")
        self.assertEqual(c.part([self.frame(25)]), "scalar head")
        self.assertEqual(c.part([self.frame(26)]), "value head")
        self.assertIsNone(c.part([torch_frame]))
        self.assertEqual(c.site([torch_frame, self.frame(25, "deferred_readouts")]),
                         "model4/player.py:25 deferred_readouts")

    def test_kinds(self):
        c = self.classifier
        compiled = {"filename": "/tmp/torchinductor_ace/ab/cabc.py", "line": 3, "name": "call"}
        recompute = {"filename": "/venv/torch/utils/checkpoint.py", "line": 9, "name": "recompute_fn"}
        cases = [
            (V.Block(1, 8, [self.frame(6)], "s2:encode_boards", 5), ("activations (forward)", "trunk layers")),
            (V.Block(1, 8, [self.frame(6), recompute], "s2:backward", 5), ("backward: checkpoint recompute", "trunk layers")),
            (V.Block(1, 8, [compiled, recompute], "s2:backward", 5), ("backward: checkpoint recompute", "compiled region")),
            (V.Block(1, 8, [compiled], "s2:backward", 5), ("backward: compiled region", "trunk layers / thinking (compiled)")),
            (V.Block(1, 8, [], "s2:backward", 5), ("backward: autograd", "no Python frame")),
            (V.Block(1, 8, [], "s2:backward", 5, ["grad:relational"]), ("gradients", "relational")),
            (V.Block(1, 8, [], "s2:optimizer", 5, ["adamw:rules:exp_avg"]), ("AdamW state", "rules")),
            (V.Block(1, 8, [self.frame(24)], "s2:readouts", 5), ("inputs", "inputs")),
            (V.Block(1, 8, [], "", -1), ("before the trace window", "?")),
        ]
        for block, expected in cases:
            self.assertEqual(V.kind_and_part(block, c), expected)


class ParseTests(unittest.TestCase):
    def test_trainer_arguments_follow_the_separator(self):
        opts = V.parse(["train", "--trainer-dir", "/src", "--out", "/o", "--skip-batches", "3723", "--keep-snapshot",
                        "--", "--data", "/d", "--max-steps", "27", "--compile-regions"])
        self.assertEqual((opts.mode, opts.skip_batches, opts.keep_snapshot), ("train", 3723, True))
        self.assertEqual(opts.trainer_argv, ["--data", "/d", "--max-steps", "27", "--compile-regions"])


class CallTests(unittest.TestCase):
    def test_calls_follow_eval4_grouping(self):
        tokens = [100, 700, 300, 50, 200, 520, 512, 400]
        self.assertEqual(V.mean_call(tokens, 3), [2, 7, 6])
        self.assertEqual(V.budget_call(tokens, 2, 1024), [7, 6])
        self.assertEqual(V.budget_call(tokens, 20, 1024), [3])
        drawn = V.random_call(100, 17, 63001)
        self.assertEqual(drawn, V.random_call(100, 17, 63001))
        self.assertEqual(len(set(drawn)), 17)
        self.assertEqual(V.random_call(5, 17, 1), [0, 1, 2, 3, 4])


if __name__ == "__main__":
    unittest.main()

"""The tactician trainer on synthetic records: reading, learning, export."""
import json
import tempfile
import unittest
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch

import tactician4 as T

D = 8


def write_records(directory: Path, decisions: int, seed: int):
    """Target questions of three units; taking unit 2 wins more when look
    column 40 of its row is high. The behavior plays the net's unit 0 with
    0.75 and the others with 0.125 each."""
    rng = np.random.default_rng(seed)
    directory.mkdir(parents=True)
    lines, blob, offset = [], bytearray(), 0
    for i in range(decisions):
        n = 3
        state = rng.normal(size=D).astype("<f4")
        units = rng.normal(size=(n, D)).astype("<f4")
        tails = np.zeros((n + 1, T.ROW_TAIL), "<f4")
        tails[1:, 16 + 40] = rng.uniform(size=n)
        draw = rng.uniform()
        played_unit = 0 if draw < 0.75 else (1 if draw < 0.875 else 2)
        played = [int(u == played_unit) for u in range(n)]
        p_win = 0.4 + (0.4 * tails[3, 16 + 40] if played_unit == 2 else 0.0)
        utility = float(rng.uniform() < p_win)
        record = dict(contract=T.RECORD_CONTRACT, seat=0, seq=i, turn=3, window="target",
                      value=0.45, keys=["a", "b", "c"], logits=[1.0, 0.0, 0.0],
                      net=[1, 0, 0], rules=[0, 0, 1], played=played,
                      probability=0.75 if played_unit == 0 else 0.125, neighbors=2,
                      offered=[[0, 1, 0], [0, 0, 1]], drawn="net", micros=10,
                      game=f"g{i}", deal=i, seed=1000 + i, policy_seed=1, focal_seat=0,
                      utility=utility, capped=False,
                      blob=dict(file="w00.f32", offset=offset, state=D, units=n * D,
                                rows=(n + 1) * T.ROW_TAIL))
        for array in (state, units, tails):
            blob += array.tobytes()
            offset += array.nbytes
        lines.append(json.dumps(record))
    (directory / "w00.jsonl").write_text("\n".join(lines) + "\n")
    (directory / "w00.f32").write_bytes(bytes(blob))


class TacticianTrainer(unittest.TestCase):
    def test_reads_learns_beats_zero_and_exports_what_it_learnt(self):
        torch.set_num_threads(1)
        with tempfile.TemporaryDirectory() as root:
            records = Path(root) / "records"
            write_records(records, 3000, 5)
            loaded = T.read_records([records])
            self.assertEqual(len(loaded), 3000)
            self.assertEqual(loaded[7]["tails"].shape, (4, T.ROW_TAIL))
            out = Path(root) / "head"
            meta = T.main(["--records", str(records), "--out", str(out), "--seed", "3",
                           "--epochs", "12", "--lr", "1e-3", "--device", "cpu"])
            self.assertEqual(meta["contract"], T.CONTRACT)
            self.assertEqual(meta["width"], 2 * D + T.ROW_TAIL)
            self.assertIn(meta["margin"], T.MARGINS)
            best = min(h["held_out"] for h in meta["history"][1:])
            self.assertLess(best, meta["history"][0]["held_out_zero"])
            self.assertEqual(meta["decisions"]["held_out"], 300)
            # The export scores rows as the torch head does.
            state = torch.load(out / "head.pt")
            head = T.TacticianHead(state["width"])
            head.load_state_dict(state["state_dict"])
            rows = torch.randn(5, state["width"])
            session = ort.InferenceSession(str(out / "tactician.onnx"),
                                           providers=["CPUExecutionProvider"])
            got = session.run(["score"], {"rows": rows.numpy()})[0]
            with torch.no_grad():
                np.testing.assert_allclose(got, head(rows).numpy(), rtol=1e-4, atol=1e-5)
            with self.assertRaises(FileExistsError):
                T.main(["--records", str(records), "--out", str(out), "--seed", "3",
                        "--epochs", "1", "--device", "cpu"])

    def test_the_choice_keeps_the_net_unless_past_the_margin(self):
        record = dict(net=[1, 0, 0], rules=[0, 0, 1], offered=[[0, 1, 0], [0, 0, 1]])
        scores = [0.0, 0.1, 0.0, 0.13]
        self.assertEqual(T.choice(scores, record, 0.0), [0, 0, 1])
        self.assertEqual(T.choice(scores, record, 0.03), [1, 0, 0])
        self.assertEqual(T.pick_margin({0.0: 0.1, 0.01: 0.1, 0.02: 0.05}), 0.01)

    def test_delta_takes_and_compact_head(self):
        torch.set_num_threads(1)
        with tempfile.TemporaryDirectory() as root:
            records = Path(root) / "records"
            write_records(records, 600, 7)
            out = Path(root) / "head_compact"
            meta = T.main(["--records", str(records), "--out", str(out), "--seed", "5",
                           "--epochs", "6", "--lr", "1e-3", "--hidden-width", "32",
                           "--delta-takes", "--turn-target-blend", "0.5", "--device", "cpu"])
            self.assertEqual(meta["hidden_width"], 32)
            self.assertTrue(meta["delta_takes"])
            self.assertEqual(meta["turn_target_blend"], 0.5)


if __name__ == "__main__":
    unittest.main()

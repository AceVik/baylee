import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import publish_live_model4 as live


class LiveSelection(unittest.TestCase):
    def test_partial_exports_and_changed_evaluated_weights_never_publish(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            model = root / "model"
            model.mkdir()
            metadata = dict(graphs={"0": "model.k0.onnx", "1": "model.k1.onnx"},
                            checked_frames=64, parity_max_abs_diff=.0001)
            for name in ("net.pt", "model.onnx", "model.k0.onnx", "model.k1.onnx", "card_structure.json"):
                (model / name).write_bytes(b"frozen")
            (model / "model.onnx.json").write_text(json.dumps(metadata))
            (root / "progress.json").write_text("{}")
            job = dict(inputs={str(p): live.digest(p) for p in model.iterdir()})
            result = model, "round-02", .2, {"same": "deals"}, {}, [job, job]
            with patch.object(live, "measured_candidate", return_value=result):
                selected = live.manifest(root)
                self.assertEqual(selected["label"], "v4-R02")
                (model / "net.pt").write_bytes(b"changed")
                with self.assertRaisesRegex(ValueError, "changed since"):
                    live.manifest(root)
                (model / "net.pt").write_bytes(b"frozen")
                (model / "model.k0.onnx").unlink()
                with self.assertRaises(FileNotFoundError):
                    live.manifest(root)

    def test_lower_scores_and_different_benchmarks_cannot_replace_current(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / "selected.json"
            current = dict(model="/frozen/old", score=.2, benchmark_id="paired")
            output.write_text(json.dumps(current))
            (root / "progress.json").write_text(json.dumps(dict(best_development_candidate="/frozen/new")))
            candidate = dict(model="/frozen/new", score=.19, benchmark_id="paired")
            with patch.object(live, "manifest", return_value=candidate):
                self.assertFalse(live.publish(root, output))
                self.assertEqual(json.loads(output.read_text()), current)
                candidate.update(score=.21, benchmark_id="incomparable")
                with self.assertRaisesRegex(ValueError, "Different benchmark"):
                    live.publish(root, output)
                self.assertEqual(json.loads(output.read_text()), current)
                candidate["benchmark_id"] = "paired"
                self.assertTrue(live.publish(root, output))
                self.assertEqual(json.loads(output.read_text()), candidate)

    def test_incomplete_evaluation_job_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "evaluation").mkdir()
            job = root / "jobs/eval-round-02-duels"
            job.mkdir(parents=True)
            (root / "evaluation/round-02-duels.json").write_text("{}")
            (job / "job.json").write_text(json.dumps(dict(status="running")))
            with self.assertRaisesRegex(ValueError, "completed evaluation"):
                live.checked_evaluation(root, "round-02", "duels")


if __name__ == "__main__":
    unittest.main()

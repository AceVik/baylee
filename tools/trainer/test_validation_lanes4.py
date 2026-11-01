"""A learning pilot keeps validation grouping fixed and preserves it on resume."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import torch
from test_recurrent4 import write_dataset


class ValidationLanes(unittest.TestCase):
    def test_training_with_separate_validation_lanes_and_resume_protocol(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / "data"
            data.mkdir()
            write_dataset(data, [5, 7, 9, 11], candidates=2)
            (data / "val").mkdir()
            shutil.copy2(data / "train/steps.jsonl.gz", data / "val/steps.jsonl.gz")
            meta = json.loads((data / "meta.json").read_text())
            meta["val_shards"] = ["steps.jsonl.gz"]
            (data / "meta.json").write_text(json.dumps(meta))
            out = root / "out"
            command = [sys.executable, str(Path(__file__).with_name("train4.py")),
                       "--data", str(data), "--out", str(out), "--d-model", "32",
                       "--layers", "1", "--heads", "4", "--d-ff", "64",
                       "--batch-size", "4", "--unroll", "4", "--epochs", "1",
                       "--validation-every", "0", "--async-checkpoint", "--validate-initial"]
            env = dict(os.environ, CUDA_VISIBLE_DEVICES="", OMP_NUM_THREADS="1", MKL_NUM_THREADS="1")

            def run(extra):
                return subprocess.run(command + extra, env=env, text=True, capture_output=True, timeout=30)

            first = run(["--validation-batch-size", "2", "--max-steps", "1"])
            self.assertEqual(first.returncode, 0, first.stderr)
            saved = torch.load(out / "last.pt", map_location="cpu", weights_only=False)
            self.assertEqual((saved["batch_size"], saved["validation_batch_size"]), (4, 2))
            self.assertEqual(saved["h"].shape[0], 4)
            self.assertEqual(saved["last_validation_step"], 1)
            self.assertEqual(saved["last_validation_metrics"]["val_frames"], 32)
            self.assertEqual(saved["training_loss_version"], 4)
            self.assertEqual(saved["checkpoint_selection"], "policy")
            self.assertGreater(saved["last_validation_metrics"]["val_choices"], 0)
            self.assertIn("step=0", first.stdout)
            metrics = json.loads((out / "meta.json").read_text())
            self.assertEqual(metrics["validation_batch_size"], 2)
            self.assertEqual(metrics["val_metrics"]["val_frames"], 32)
            resume = ["--resume", str(out / "last.pt"), "--max-steps", "2"]
            wrong_objective = run(resume + ["--validation-batch-size", "2", "--checkpoint-selection", "combined"])
            self.assertNotEqual(wrong_objective.returncode, 0)
            self.assertIn("Resume checkpoint selection differs", wrong_objective.stderr)
            wrong_seed = run(resume + ["--validation-batch-size", "2", "--seed", "999"])
            self.assertNotEqual(wrong_seed.returncode, 0)
            self.assertIn("Resume training sampler version/seed", wrong_seed.stderr)
            old_sampler = dict(saved)
            old_sampler.pop("training_sampler_version")
            torch.save(old_sampler, out / "old-sampler.pt")
            old = run(["--resume", str(out / "old-sampler.pt"), "--validation-batch-size", "2"])
            self.assertNotEqual(old.returncode, 0)
            self.assertIn("Resume training sampler version/seed", old.stderr)
            rejected = run(resume + ["--validation-batch-size", "4"])
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("Resume validation batch lanes", rejected.stderr)
            accepted = run(resume + ["--validation-batch-size", "2"])
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            saved = torch.load(out / "last.pt", map_location="cpu", weights_only=False)
            self.assertEqual(saved["step_num"], 2)
            self.assertEqual(saved["h"].shape[0], 4)
            self.assertEqual(saved["last_validation_step"], 2)
            self.assertEqual(saved["last_validation_metrics"]["val_frames"], 32)


if __name__ == "__main__":
    unittest.main()

"""Check snapshot consistency across subsequent updates and atomic failures."""
from collections import OrderedDict
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

import numpy as np
import torch

from checkpoint4 import atomic_save
from training4.checkpoint_async import AsyncCheckpointWriter


class AsyncCheckpointTests(unittest.TestCase):
    def test_background_write_is_immutable_and_publishes_atomically(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "last.pt"
            atomic_save({"step": 0}, path)
            started, release = threading.Event(), threading.Event()
            state = OrderedDict(weight=torch.arange(5.0))
            state._metadata = {"": {"version": 2}}
            payload = {"state_dict": state, "rng": np.arange(4),
                       "optimizer": [{"step": torch.tensor(3.0)}], "h": torch.ones(2, 9, 3)}

            def blocked_write(frozen, target):
                started.set()
                if not release.wait(5):
                    raise TimeoutError("Test did not release checkpoint writer")
                atomic_save(frozen, target)

            with patch("training4.checkpoint_async.atomic_save", side_effect=blocked_write):
                with AsyncCheckpointWriter("cpu") as writer:
                    try:
                        writer.save(payload, path)
                        self.assertTrue(started.wait(2))
                        self.assertEqual(torch.load(path, weights_only=False)["step"], 0)
                        payload["state_dict"]["weight"].add_(100)
                        payload["state_dict"]._metadata[""]["version"] = 99
                        payload["rng"][:] = 99
                        payload["optimizer"][0]["step"].add_(1)
                        payload["h"].zero_()
                        writer.before_mutation()
                    finally:
                        release.set()
            saved = torch.load(path, weights_only=False)
            torch.testing.assert_close(saved["state_dict"]["weight"], torch.arange(5.0))
            self.assertEqual(saved["state_dict"]._metadata[""]["version"], 2)
            np.testing.assert_array_equal(saved["rng"], np.arange(4))
            self.assertEqual(saved["optimizer"][0]["step"].item(), 3)
            self.assertTrue(saved["h"].eq(1).all())
            self.assertFalse(path.with_name("last.pt.partial").exists())

    def test_write_error_reaches_caller_and_preserves_previous_checkpoint(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "last.pt"
            atomic_save({"step": 7}, path)
            with patch("training4.checkpoint_async.atomic_save", side_effect=OSError("disk full")):
                writer = AsyncCheckpointWriter("cpu")
                writer.save({"step": 8}, path)
                with self.assertRaisesRegex(OSError, "disk full"):
                    writer.close()
                with self.assertRaisesRegex(RuntimeError, "closed"):
                    writer.save({}, path)
            self.assertEqual(torch.load(path, weights_only=False)["step"], 7)

    @unittest.skipUnless(torch.cuda.is_available(), "CUDA snapshot ordering")
    def test_cuda_snapshot_precedes_parameter_optimizer_and_memory_mutation(self):
        torch.set_num_threads(1)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "last.pt"
            compute = torch.cuda.Stream()
            with torch.cuda.stream(compute), AsyncCheckpointWriter("cuda") as writer:
                model = torch.nn.Linear(64, 64).cuda()
                optimizer = torch.optim.AdamW(model.parameters(), lr=0.01)
                model(torch.ones(4, 64, device="cuda")).sum().backward()
                optimizer.step()
                hidden = torch.full((1024, 9, 64), 0.25, device="cuda")
                expected_weight = model.weight.detach().cpu().clone()
                expected_moment = optimizer.state[model.weight]["exp_avg"].cpu().clone()
                writer.save({"model": model.state_dict(), "optimizer": optimizer.state_dict(),
                             "h": hidden, "next_batch": 17}, path)
                # Forward/backward are allowed while the copy is in flight.
                optimizer.zero_grad(set_to_none=True)
                model(torch.ones(4, 64, device="cuda")).square().sum().backward()
                writer.before_mutation()
                optimizer.step()
                hidden.add_(5)
            saved = torch.load(path, map_location="cpu", weights_only=False)
            torch.testing.assert_close(saved["model"]["weight"], expected_weight, rtol=0, atol=0)
            torch.testing.assert_close(saved["optimizer"]["state"][0]["exp_avg"], expected_moment, rtol=0, atol=0)
            self.assertEqual(saved["optimizer"]["state"][0]["step"].item(), 1)
            self.assertEqual(saved["next_batch"], 17)
            self.assertTrue(saved["h"].eq(0.25).all())
            resumed = torch.nn.Linear(64, 64)
            resumed.load_state_dict(saved["model"])
            resumed_optimizer = torch.optim.AdamW(resumed.parameters())
            resumed_optimizer.load_state_dict(saved["optimizer"])
            resumed(torch.ones(4, 64)).sum().backward()
            resumed_optimizer.step()
            self.assertEqual(resumed_optimizer.state[resumed.weight]["step"].item(), 2)


if __name__ == "__main__":
    unittest.main()

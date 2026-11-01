"""A resumed H must follow exactly the sampler lanes and cursor that produced it."""
import unittest
from types import SimpleNamespace

from benchmark_state4 import checkpoint_sampler
from sampler4 import StatefulBatchSampler4


class BenchmarkSampler(unittest.TestCase):
    def setUp(self):
        self.dataset = SimpleNamespace(trajectories=[list(range(i * 10, i * 10 + 7)) for i in range(12)])
        self.saved = dict(training_sampler_version=StatefulBatchSampler4.VERSION,
                          sampler_seed=31423, batch_size=3, epoch=4, next_batch=5)

    def test_resume_matches_original_lanes_and_nonzero_cursor(self):
        original = StatefulBatchSampler4(self.dataset, 3, shuffle=True, seed=31423)
        original.epoch = 3
        complete = list(original)
        resumed = checkpoint_sampler(self.dataset, self.saved, 3)
        self.assertEqual(list(resumed), complete[5:])
        wrong = StatefulBatchSampler4(self.dataset, 3, shuffle=True, seed=42)
        wrong.epoch = 3
        self.assertNotEqual(list(wrong)[5:], complete[5:])

    def test_resume_rejects_incompatible_or_missing_contract(self):
        for change in ({"training_sampler_version": -1}, {"sampler_seed": None}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                checkpoint_sampler(self.dataset, {**self.saved, **change}, 3)
        with self.assertRaises(ValueError):
            checkpoint_sampler(self.dataset, self.saved, 3, seed=42)
        with self.assertRaises(ValueError):
            checkpoint_sampler(self.dataset, self.saved, 4)

    def test_fresh_lanes_reset_cursor_and_allow_explicit_seed(self):
        fresh = checkpoint_sampler(self.dataset, {**self.saved, "training_sampler_version": -1},
                                   4, fresh=True, seed=42)
        expected = StatefulBatchSampler4(self.dataset, 4, shuffle=True, seed=42)
        expected.epoch = 3
        self.assertEqual(fresh.start_batch, 0)
        self.assertEqual(list(fresh), list(expected))


if __name__ == "__main__":
    unittest.main()

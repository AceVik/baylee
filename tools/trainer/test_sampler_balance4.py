"""Length balancing preserves recurrent causality and avoids empty training tails."""
import random
import unittest

from sampler4 import StatefulBatchSampler4


def dataset(lengths):
    trajectories, position = [], 0
    for length in lengths:
        trajectories.append(list(range(position, position + length)))
        position += length
    return type("Dataset", (), {"trajectories": trajectories})()


class BalancedLanes(unittest.TestCase):
    def test_all_segments_once_in_consecutive_persistent_lanes(self):
        for seed in range(20):
            rng = random.Random(seed)
            ds = dataset([rng.randrange(1, 80) for _ in range(60)])
            for size in (1, 2, 8, 64):
                sampler = StatefulBatchSampler4(ds, size, seed=seed)
                expected_length = len(sampler)
                batches = list(sampler)
                self.assertEqual(len(batches), expected_length)
                locations = {}
                for time, batch in enumerate(batches):
                    for lane, segment in enumerate(batch):
                        if segment >= 0:
                            self.assertNotIn(segment, locations)
                            locations[segment] = (time, lane)
                self.assertEqual(set(locations), {i for tr in ds.trajectories for i in tr})
                for tr in ds.trajectories:
                    time, lane = locations[tr[0]]
                    self.assertEqual([locations[i] for i in tr],
                                     [(time + offset, lane) for offset in range(len(tr))])

    def test_unequal_trajectories_keep_training_lanes_full(self):
        ds = dataset([200, 180, 160, 140, 120, 100, 80, 60] + [7] * 120 + [1] * 64)
        batches = list(StatefulBatchSampler4(ds, 8, seed=2719))
        occupied = sum(i >= 0 for batch in batches for i in batch)
        self.assertGreater(occupied / (len(batches) * 8), .99)

    def test_seed_epoch_and_resume_suffix_are_reproducible(self):
        ds = dataset([7] * 40)
        first = StatefulBatchSampler4(ds, 8, seed=42)
        epoch0, epoch1 = list(first), list(first)
        self.assertNotEqual(epoch0, epoch1)
        again = StatefulBatchSampler4(ds, 8, seed=42)
        self.assertEqual(list(again), epoch0)
        again.epoch = 1
        again.start_batch = 4
        self.assertEqual(list(again), epoch1[4:])
        self.assertNotEqual(list(StatefulBatchSampler4(ds, 8, seed=43)), epoch0)

    def test_validation_keeps_existing_grouping(self):
        ds = dataset([2, 4, 1])
        self.assertEqual(list(StatefulBatchSampler4(ds, 2, shuffle=False)),
                         [[0, 2], [1, 3], [6, 4], [-1, 5]])


if __name__ == "__main__":
    unittest.main()

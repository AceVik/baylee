"""Copy/compute overlap must preserve order, data, fallback and buffer lifetime."""
import unittest
import torch
from training4.prefetch import prefetch_batches


class GpuPrefetch(unittest.TestCase):
    def test_cpu_path_keeps_original_objects_and_order(self):
        batches = [{"x": torch.tensor([i])} for i in range(5)]
        for ahead in range(4):
            actual = list(prefetch_batches(batches, torch.device("cpu"), ahead))
            for expected, (host, staged) in zip(batches, actual):
                self.assertIs(expected, host)
                self.assertIsNone(staged)

    @unittest.skipUnless(torch.cuda.is_available(), "CUDA copy-stream lifetime check")
    def test_cuda_order_oversized_fallback_and_early_close(self):
        device = torch.device("cuda")
        batches = [{"x": torch.full((8192+i,), float(i)).pin_memory(),
                    "mask": torch.tensor([i % 2 == 0]).pin_memory()} for i in range(12)]
        for ahead in (1, 2, 3):
            sums = []
            iterator = prefetch_batches(batches, device, ahead)
            try:
                for host, staged in iterator:
                    self.assertIsNotNone(staged)
                    self.assertEqual(staged["mask"].dtype, torch.bool)
                    # Do not synchronize per batch: consume allocations on the
                    # default stream while the producer reuses its own buffers.
                    sums.append(staged["x"].square().sum())
            finally:
                iterator.close()
            expected = torch.tensor([(8192+i)*i*i for i in range(12)], dtype=torch.float32)
            torch.testing.assert_close(torch.stack(sums).cpu(), expected, atol=0, rtol=0)
        iterator = prefetch_batches(batches, device, 3)
        try:
            host, staged = next(iterator)
            result = staged["x"].sum()
        finally:
            iterator.close()
        self.assertEqual(float(result.cpu()), 0)
        for host, staged in prefetch_batches(batches, device, 1, limit_bytes=1):
            self.assertIsNone(staged)
        self.assertIsNone(next(prefetch_batches([{"x": torch.ones(3)}], device, 1))[1])


if __name__ == "__main__":
    unittest.main()

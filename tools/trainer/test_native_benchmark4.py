"""Native host statistics around the retained Torch measurement boundaries."""
import os
from pathlib import Path
import statistics
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import torch
from torch.utils.data import DataLoader
from legacy_evaluation import native_metrics


class NativeBenchmark(unittest.TestCase):
    def setUp(self):
        binary = Path(__file__).resolve().parents[2] / 'target/debug/legacy_metrics'
        self.environment = patch.dict(os.environ, BAYLEE_LEGACY_METRICS=str(binary))
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def test_quantile_definitions_match_original_numpy_and_statistics(self):
        for length in (1, 2, 3, 8, 20, 101, 129, 500):
            samples = torch.rand(length, generator=torch.Generator().manual_seed(length),
                                 dtype=torch.float64).mul(3).add(.001).tolist()
            samples.reverse()
            with self.subTest(length=length):
                inference = native_metrics('benchmark-statistics', mode='inference', samples=samples)
                self.assertEqual(inference['p50_ms'], float(np.median(samples)))
                self.assertEqual(inference['p95_ms'], float(np.quantile(samples, .95)))
                t1 = native_metrics('benchmark-statistics', mode='t1', samples=samples)
                self.assertEqual(t1, {f'p{q}_ms': round(float(np.percentile(samples, q)), 2)
                                      for q in (50, 95, 99)})
                ordered = sorted(samples)
                median = statistics.median(ordered)
                step = native_metrics('benchmark-statistics', mode='step', samples=samples, batch=3)
                self.assertEqual(step, dict(median_ms=round(median*1000, 3),
                    p95_ms=round(ordered[int(length*.95)]*1000, 3), decisions_per_s=round(3/median)))
                training = native_metrics('benchmark-statistics', mode='training', samples=samples,
                                          frames=123, decisions=31)
                self.assertEqual(training, dict(updates_per_second=1000/np.mean(samples),
                    valid_frames_per_second=123/(sum(samples)/1000),
                    decisions_per_second=31/(sum(samples)/1000)))
        for sample in (2.675, .125, .375, 1.005):
            result = native_metrics('benchmark-statistics', mode='t1', samples=[sample])
            self.assertEqual(result['p50_ms'], round(sample, 2))

    def test_reference_calibration_preserves_fp32_bins_and_step_sums(self):
        for length in (1, 3, 8, 20, 129, 500):
            prediction = torch.rand(length, generator=torch.Generator().manual_seed(length)).numpy()
            prediction[0] = np.nextafter(np.float32(.3), np.float32(0))
            truth = torch.arange(length).remainder(2).float().numpy()
            state = native_metrics('reference-calibration', prediction=prediction.tolist(), truth=truth.tolist(),
                steps=[[0., 0., 0., length, 0., 0., 0., 0., 0.]])['calibration']
            self.assertEqual(state[20], float(prediction.sum()))
            self.assertEqual(state[21], float(truth.sum()))
            bins = np.minimum((prediction*10).astype(int), 9)
            expected = np.zeros(10)
            np.add.at(expected, bins, prediction)
            self.assertEqual(state[:10], expected.tolist())

    def test_native_resource_reports_preserve_python_rounding_and_units(self):
        elapsed = float(torch.tensor(1.125, dtype=torch.float64))
        result = native_metrics('benchmark-resources', mode='step-training',
                                elapsed=elapsed, steps=7, batch=4, peak_bytes=123456789)
        self.assertEqual(result, dict(steps_per_s=round(7/elapsed, 2),
            samples_per_s=round(7*4/elapsed), peak_gb=round(123456789/2**30, 2)))
        result = native_metrics('benchmark-resources', mode='t1-training',
            elapsed=elapsed, iterations=3, parameters=12345678, batch=2, unroll=4, peak_bytes=123456789)
        each = elapsed/3
        self.assertEqual(result, dict(params_m=round(12345678/1e6, 2),
            peak_vram_mb=round(123456789/(1024*1024), 1), step_ms=round(each*1000, 1),
            decision_steps_per_s=round((2*4)/each, 1)))
        self.assertEqual(native_metrics('benchmark-resources', mode='speedup', cpu=3., gpu=2., batch=4), 6.)

    def test_real_frame_sampling_plan_and_stable_tied_selection(self):
        for length in (1, 2, 119, 120, 1000, 149014):
            self.assertEqual(native_metrics('benchmark-plan', mode='rows', length=length),
                             np.linspace(0, length-1, 120, dtype=int).tolist())
        for entities in ([3], [3, 3], [5, 3, 3, 8, 2, 8]):
            ordered = sorted(range(len(entities)), key=entities.__getitem__)
            expected = [ordered[i] for i in sorted({0, len(ordered)//2, len(ordered)-1})]
            self.assertEqual(native_metrics('benchmark-plan', mode='select', entities=entities), expected)

    def test_torch_callback_timing_keeps_warmup_and_invocation_count(self):
        from bench_inference4 import timed
        calls = []
        with patch('bench_inference4.time.perf_counter', side_effect=[0., .001, 1., 1.003]):
            result = timed(lambda: calls.append(torch.ones(1).sum()), 2)
        self.assertEqual(len(calls), 5)
        expected = [(.001-0.)*1000, (1.003-1.)*1000]
        self.assertEqual(result['p50_ms'], np.median(expected))
        self.assertEqual(result['p95_ms'], np.quantile(expected, .95))

    def test_cpu_reference_validation_matches_device_metrics_without_updates(self):
        import dataset4 as D
        import model4 as M
        from test_recurrent4 import write_dataset
        from training4.validation import evaluate
        from training4.validation_device import evaluate_device
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_dataset(root, [3, 7, 5], candidates=2)
            dataset = D.TrajectoryDataset4(root, unroll_len=4)
            self.addCleanup(dataset.store.close)
            def loader():
                return DataLoader(dataset, batch_sampler=D.StatefulBatchSampler4(dataset, 2, shuffle=False),
                                  collate_fn=D.collate_trajectories)
            torch.manual_seed(13)
            net = M.PlayerNet4(M.Config4(d_model=32, n_heads=4, n_layers=1, d_ff=64)).eval()
            net.attach_structure(dataset.structure.data)
            before = {key: value.clone() for key, value in net.state_dict().items()}
            reference = evaluate(net, loader(), torch.device('cpu'), 2)
            device = evaluate_device(net, loader(), torch.device('cpu'), 2)
            self.assertEqual(reference.keys(), device.keys())
            self.assertEqual(reference['val_frames'], 15)
            for key in reference:
                self.assertAlmostEqual(reference[key], device[key], delta=2e-6, msg=key)
            self.assertIsInstance(reference['val_frames'], int)
            self.assertIsInstance(reference['val_active_choices'], int)
            for key, value in net.state_dict().items():
                self.assertTrue(torch.equal(value, before[key]), key)


if __name__ == '__main__':
    unittest.main()

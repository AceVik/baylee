import unittest
import torch
from batch_audit5.worker import measure


class AuditTests(unittest.TestCase):
    def test_padding_and_inactive_time_denominator(self):
        valid=torch.tensor([[True,True,False],[True,False,False]])
        masks=torch.tensor([[[True,True],[True,False],[True,True]],[[True,False],[False,False],[False,False]]])
        batch=dict(head=torch.zeros_like(valid,dtype=torch.long),step_valid=valid,is_decision=torch.tensor([[True,False,False],[True,False,False]]))
        for name in ('entity','edge','cand','pair'):batch[name+'_mask']=masks
        result=measure(batch)
        self.assertEqual(result['thinking_decision_rows'],2)
        self.assertEqual(result['active_timesteps'],2)
        self.assertEqual(result['entities']['actual_entries'],4)
        self.assertEqual(result['entity_attention_E2']['actual_entries'],6)
        self.assertEqual(result['logical_batch_bytes'],sum(x.numel()*x.element_size() for x in batch.values()))
        self.assertLess(result['raw_batch_bytes'],result['logical_batch_bytes']) # shared mask storage counted once
        self.assertEqual(result['decision_head_counts']['0'],2)



class NativeAuditIntegration(unittest.TestCase):
    def test_complete_episode_tensor_measurement_native_report(self):
        import json
        import os
        from pathlib import Path
        import subprocess
        import sys
        import tempfile
        from unittest.mock import patch
        from test_recurrent4 import write_dataset
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            data = root / 'data'
            data.mkdir()
            write_dataset(data, [3, 5, 4], candidates=3)
            trainer = Path(__file__).resolve().parent
            binary_dir = trainer.parents[1] / 'target/debug'
            job = root / 'job.json'
            job.write_text(json.dumps(dict(command=[sys.executable, '-u', str(trainer / 'train4.py'),
                '--data', str(data), '--unroll', '2', '--batch-size', '2', '--seed', '17'])))
            out = root / 'report.json'
            with patch.dict(os.environ, PATH=str(binary_dir) + os.pathsep + os.environ['PATH']):
                completed = subprocess.run([str(binary_dir / 'audit_batches5'), '--job', str(job), '--out', str(out),
                                '--python', sys.executable, '--samples', '4', '--budget-mib', '1'],
                               capture_output=True, text=True)
                self.assertEqual(completed.returncode, 0, completed.stderr)
            report = json.loads(out.read_text())
            self.assertEqual(report['schema_version'], 2)
            self.assertEqual(report['sampling_version'], 'integer_even_v1')
            self.assertGreater(report['measured'], 0)
            self.assertEqual(report['skipped'], 0)
            self.assertEqual(report['benchmark_windows']['first'], 0)
            for batch in report['batches']:
                self.assertEqual(batch['raw_batch_bytes'], batch['predicted_collated_bytes'])
                self.assertLessEqual(batch['thinking_row_fraction_B_active_time'], 1)

if __name__ == '__main__':
    unittest.main()

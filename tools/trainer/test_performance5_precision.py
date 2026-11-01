"""Future probe controls and metadata, without CUDA initialization or timing."""
from types import SimpleNamespace as NS
import unittest
import sys
from unittest.mock import patch
from performance5 import precision
from performance5.runner import VARIANTS, configure


class PrecisionTests(unittest.TestCase):
    def fake(self, warn=True):
        calls=[]
        fake=NS(backends=NS(cuda=NS(matmul=NS(allow_tf32=True)),
                          cudnn=NS(allow_tf32=True, deterministic=False, benchmark=True)),
                get_float32_matmul_precision=lambda:'high',
                are_deterministic_algorithms_enabled=lambda:True)
        def set_precision(value):
            calls.append(value)
            fake.get_float32_matmul_precision=lambda:value
        fake.set_float32_matmul_precision=set_precision
        if warn:fake.is_deterministic_algorithms_warn_only_enabled=lambda:True
        return fake,calls

    def test_identical_control_variant_configures_identical_execution(self):
        self.assertEqual(VARIANTS['baseline'],VARIANTS['baseline-repeat'])
        self.assertIsNot(VARIANTS['baseline'],VARIANTS['baseline-repeat'])
        cfg=NS(activation_checkpointing=True,n_layers=2)
        first,second=NS(cfg=cfg),NS(cfg=cfg)
        configure(first,VARIANTS['baseline'],False)
        configure(second,VARIANTS['baseline-repeat'],False)
        self.assertEqual(vars(first),vars(second))

    def test_inherit_records_effective_flags_without_changing_them(self):
        fake,calls=self.fake()
        with patch.object(precision,'torch',fake):
            report=precision.configure('inherit',True)
            self.assertEqual(report,dict(float32_matmul_precision='high',cuda_matmul_allow_tf32=True,
                cudnn_allow_tf32=True,cudnn_deterministic=False,cudnn_benchmark=True,
                deterministic_algorithms=True,deterministic_algorithms_warn_only=True,compile_enabled=True))
            self.assertEqual(calls,[])
            self.assertEqual(precision.require_unchanged(report,True),report)
            fake.backends.cuda.matmul.allow_tf32=False
            with self.assertRaisesRegex(RuntimeError,'precision flags changed'):
                precision.require_unchanged(report,True)

    def test_cli_defaults_inherit_and_highest_is_explicit(self):
        from performance5.runner import main
        base=['bench','--checkpoint','unused','--data','unused','--out','unused']
        for extra,expected in [([], 'inherit'),(['--float32-precision','highest'],'highest')]:
            with patch.object(sys,'argv',base+extra),patch('performance5.runner.run') as run:
                main()
            args=run.call_args.args[0]
            self.assertEqual(args.float32_precision,expected)
            self.assertEqual(args.variants.split(',')[:2],['baseline','baseline-repeat'])

    def test_highest_disables_both_tf32_controls_and_keeps_other_flags(self):
        fake,calls=self.fake(warn=False)
        with patch.object(precision,'torch',fake):
            report=precision.configure('highest',False)
            self.assertEqual(calls,['highest'])
            self.assertEqual(report['float32_matmul_precision'],'highest')
            self.assertFalse(report['cuda_matmul_allow_tf32'])
            self.assertFalse(report['cudnn_allow_tf32'])
            self.assertTrue(report['cudnn_benchmark'])
            self.assertTrue(report['deterministic_algorithms'])
            self.assertIsNone(report['deterministic_algorithms_warn_only'])
            self.assertFalse(report['compile_enabled'])
            with self.assertRaises(ValueError):precision.configure('unknown',False)


if __name__=='__main__':unittest.main()

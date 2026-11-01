"""Real native publication must preserve checkpoints on Torch serialization failure."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import torch
from checkpoint4 import atomic_save
from tensor_io4 import TensorPublication


class TensorIO(unittest.TestCase):
    def test_failed_torch_serialization_never_publishes_its_partial_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'net.pt'
            atomic_save({'tensor':torch.ones(2)},path)
            def fail(payload, temporary):
                Path(temporary).write_bytes(b'partial-torch-zip')
                raise RuntimeError('torch serialization failed')
            with patch('checkpoint4.torch.save',side_effect=fail),self.assertRaisesRegex(RuntimeError,'serialization failed'):
                atomic_save({'tensor':torch.zeros(2)},path)
            torch.testing.assert_close(torch.load(path,weights_only=False)['tensor'],torch.ones(2))
            self.assertEqual(list(Path(directory).glob('net.pt.partial-*')),[])

    def test_disconnect_aborts_native_reservation_after_real_torch_save(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'net.pt'
            publication=TensorPublication('file',path).__enter__()
            temporary=publication.path
            torch.save({'tensor':torch.ones(2)},temporary)
            publication._close()
            self.assertFalse(path.exists())
            self.assertFalse(temporary.exists())

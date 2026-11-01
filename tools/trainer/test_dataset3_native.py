"""Actual native v3 cache integration with the preserved Torch dataset adapter."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import torch
import dataset3 as D3


class NativeDataset3Tests(unittest.TestCase):
    def test_native_rebase_cache_and_torch_mapping(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            meta = dict(encoder_version=3, ent_cols=['a', 'b'], seat_cols=['a', 'b'],
                        glob_cols=['turn'], meta_cols=['game', 'seat'], deck_cols=['n'], max_seats=2,
                        shards=[dict(dir='a', samples=1, entities=2, decks=1, opts=2),
                                dict(dir='b', samples=1, entities=1, decks=2, opts=1)])
            (root / 'dataset.json').write_text(json.dumps(meta))
            (root / 'games.jsonl').write_text('{}\n{}\n')
            np.array([7], dtype='<i4').tofile(root / 'card_ids.i32')
            np.array([.5], dtype='<f4').tofile(root / 'card_feats.f32')
            for index, shard in enumerate(meta['shards']):
                directory = root / shard['dir']
                directory.mkdir()
                columns = {
                    'ent_card.i32': np.arange(shard['entities'], dtype='<i4'),
                    'ent_feat.i16': np.zeros((shard['entities'], 2), dtype='<i2'),
                    'ent_off.i64': np.array([0, shard['entities']], dtype='<i8'),
                    'seat.i16': np.zeros((1, 2, 2), dtype='<i2'),
                    'glob.i16': np.array([index + 1], dtype='<i2'),
                    'meta.i32': np.array([index, 0], dtype='<i4'),
                    'deck_card.i32': np.zeros(shard['decks'], dtype='<i4'),
                    'deck_feat.i16': np.zeros(shard['decks'], dtype='<i2'),
                    'deck_off.i64': np.array([0, shard['decks']], dtype='<i8'),
                    'opt.i16': np.zeros((shard['opts'], 3), dtype='<i2'),
                    'opt_off.i64': np.array([0, shard['opts']], dtype='<i8'),
                }
                for name, values in columns.items():
                    values.tofile(directory / name)
            binary_dir = Path(__file__).resolve().parents[2] / 'target/debug'
            with patch.dict(os.environ, PATH=str(binary_dir) + os.pathsep + os.environ['PATH']):
                dataset = D3.load(root)
                self.assertEqual(dataset.ent_off.tolist(), [0, 2, 3])
                self.assertEqual(dataset.deck_off.tolist(), [0, 1, 3])
                self.assertEqual(dataset.opt_off.tolist(), [0, 2, 3])
                self.assertEqual(dataset.n, 2)
                self.assertEqual(torch.from_numpy(dataset.glob.copy()).sum().item(), 3)
                cached = D3.load(root)
                self.assertEqual(str(dataset.glob.filename), str(cached.glob.filename))


if __name__ == '__main__':
    unittest.main()

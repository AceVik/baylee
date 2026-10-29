"""Versioned disk records; only small indices/predictions are held in RAM."""
import json
from pathlib import Path
import numpy as np
from .source_identity import extraction_sources_digest
from trajectory_store4 import _native

VERSION = 2


def metadata(kind, **value):
    return json.loads(_native(['calibration-io'],json.dumps(dict(kind=kind,**value),allow_nan=False).encode()))


def record_dtype(width):
    return np.dtype([("features", "<f4", (8, 2 * width)), ("active", "?", (8,)),
                     ("allied", "?", (8,)), ("outcome", "<f4"), ("source_value", "<f4"),
                     ("trajectory", "<i8"), ("game", "<u8"), ("seat", "<u4"),
                     ("cursor", "<u8"), ("players", "<i4"), ("turn_norm", "<f4"),
                     ("commander", "?"), ("team", "?"), ("source", "<i4"),
                     ("decision", "?"), ("neural_decision", "?"), ("terminal", "?")])


def identity(source_hash, dataset, stride, board_chunk, device_type):
    return metadata('identity',source_hash=source_hash, dataset_fingerprint=dataset.fingerprint,
                extraction_sources_sha256=extraction_sources_digest(), device_type=device_type,
                structure_sha256=dataset.structure.digest,
                stride=stride,unroll=dataset.unroll_len, board_chunk=board_chunk)


def key_for(settings):
    return metadata('key',settings=settings)


class FeatureCache:
    def __init__(self, directory, width):
        self.path = Path(directory)
        admitted=metadata('admit',path=str(self.path),width=width)
        self.meta = admitted['meta']
        dtype = record_dtype(width)
        self.records = np.memmap(self.path / "records.bin", mode="r", dtype=dtype,
                                 shape=(self.meta["records"],))
        self.indices=np.asarray(admitted['indices'],dtype=np.int64)

    def tensor_batch(self, indices, device):
        import torch
        rows = self.records[indices]
        return tuple(torch.from_numpy(np.array(rows[name], copy=True)).to(device)
                     for name in ("features", "active", "allied", "outcome"))

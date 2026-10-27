"""Versioned disk records; only small indices/predictions are held in RAM."""
import hashlib
import json
from pathlib import Path
import numpy as np
from .source_identity import extraction_sources_digest, file_digest

VERSION = 2


def record_dtype(width):
    return np.dtype([("features", "<f4", (8, 2 * width)), ("active", "?", (8,)),
                     ("allied", "?", (8,)), ("outcome", "<f4"), ("source_value", "<f4"),
                     ("trajectory", "<i8"), ("game", "<u8"), ("seat", "<u4"),
                     ("cursor", "<u8"), ("players", "<i4"), ("turn_norm", "<f4"),
                     ("commander", "?"), ("team", "?"), ("source", "<i4"),
                     ("decision", "?"), ("neural_decision", "?"), ("terminal", "?")])


def identity(source_hash, dataset, stride, board_chunk, device_type):
    return dict(version=VERSION, source_sha256=source_hash, dataset_fingerprint=dataset.fingerprint,
                extraction_sources_sha256=extraction_sources_digest(), device_type=device_type,
                structure_sha256=dataset.structure.digest, sample_every=stride,
                unroll=dataset.unroll_len, board_chunk=board_chunk,
                thinking="recorded positive K on Net decisions; otherwise K1; passive K0",
                precision="float32", sampling="cursor stride plus first and terminal; no frame skipping")


def key_for(settings):
    return hashlib.sha256(json.dumps(settings, sort_keys=True).encode()).hexdigest()


class FeatureCache:
    def __init__(self, directory, width):
        self.path = Path(directory)
        self.meta = json.loads((self.path / "complete.json").read_text())
        dtype = record_dtype(width)
        if (self.path / "records.bin").stat().st_size != self.meta["records"] * dtype.itemsize:
            raise ValueError("Incomplete calibration feature cache")
        if self.meta.get("records_sha256") != file_digest(self.path / "records.bin"):
            raise ValueError("Calibration feature cache records digest differs")
        self.records = np.memmap(self.path / "records.bin", mode="r", dtype=dtype,
                                 shape=(self.meta["records"],))
        eligible = np.asarray(self.meta["eligible_trajectories"], dtype=np.int64)
        self.indices = np.flatnonzero(np.isin(self.records["trajectory"], eligible))
        if not len(self.indices):
            raise ValueError("No sampled learner trajectories in feature cache")

    def tensor_batch(self, indices, device):
        import torch
        rows = self.records[indices]
        return tuple(torch.from_numpy(np.array(rows[name], copy=True)).to(device)
                     for name in ("features", "active", "allied", "outcome"))

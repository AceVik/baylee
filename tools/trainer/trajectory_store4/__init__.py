"""PyTorch dataset adapter over caches admitted and packed by native Rust.

Only read-only mmap decoding stays beside the tensor loader. Validation,
fingerprints, shard parsing, packing and cache publication have no Python fallback.
"""
from __future__ import annotations
import json
import mmap
import os
from pathlib import Path
import shutil
import struct
import subprocess
import zlib
import numpy as np
import torch

INDEX = np.dtype([("game", "<u4"), ("seat", "<u4"), ("cursor", "<u8"),
                  ("offset", "<u8"), ("length", "<u4"), ("flags", "<u4")])
BST4_MAGIC = b"BST4"
BST4_HDR = struct.Struct("<4s10I2i9f12I")

def _executable():
    executable = shutil.which("trajectory_cache4")
    if executable is None:
        root = Path(__file__).resolve().parents[3]
        executable = next((str(root / "target" / profile / "trajectory_cache4")
                           for profile in ("release", "debug")
                           if (root / "target" / profile / "trajectory_cache4").is_file()), None)
    if executable is None:
        raise RuntimeError("Build native tensor-cache support: cargo build --release -p baylee-train-ops --bin trajectory_cache4")
    return executable

def _native(arguments, payload=None):
    executable = _executable()
    result = subprocess.run([executable, *map(str, arguments)], input=payload, capture_output=True)
    if result.returncode:
        raise ValueError(result.stderr.decode(errors="replace").strip())
    return result.stdout

def validate_bc_dataset(root, meta):
    """Delegate immutable companion admission before constructing Torch datasets."""
    _native(["validate", "--data-dir", root, "--metadata-stdin"], json.dumps(meta).encode())

def reject_shadow_policy_gradient(meta):
    """Delegate teacher-only rejection before a Torch policy-gradient update."""
    _native(["validate", "--policy-gradient"], json.dumps(meta).encode())

def pack_step_bst4(step, roots):
    """Native binary fixture bridge for Torch encoder equivalence tests."""
    return _native(["pack"], json.dumps({"step": step, "roots": roots}).encode())

class TrajectoryStore4(torch.utils.data.Dataset):
    """Read-only frame store for PyTorch DataLoader worker processes."""
    def __init__(self, data_dir: Path, split: str, shards: list[str]):
        super().__init__()
        command = ["build", "--data-dir", data_dir, "--split", split]
        for shard in shards:
            command += ["--shard", shard]
        self._cache_command = command
        receipt = json.loads(_native(command))
        self.fingerprint = receipt["fingerprint"]
        self.data_path = Path(receipt["data_path"])
        self.index_path = Path(receipt["index_path"])
        self.index = (np.memmap(self.index_path, mode="r", dtype=INDEX)
                      if receipt["records"] else np.empty(0, dtype=INDEX))
        self._pid = self._file = self._map = None

    def __len__(self):
        return len(self.index)

    def __getitem__(self, row):
        return self.read_raw(row)

    def read_raw(self, row):
        if self._pid != os.getpid():
            self.close()
            receipt = json.loads(_native(self._cache_command))
            if receipt['fingerprint'] != self.fingerprint:
                raise ValueError('Trajectory source changed before tensor mmap load')
            self.index = (np.memmap(self.index_path, mode='r', dtype=INDEX)
                          if receipt['records'] else np.empty(0, dtype=INDEX))
            self._file = self.data_path.open("rb")
            self._map = mmap.mmap(self._file.fileno(), 0, access=mmap.ACCESS_READ)
            self._pid = os.getpid()
        record = self.index[row]
        start, length = int(record["offset"]), int(record["length"])
        return zlib.decompress(self._map[start:start + length])

    def read(self, row):
        raw = self.read_raw(row)
        if raw[:4] != BST4_MAGIC:
            return json.loads(raw)
        h = BST4_HDR.unpack_from(raw)
        return {"game_id": h[2], "seat": h[3], "step_cursor": h[4], "turn": h[5],
                "is_decision": bool(h[6] & 1), "is_terminal": bool(h[6] & 2),
                "head": h[7], "pending_kind": h[8], "loss_weight": h[13], "outcome": h[15],
                "target": {"single_idx": h[12], "scalar_norm": h[14]},
                "aux": dict(zip(("turns_remaining_norm", "my_final_life_norm", "opp_min_final_life_norm", "next_life_delta_norm"), h[16:20])),
                "_bst4": raw}

    def __getstate__(self):
        return {**self.__dict__, "_pid": None, "_file": None, "_map": None}

    def close(self):
        if self._map is not None:
            self._map.close()
            self._file.close()
        self._pid = self._file = self._map = None

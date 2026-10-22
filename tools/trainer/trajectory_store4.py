"""Disk-backed compressed trajectory records; bounded RAM and atomic cache builds.

The cache is an optimization of immutable converter output, not another source
of training labels. Each index row points to one compressed JSON record. Only
the small index is resident; worker processes reopen the files after fork/spawn.
"""
from __future__ import annotations

import fcntl
import gzip
import hashlib
import json
import mmap
import os
from pathlib import Path
import struct
import zlib
import re
import shutil
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import orjson

INDEX = np.dtype([
    ("game", "<u4"), ("seat", "<u4"), ("cursor", "<u8"),
    ("offset", "<u8"), ("length", "<u4"), ("flags", "<u4"),
])
ROW = struct.Struct("<IIQQII")
STATE_FIELD = re.compile(rb',\s*"state"\s*:')


def record_header(line: bytes) -> dict:
    # serde emits the small index fields before the large state arrays. Parsing
    # that prefix avoids creating millions of unused Python floats at startup.
    # Other valid JSON key orders take the general path.
    match = STATE_FIELD.search(line)
    if match:
        try:
            header = orjson.loads(line[:match.start()] + b"}")
            if all(k in header for k in ("game_id", "seat", "step_cursor", "is_decision", "is_terminal")):
                return header
        except orjson.JSONDecodeError:
            pass
    return orjson.loads(line)


def build_part(path: Path, data_path: Path, index_path: Path) -> int:
    count = 0
    with data_path.open("wb") as data, index_path.open("wb") as index, gzip.open(path, "rb") as inp:
        for line in inp:
            if not line.strip():
                continue
            step = record_header(line)
            blob = zlib.compress(line, level=1)
            flags = int(step["is_decision"]) | (int(step.get("is_terminal", False)) << 1)
            index.write(ROW.pack(step["game_id"], step["seat"], step["step_cursor"],
                                 data.tell(), len(blob), flags))
            data.write(blob)
            count += 1
    return count


class TrajectoryStore4:
    def __init__(self, data_dir: Path, split: str, shards: list[str]):
        paths = [data_dir / split / name for name in shards]
        identity = hashlib.sha256((data_dir / "meta.json").read_bytes())
        for path in paths:
            st = path.stat()  # Missing shards are corruption, never silently skipped.
            identity.update(f"{path.name}:{st.st_size}:{st.st_mtime_ns}".encode())
        self.fingerprint = identity.hexdigest()
        cache = data_dir / ".trajectory-cache-v2"
        cache.mkdir(exist_ok=True)
        stem = f"{split}-{self.fingerprint[:24]}"
        self.data_path = cache / f"{stem}.bin"
        self.index_path = cache / f"{stem}.idx"
        done = cache / f"{stem}.json"
        with (cache / f"{stem}.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if not done.exists():
                self._build(paths, done)
        size = self.index_path.stat().st_size
        if size % INDEX.itemsize:
            raise ValueError(f"Corrupt trajectory index: {self.index_path}")
        self.index = (np.memmap(self.index_path, mode="r", dtype=INDEX)
                      if size else np.empty(0, dtype=INDEX))
        self._pid = None
        self._file = None
        self._map = None

    def _build(self, paths: list[Path], done: Path) -> None:
        data_tmp = self.data_path.with_suffix(".bin.partial")
        index_tmp = self.index_path.with_suffix(".idx.partial")
        count = 0
        parts = [(path, data_tmp.with_suffix(f".part{i}.bin"), index_tmp.with_suffix(f".part{i}.idx"))
                 for i, path in enumerate(paths)]
        with ThreadPoolExecutor(max_workers=min(4, max(1, len(parts)))) as pool:
            jobs = [pool.submit(build_part, *part) for part in parts]
            for job in jobs:
                count += job.result()
        with data_tmp.open("wb") as data, index_tmp.open("wb") as index:
            for _, part_data, part_index in parts:
                rows = np.fromfile(part_index, dtype=INDEX)
                rows["offset"] += data.tell()
                index.write(rows.tobytes())
                with part_data.open("rb") as src:
                    shutil.copyfileobj(src, data, length=1 << 20)
                part_data.unlink()
                part_index.unlink()
            data.flush()
            index.flush()
            os.fsync(data.fileno())
            os.fsync(index.fileno())
        os.replace(data_tmp, self.data_path)
        os.replace(index_tmp, self.index_path)
        marker = done.with_suffix(".json.partial")
        marker.write_text(json.dumps({"fingerprint": self.fingerprint, "records": count}))
        os.replace(marker, done)

    def read(self, row: int) -> dict:
        if self._pid != os.getpid():
            if self._map is not None:
                self._map.close()
                self._file.close()
            self._file = self.data_path.open("rb")
            self._map = mmap.mmap(self._file.fileno(), 0, access=mmap.ACCESS_READ)
            self._pid = os.getpid()
        rec = self.index[row]
        start = int(rec["offset"])
        return orjson.loads(zlib.decompress(self._map[start:start + int(rec["length"])]))

    def __getstate__(self):
        state = dict(self.__dict__)
        state.update(_pid=None, _file=None, _map=None)
        return state

    def close(self):
        if self._map is not None:
            self._map.close()
            self._file.close()
            self._map = self._file = self._pid = None

